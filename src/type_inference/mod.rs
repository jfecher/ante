use std::{
    borrow::Cow,
    cell::{Cell, OnceCell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    sync::Arc,
};

use inc_complete::DbGet;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use smallbitvec::SmallBitVec;

use crate::{
    diagnostics::Diagnostic,
    incremental::{
        self, DbHandle, ExportedDefinitions, ExportedTypes, GetItem, Resolve, TargetPointerSize, TypeCheck,
        TypeCheckSCC,
    },
    iterator_extensions::{map_btree, mapvec},
    lexer::token::{Integer, IntegerKind},
    name_resolution::{
        Origin, ResolutionResult,
        namespace::{CrateId, SourceFileId},
    },
    parser::{
        cst::{self, Name, ReferenceKind, TopLevelItem, TopLevelItemKind},
        desugar_context::DesugarContext,
        get_item::TraitDefinition,
        ids::{ExprId, NameId, PathId, PatternId, TopLevelId, TopLevelName},
    },
    shared_arc::Shared,
    type_inference::{
        errors::{Locateable, TypeErrorKind},
        fresh_expr::ExtendedTopLevelContext,
        generics::Generic,
        implicits::ImplicitsContext,
        places::{Mutability, PlaceMode},
        row::{RowKind, RowMode},
        types::{
            FunctionType, LocalKinds, Omissions, ParameterType, PrimitiveType, Type, TypeBindings, TypeVariableId,
        },
    },
};

mod affine;
mod borrows;
mod cst_traversal;
pub mod dependency_graph;
pub mod dictionary_coercions;
mod drops;
pub mod errors;
mod free_variables;
pub mod fresh_expr;
pub mod generics;
pub mod get_type;
mod implicits;
pub mod kinds;
mod moved_captures;
pub mod patterns;
mod places;
mod recursive_types;
pub(crate) mod row;
pub(crate) mod type_body;
mod type_definitions;
pub mod types;
mod value_impls;

pub use get_type::get_type_impl;
pub use type_body::TypeBody;

/// Actually type check a statement and its contents.
/// Unlike `get_type_impl`, this always type checks the expressions inside a statement
/// to ensure they type check correctly.
pub fn type_check_impl(context: &TypeCheckSCC, compiler: &DbHandle) -> Arc<TypeCheckSCCResult> {
    incremental::enter_query();
    let items = TypeChecker::item_contexts(&context.0, compiler);
    let mut checker = TypeChecker::new(&items, compiler);

    // We infer every item before borrow checking any of them since what an item drops, and how, can
    // depend on types that are only known once a later item in the SCC is inferred.
    let inferred = mapvec(context.0.iter(), |item_id| {
        incremental::println(format_args!("Type checking {item_id:?}"));
        checker.start_item(*item_id);
        checker.push_implicits_scope();
        let depth = checker.current_scope_depth();

        let item = &checker.item_contexts[item_id].0;
        match &item.kind {
            TopLevelItemKind::Definition(definition) => checker.check_definition(definition, true),
            TopLevelItemKind::TypeDefinition(type_definition) => checker.check_type_definition(type_definition),
            TopLevelItemKind::TraitDefinition(_) | TopLevelItemKind::EffectDefinition(_) => {
                unreachable!("Traits/effects should be desugared into types by this point")
            },
            TopLevelItemKind::TraitImpl(_) => {
                unreachable!("TraitImpls should be desugared into definitions by this point")
            },
            TopLevelItemKind::Comptime(comptime) => checker.check_comptime(comptime),
        };

        // Need to resolve any remaining implicits before the borrowing pass runs
        checker.pop_implicits_scope();
        (depth, checker.suspend_item())
    });

    checker.default_unknown_types(inferred.iter().map(|(_, (_, maps))| maps));
    let items = mapvec(context.0.iter().zip(inferred), |(item_id, (depth, suspended))| {
        checker.resume_item(*item_id, suspended);
        checker.resolve_value_impls();
        checker.check_impl_parameters_are_copy();
        let item = &checker.item_contexts[item_id].0;
        if let TopLevelItemKind::Definition(definition) = &item.kind {
            checker.check_borrows(definition, depth);
        }
        (*item_id, checker.finish_item())
    });

    // Drops in a later item of the SCC may add effects to an earlier item's rows
    checker.default_deferred_effects_to_pure();
    checker.default_once_variables();

    // Whether a dictionary coercion widens any effects may depend on later items in the SCC
    checker.resolve_dictionary_coercions();

    incremental::exit_query();
    Arc::new(checker.finish(items))
}

/// The result of [TypeChecker::once_coercion] coercing a function to a `once fn`
enum OnceCoercion {
    /// No coercion was needed. The function is already a `once fn` or isn't expected to be one
    None,
    /// The function was only given this type. Its captures are `Copy` so nothing needs to drop them
    Retyped(Type),
    /// The expression was wrapped in a new `once fn`, which still needs to be type checked
    Wrapped,
}

/// A `TypeCheckSCCResult` holds the `IndividualTypeCheckResult` of every item in
/// the SCC for a particular TopLevelId
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeCheckSCCResult {
    pub items: BTreeMap<TopLevelId, Shared<IndividualTypeCheckResult>>,
    pub bindings: Shared<TypeBindings>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndividualTypeCheckResult {
    pub maps: TypeMaps,

    /// The type checker may create additional expressions, patterns, etc.,
    /// which it places in this context. This is a full replacement for the
    /// [TopLevelContext] output from the parser. Continuing to use the old
    /// [TopLevelContext] will work for most expressions but lead to panics
    /// when newly created items from the type checking pass are used.
    pub context: ExtendedTopLevelContext,

    /// One or more names may be externally visible outside this top-level item.
    /// Each of these names will be generalized and placed in this map.
    /// Ex: in `foo = (bar = 1; bar + 2)` only `foo: I32` will be generalized,
    /// but in `a, b = 1, 2`, both `a` and `b` will be.
    /// Ex2: in `type Foo = | A | B`, `A` and `B` will both be generalized, and
    /// there is no need to generalize `Foo` itself.
    pub generalized: FxHashMap<NameId, Type>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TypeMaps {
    pub name_types: FxHashMap<NameId, Type>,
    pub path_types: FxHashMap<PathId, Type>,
    pub expr_types: FxHashMap<ExprId, Type>,
    pub pattern_types: FxHashMap<PatternId, Type>,
}

/// The TypeChecker is responsible for checking for type errors inside of an
/// inference group. An inference group is a set of top-level items which form
/// an SCC in the type inference dependency graph. Usually each group is only
/// a single item but larger groups are possible for mutually recursive definitions
/// without type signatures.
///
/// The TypeChecker is the main context object for the type inference incremental computation.
/// Its outputs are:
/// - A type for all [NameId], [PathId], and [ExprId] objects (possibly an error type)
/// - Errors or warnings accumulated to the compiler's [Diagnostic] list
/// - A new resolved [Origin] for each [Origin::TypeResolution] outputted from the name resolution pass
/// - New expressions & paths resulting from the compilation of match expressions into decision trees
struct TypeChecker<'local, 'inner> {
    compiler: &'local DbHandle<'inner>,
    name_types: FxHashMap<NameId, Type>,
    path_types: FxHashMap<PathId, Type>,
    pattern_types: FxHashMap<PatternId, Type>,
    expr_types: FxHashMap<ExprId, Type>,

    bindings: TypeBindings,

    /// Type inference is the first pass where type variables are introduced.
    /// This field starts from 0 to give each a unique ID within the current inference group.
    next_type_variable_id: Cell<u32>,

    /// Contains the ItemContext for each item in the TypeChecker's type check group.
    /// Most often, this is just a single item. In the case of mutually recursive type
    /// inference however, it will include every item in the recursive SCC to infer.
    item_contexts: &'local ItemContexts,

    /// The type checker may output new expression, path, or name IDs so we
    /// extend each [TopLevelContext] with these new ids.
    id_contexts: FxHashMap<TopLevelId, ExtendedTopLevelContext>,

    /// The current top-level item being type checked. This is empty upon initialization, but
    /// while type checking, this should always be non-empty.
    current_item: Option<TopLevelId>,

    /// The return type of the current function. Used to type check `return` statements.
    function_return_type: Option<Type>,

    /// True if the innermost lambda being checked is the condition or body of a loop. A `return`
    /// here performs an effect to leave the loop, see [Self::perform_early_return].
    in_loop_part: bool,

    /// The effect row of the function whose body is currently being checked.
    current_effect_row: Type,

    /// Effects of the current function body left to its boundary, see [Self::split_deferred_effects]
    deferred_effects: Vec<Type>,

    /// Types of each top-level item in the current SCC being worked on
    item_types: Rc<FxHashMap<TopLevelName, Type>>,

    /// For a name whose signature is fully explicit, its frozen `Forall`-wrapped type from
    /// `try_get_generalized_type` - using the exact same generic ids as `item_types`' entry for
    /// that name, since both come from one `open_inferred_generics` call. `generalize_all` uses
    /// this directly instead of re-deriving a type from the (possibly body-narrowed) `item_types`
    /// entry, so nothing body-checking does can change what's exported for this name.
    declared_types: FxHashMap<TopLevelName, Type>,

    /// The outer Vec represents each scope (roughly each block of code),
    /// while the inner Vec is the implicits context for that scope. This contains
    implicits: Vec<ImplicitsContext>,

    /// The current lexical nesting depth. Used to track the scope anonymous places
    /// are valid in.
    scope_depth: u32,

    /// State local to the item currently being checked
    item: ItemState,

    /// If the item currently being checked is an impl of a trait, the impl and the trait
    current_impl: Option<(&'local cst::ImplInfo, TraitDefinition)>,

    /// Each dictionary coerced to a trait type which may have wider effect args, by item.
    /// These are decided once every item in the SCC is checked.
    pending_dictionary_coercions: dictionary_coercions::PendingDictionaryCoercions,

    /// Cached type for the Prelude's `String` struct, lazily resolved on first use.
    string_type: Option<Type>,

    /// Cached origins of stdlib definitions, by module and name. See [Self::std_definition_origin].
    std_origins: FxHashMap<(SourceFileId, &'static str), Origin>,

    /// Cached TopLevelName for the Prelude's `Copy` trait and its number of generics, lazily
    /// resolved on first use.
    copy_trait: Option<(TopLevelName, usize)>,

    /// The Prelude's `Drop` trait and its number of generics, lazily resolved on first use.
    drop_trait: Option<(TopLevelName, usize)>,

    /// The Prelude's pair type, resolved the first time a closure captures more than one value
    pair_type: OnceCell<TopLevelName>,

    /// Open effect rows. These are only defaulted to pure once every item in the SCC is borrow
    /// checked since a drop may still add effects to them.
    deferred_effect_defaults: Vec<TypeVariableId>,

    /// Each loop condition and body has its effect row left open so that it can include the effects
    /// of its drops. These are the row variables used for that. See [Self::discharge_effects].
    loop_row_ends: FxHashSet<TypeVariableId>,

    /// Environment variables for the body and each branch of every `handle`. These are only decided
    /// once the `handle` is borrow checked.
    handle_environments: FxHashSet<TypeVariableId>,

    /// Variables for each function type's [FunctionType::once]. Any the borrow checker leaves
    /// unbound are functions that may be called any number of times.
    once_variables: Vec<TypeVariableId>,

    /// False while the borrow walk type checks the drops it inserts. These are never copied or
    /// dropped themselves so we don't need to search for their impls. See [value_impls].
    requests_value_impls: bool,

    /// The kind of each type variable that isn't a plain type. A variable gets its kind either when
    /// it is created (see [Self::next_variable]) or when the type it was converted with enters the
    /// checker (see [Self::record_kinds]).
    variable_kinds: RefCell<FxHashMap<TypeVariableId, kinds::Kind>>,

    /// Cache of whether the first parameter of each trait searched for is a type. See
    /// [Self::subject_is_rigid_type]
    type_subject_traits: FxHashMap<TopLevelName, bool>,

    /// Maps each open effect row that reached the boundary of a function with a closed row to that
    /// closed row. Anything the open row is later bound to must fit within the closed row, which we
    /// check when binding it. See [Self::check_effect_bound].
    effect_bounds: FxHashMap<TypeVariableId, Type>,

    /// The Prelude's `Mutate` effect
    mutate_name: TopLevelName,

    /// The Prelude's `Extract` trait
    extract_name: TopLevelName,

    /// The Prelude's `Insert` trait
    insert_name: TopLevelName,

    /// True while inferring the left-hand side of an assignment
    inferring_assignment_lhs: bool,

    /// Type variables created for polymorphic integer literals. Entries are never removed.
    /// Used during unification to restrict these variables to integer types so mismatches error
    /// at the unification site instead of a confusing error when defaulting to I32 later.
    integer_literal_vars: FxHashSet<TypeVariableId>,

    /// Same as `integer_literal_vars` but for polymorphic float literals.
    float_literal_vars: FxHashSet<TypeVariableId>,

    /// We need to remember any type variables that the auto-ref coercion applies to to prevent
    /// them from later unifying with references, otherwise it could bind to `ref (ref t)` internally
    /// which can lead to soundness errors causing segfaults at runtime.
    value_type_vars: FxHashSet<TypeVariableId>,
}

/// The parts of the [TypeChecker]'s state which are local to the item being checked. Since ids such
/// as [ExprId] and [NameId] are local to each item, this is replaced as a whole by
/// [TypeChecker::start_item]. State which must outlive an item, such as anything decided only
/// once each item in the SCC is checked, is kept in the [TypeChecker] keyed by item instead.
#[derive(Default)]
struct ItemState {
    /// Tracks ExprIds for which `check_lambda` was called due to an implicit parameter coercion
    /// wrapper. For these, `check_for_closure` is deferred until after `pop_implicits_scope` of
    /// the enclosing lambda resolves the delayed implicits that fill in the wrapper's free vars.
    coercion_wrapper_exprs: FxHashSet<ExprId>,

    /// Keep track of which variable pattern aliases alias to catch double or partial
    /// moves when both an alias and the original name are moved.
    binding_places: FxHashMap<NameId, places::Place>,

    /// For each value the borrow walk may copy or drop, the implicits we search with and the impl
    /// we found. See [value_impls].
    value_impls: value_impls::ValueImplTable,

    /// Failed implicit searches by destination
    failed_implicit_searches: FxHashMap<ExprId, implicits::ImplicitError>,

    /// Cache of whether each fully known type is `Copy` with the local implicits in scope. This is
    /// only used when the types of those implicits are fully known as well. See
    /// [TypeChecker::type_is_copy_in].
    copy_memo: FxHashMap<(Vec<NameId>, Type), bool>,

    /// How many times an implicit search tried a candidate from the current SCC. Since the types of
    /// these candidates may still be refined, a search that tried one can't have its result reused.
    scc_candidates_tried: usize,

    /// Delayed implicits resolved early, mapped by the destination expr.
    /// These stay queued in their scope so that the position of other queued implicits is unchanged,
    /// mapping to their target type from before they were resolved.
    resolved_early_implicits: FxHashMap<ExprId, Type>,

    /// Set while checking the constructor of an impl of a trait. See [ImplState].
    impl_: Option<ImplState>,

    /// The body and each branch of every `handle` in the item
    handle_lambdas: FxHashSet<ExprId>,
}

/// An impl of a trait whose constructor is being checked. Each of the impl's methods takes a
/// reference to the impl's data, which holds the impl's parameters, so a method reads them from
/// there rather than capturing them.
struct ImplState {
    /// The lambda of each of the impl's methods, see [TypeChecker::check_impl_method]
    methods: FxHashSet<ExprId>,

    /// The `Copy` impl requested for each of the impl's parameters which aren't trivially `Copy`.
    /// See [TypeChecker::check_impl_parameters_are_copy].
    copy_parameters: Vec<PatternId>,
}

impl TraitDefinition {
    /// Whether each member, in declaration order, is a method rather than a constant. A member is a
    /// method if its type is a function type once any type aliases are expanded. Each method takes a
    /// reference to the data of its impl, see [crate::mir::builder] for how dictionaries are lowered.
    pub fn method_flags(&self, db: &impl DbGet<TypeCheck>) -> SmallBitVec {
        let result = TypeCheck(self.id()).get(db);
        self.members().iter().map(|(name, _)| Self::member_is_method(&result, *name)).collect()
    }

    /// Whether the member at `index` is a method rather than a constant, see [Self::method_flags]
    pub fn is_method(&self, index: usize, db: &impl DbGet<TypeCheck>) -> bool {
        let Some((name, _)) = self.members().get(index) else { return false };
        Self::member_is_method(&TypeCheck(self.id()).get(db), *name)
    }

    /// Whether the member `name` of the trait type checked as `result` is a method
    fn member_is_method(result: &dependency_graph::TypeCheckResult, name: NameId) -> bool {
        let bindings = &result.bindings;
        let typ = result.result.generalized.get(&name);
        typ.is_some_and(|typ| matches!(typ.follow(bindings).ignore_forall().follow(bindings), Type::Function(_)))
    }
}

/// Map from each TopLevelId to a tuple of (the item, parse context, resolution context)
type ItemContexts = FxHashMap<TopLevelId, (Shared<TopLevelItem>, Shared<DesugarContext>, Arc<ResolutionResult>)>;

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    fn new(item_contexts: &'local ItemContexts, compiler: &'local DbHandle<'inner>) -> Self {
        let id_contexts = item_contexts
            .iter()
            .map(|(id, (_, context, _))| (*id, ExtendedTopLevelContext::new(context.clone())))
            .collect();

        let mut this = Self {
            compiler,
            bindings: Default::default(),
            next_type_variable_id: Cell::new(0),
            name_types: Default::default(),
            path_types: Default::default(),
            expr_types: Default::default(),
            pattern_types: Default::default(),
            item_types: Default::default(),
            declared_types: Default::default(),
            current_item: None,
            function_return_type: None,
            in_loop_part: false,
            current_effect_row: Type::pure(),
            deferred_effects: Vec::new(),
            item_contexts,
            id_contexts,
            implicits: Vec::new(),
            scope_depth: 0,
            item: ItemState::default(),
            current_impl: None,
            pending_dictionary_coercions: Default::default(),
            string_type: None,
            std_origins: Default::default(),
            copy_trait: None,
            drop_trait: None,
            pair_type: Default::default(),
            deferred_effect_defaults: Vec::new(),
            once_variables: Vec::new(),
            handle_environments: Default::default(),
            loop_row_ends: Default::default(),
            effect_bounds: Default::default(),
            variable_kinds: Default::default(),
            type_subject_traits: Default::default(),
            requests_value_impls: true,
            mutate_name: prelude_type_name(compiler, "Mutate"),
            extract_name: prelude_type_name(compiler, "Extract"),
            insert_name: prelude_type_name(compiler, "Insert"),
            inferring_assignment_lhs: false,
            integer_literal_vars: Default::default(),
            float_literal_vars: Default::default(),
            value_type_vars: Default::default(),
        };

        let mut item_types = FxHashMap::default();
        for (item_id, (item, context, resolution)) in item_contexts.iter() {
            for name in resolution.top_level_names.iter() {
                let name = TopLevelName::new(*item_id, *name);
                let typ = if let TopLevelItemKind::Definition(definition) = &item.kind {
                    let next_id = &mut this.next_type_variable_id.get();

                    // TODO: This prevents a MIR panic in examples/codegen/sync/mutex_counter.an
                    // from a subtype'd function having a different expected type at its definition
                    // vs call site. This should ideally be fixable with only `get_partial_type`.
                    let typ = match get_type::try_get_seeded_and_declared_type(
                        definition,
                        context.as_ref(),
                        resolution,
                        compiler,
                        next_id,
                    ) {
                        Some((seeded, declared)) => {
                            this.declared_types.insert(name, declared);
                            seeded
                        },
                        None => get_type::get_partial_type(
                            definition,
                            context.as_ref(),
                            resolution,
                            compiler,
                            next_id,
                            true,
                        ),
                    };

                    this.next_type_variable_id.set(*next_id);
                    this.record_kinds(&typ, &kinds::Kind::Type);
                    typ
                } else {
                    this.next_type_variable()
                };
                item_types.insert(name, typ);
            }
        }
        // We have to go through this extra step since `generalize_all` needs an Rc
        // to clone this field cheaply since `generalize` requires a mutable `self`.
        let this_item_types = Rc::get_mut(&mut this.item_types).expect("No clones should be possible here");
        *this_item_types = item_types;

        this
    }

    fn item_contexts(items: &[TopLevelId], compiler: &DbHandle) -> ItemContexts {
        items
            .iter()
            .map(|item_id| {
                let (item, item_context) = GetItem(*item_id).get(compiler);
                let resolve = Resolve(*item_id).get(compiler);
                (*item_id, (item, item_context, resolve))
            })
            .collect()
    }

    /// Returns the context of the current item, containing mappings for IDs set during parsing.
    /// This will not contain any new IDs added by this type checking pass - for that use
    /// [Self::current_extended_context_mut]. This method is still useful since the returned
    /// context refers to a separate lifetime, so self may still be used mutably.
    fn current_context(&self) -> &'local DesugarContext {
        let item = self.current_item.expect("TypeChecker: Expected current_item to be set");
        self.item_contexts[&item].1.as_ref()
    }

    fn current_resolve(&self) -> &'local ResolutionResult {
        let item = self.current_item.expect("TypeChecker: Expected current_item to be set");
        self.item_contexts[&item].2.as_ref()
    }

    /// The parameters of the current item if it is an `impl`, see [cst::Definition::impl_parameters]
    fn current_impl_parameters(&self) -> &'local [cst::Parameter] {
        let item = self.current_item.expect("TypeChecker: Expected current_item to be set");
        let (item, context, _) = &self.item_contexts[&item];
        match &item.kind {
            TopLevelItemKind::Definition(definition) => definition.impl_parameters(context.as_ref()),
            _ => &[],
        }
    }

    /// Return the current extended context.
    /// Note that this context only includes new items added by this type checker, it does
    /// not contain any existing items from the resolver until the type checker finishes
    /// and inserts the pre-existing items.
    fn current_extended_context(&self) -> &ExtendedTopLevelContext {
        let item = self.current_item.expect("TypeChecker: Expected current_item to be set");
        self.id_contexts.get(&item).expect("Expected TopLevelId to be in id_contexts")
    }

    /// Resolve `id` to its rewritten expr if this pass has replaced it in the extended
    /// context, falling back to the original parse-time expr otherwise.
    fn resolved_expr(&self, id: ExprId) -> Cow<'local, cst::Expr> {
        match self.current_extended_context().extended_expr(id) {
            Some(expr) => Cow::Owned(expr.clone()),
            None => Cow::Borrowed(&self.current_context()[id]),
        }
    }

    /// Returns the local variable `id` refers to, if it is just a variable
    fn local_variable(&self, id: ExprId) -> Option<NameId> {
        self.classify_place(id).filter(|place| place.fields.is_empty())?.local()
    }

    /// The name of the member a member access accesses
    fn member_name(&self, access: &cst::MemberAccess) -> &cst::Name {
        &self.current_extended_context()[access.member]
    }

    /// Return the current extended context.
    /// Note that this context only includes new items added by this type checker, it does
    /// not contain any existing items from the resolver until the type checker finishes
    /// and inserts the pre-existing items.
    fn current_extended_context_mut(&mut self) -> &mut ExtendedTopLevelContext {
        let item = self.current_item.expect("TypeChecker: Expected current_item to be set");
        self.id_contexts.get_mut(&item).expect("Expected TopLevelId to be in id_contexts")
    }

    /// Returns the [Origin] of the given [PathId]. May return [None] if there
    /// was an error during name resolution.
    fn path_origin(&self, path: PathId) -> Option<Origin> {
        let origin = self.current_resolve().path_origins.get(&path).copied();
        origin.or_else(|| self.current_extended_context().path_origin(path))
    }

    /// Returns the `String` type defined in the Prelude, caching it for subsequent calls.
    fn get_string_type(&mut self) -> Type {
        if let Some(typ) = &self.string_type {
            return typ.clone();
        }
        let typ = Type::UserDefined(Origin::TopLevelDefinition(prelude_type_name(self.compiler, "String")));
        self.string_type = Some(typ.clone());
        typ
    }

    /// Returns the origin of the Prelude definition `name`, caching it.
    fn prelude_definition_origin(&mut self, name: &'static str) -> Origin {
        self.std_definition_origin(SourceFileId::prelude(), name)
    }

    /// Returns the origin of the `Std.EarlyReturn.early_return` operation
    fn early_return_origin(&mut self) -> Origin {
        let module = SourceFileId::new(CrateId::STDLIB, std::path::Path::new("EarlyReturn.an"));
        self.std_definition_origin(module, "early_return")
    }

    /// Returns the origin of the definition `name` exported by the stdlib `module`, caching it.
    fn std_definition_origin(&mut self, module: SourceFileId, name: &'static str) -> Origin {
        if let Some(origin) = self.std_origins.get(&(module, name)) {
            return *origin;
        }
        let exported = ExportedDefinitions(module).get(self.compiler);
        let top_level_name = *exported
            .definitions
            .get(&Arc::new(name.to_string()))
            .unwrap_or_else(|| panic!("`{name}` not found in its stdlib module"));
        let (item, _) = GetItem(top_level_name.top_level_item).get(self.compiler);
        let origin = crate::name_resolution::origin_of_top_level_definition(top_level_name, &item.kind);
        self.std_origins.insert((module, name), origin);
        origin
    }

    /// If `callee` resolved to an operation which exits a loop, return which kind. We check what
    /// `callee` resolved to rather than its syntax since these operations can also be called by
    /// name or through an alias.
    fn loop_exit(&mut self, callee: ExprId) -> Option<LoopExit> {
        let path = match self.resolved_expr(callee).as_ref() {
            cst::Expr::Variable(path) => *path,
            _ => return None,
        };
        let origin @ Origin::EffectOperation { .. } = self.path_origin(path)? else { return None };
        if origin == self.prelude_definition_origin("break") || origin == self.prelude_definition_origin("continue") {
            Some(LoopExit::Jump)
        } else if origin == self.early_return_origin() {
            Some(LoopExit::Return)
        } else {
            None
        }
    }

    /// Returns the `IO` effect alias defined in the Prelude
    fn get_io_effects(&mut self) -> Type {
        let io_name = prelude_type_name(self.compiler, "IO");
        self.with_next_id(|next_id| crate::type_inference::types::expand_effect_alias(self.compiler, io_name, next_id))
            .expect("ICE: IO effect not found in Prelude")
    }

    fn finish(mut self, items: Vec<(TopLevelId, TypeMaps)>) -> TypeCheckSCCResult {
        let mut generalized = self.generalize_all();
        let items = items
            .into_iter()
            .map(|(id, maps)| {
                let generalized = generalized.remove(&id).unwrap_or_default();
                let mut context = self.id_contexts.remove(&id).unwrap();
                let item_context = self.item_contexts.get(&id).unwrap();
                context.extend_from_resolution_result(item_context.2.as_ref());
                (id, Shared::new(IndividualTypeCheckResult { maps, generalized, context }))
            })
            .collect();

        TypeCheckSCCResult { items, bindings: Shared::new(self.bindings) }
    }

    /// Check if the integer fits in the given kind, error if not
    fn check_int_fits(&self, value: Integer, kind: IntegerKind, locator: impl Locateable) {
        let ptr_size = TargetPointerSize.get(self.compiler);
        let fits = match kind.max_magnitude(value.negative, ptr_size) {
            Some(max) => value.magnitude <= max,
            None => false,
        };
        if !fits {
            let location = locator.locate(self);
            self.compiler.accumulate(Diagnostic::IntegerTooLarge { value, kind, location });
        }
    }

    /// Prepare the TypeChecker to type check another item.
    fn start_item(&mut self, item_id: TopLevelId) {
        self.enter_item(item_id);
        self.item = ItemState::default();

        // Nearly every id in the item gets a type
        let context = &self.item_contexts[&item_id].1;
        self.expr_types.reserve(context.exprs_len());
        self.name_types.reserve(context.names_len());
        self.path_types.reserve(context.paths_len());
        self.pattern_types.reserve(context.patterns_len());

        for (name, typ) in self.item_types.iter() {
            if name.top_level_item == item_id {
                self.name_types.insert(name.local_name_id, typ.clone());
            }
        }
    }

    /// Once every item in the SCC is inferred, a type variable that is still unbound and isn't part
    /// of an item's generalized type can never have a value. Ex: the error type of a `Result` that
    /// is only ever `Ok`. We default each of these that a value may hold to `Unit`. This happens
    /// before borrow checking so that values holding one don't need a `Drop` impl for it.
    ///
    /// Only variables of kind type are defaulted since an unbound effect, place, or type
    /// constructor is not the type of any value.
    fn default_unknown_types<'a>(&mut self, items: impl Iterator<Item = &'a TypeMaps>) {
        let types = items.flat_map(|maps| maps.name_types.values().chain(maps.expr_types.values()));

        let mut atoms = Vec::new();
        let mut held = FxHashSet::default();
        let mut environments = FxHashSet::default();
        for typ in types {
            atoms.clear();
            typ.owned_type_atoms(&self.bindings, self.compiler, &mut atoms);
            for atom in &atoms {
                if let Type::Variable(id) = atom
                    && !self.is_unresolved_literal(atom)
                {
                    held.insert(*id);
                }
            }
            typ.for_each_subterm(&self.bindings, &mut |typ, bindings| {
                if let Type::Function(function) = typ
                    && let Type::Variable(id) = function.environment.follow(bindings)
                {
                    environments.insert(*id);
                }
            });
        }
        let mut generalized = FxHashSet::default();
        for typ in self.item_types.values() {
            generalized.extend(typ.free_unification_vars(&self.bindings));
        }

        // Similarly, if a function type's environment is still unknown then no closure of that type
        // was ever made, so any value of it is a plain function
        for id in environments {
            if !generalized.contains(&id) && !self.handle_environments.contains(&id) {
                self.bindings.insert(id, Type::NO_CLOSURE_ENV);
            }
        }

        for id in held {
            if !generalized.contains(&id) && self.has_kind(id, &kinds::Kind::Type) {
                self.bindings.insert(id, Type::UNIT);
            }
        }
    }

    /// Make `item_id` the current item
    fn enter_item(&mut self, item_id: TopLevelId) {
        self.current_item = Some(item_id);
        let (item, context, resolve) = &self.item_contexts[&item_id];
        self.current_impl = match &item.kind {
            TopLevelItemKind::Definition(definition) => {
                get_type::trait_impl_of(definition, context, resolve, self.compiler)
            },
            _ => None,
        };
    }

    /// Pause the current item so it can be continued later with [Self::resume_item]
    fn suspend_item(&mut self) -> (ItemState, TypeMaps) {
        self.current_item = None;
        (std::mem::take(&mut self.item), self.take_type_maps())
    }

    /// Continue an item paused by [Self::suspend_item]
    fn resume_item(&mut self, item_id: TopLevelId, (state, maps): (ItemState, TypeMaps)) {
        self.enter_item(item_id);
        self.item = state;
        self.name_types = maps.name_types;
        self.path_types = maps.path_types;
        self.expr_types = maps.expr_types;
        self.pattern_types = maps.pattern_types;
    }

    /// Finishes the current item, adding all bindings to the relevant entry in
    /// `self.finished_items`, clearing them out in preparation for resolving the next item.
    fn finish_item(&mut self) -> TypeMaps {
        self.current_item = None;
        self.take_type_maps()
    }

    /// Take the types of everything in the current item, leaving its maps empty
    fn take_type_maps(&mut self) -> TypeMaps {
        TypeMaps {
            name_types: std::mem::take(&mut self.name_types),
            path_types: std::mem::take(&mut self.path_types),
            expr_types: std::mem::take(&mut self.expr_types),
            pattern_types: std::mem::take(&mut self.pattern_types),
        }
    }

    fn next_type_variable_id(&self) -> TypeVariableId {
        let id = TypeVariableId(self.next_type_variable_id.get());
        self.next_type_variable_id.update(|id| id + 1);
        id
    }

    fn next_type_variable(&self) -> Type {
        Type::Variable(self.next_type_variable_id())
    }

    /// Create a fresh environment variable for the body or a branch of a `handle`
    fn next_handle_environment(&mut self) -> Type {
        let id = self.next_type_variable_id();
        self.handle_environments.insert(id);
        Type::Variable(id)
    }

    /// Create a fresh [FunctionType::once] variable for the borrow checker to decide later
    fn next_once_variable(&mut self) -> Type {
        let id = self.next_type_variable_id();
        self.once_variables.push(id);
        Type::Variable(id)
    }

    /// Any [FunctionType::once] left undecided defaults to a function that may be called any number
    /// of times
    fn default_once_variables(&mut self) {
        for id in std::mem::take(&mut self.once_variables) {
            self.bind_if_unbound(&Type::Variable(id), Type::MANY);
        }
    }

    /// If `typ` follows to an unbound variable, bind it to `value`
    pub(super) fn bind_if_unbound(&mut self, typ: &Type, value: Type) {
        if let Type::Variable(id) = typ.follow(&self.bindings) {
            let id = *id;
            self.bindings.insert(id, value);
        }
    }

    /// Create a fresh type variable of the given kind
    fn next_variable(&self, kind: kinds::Kind) -> Type {
        let id = self.next_type_variable_id();
        self.set_kind(id, kind);
        Type::Variable(id)
    }

    /// Create a fresh variable for each argument of `constructor` at `indices`, using the kind the
    /// constructor declares for that argument
    fn fresh_arguments(&self, constructor: &Type, indices: std::ops::Range<usize>) -> Vec<Type> {
        let kinds = match constructor.constructor_kind(self.compiler) {
            Some(kinds::Kind::TypeConstructorComplex { params, .. }) => params,
            _ => Vec::new(),
        };
        mapvec(indices, |index| self.next_variable(kinds.get(index).cloned().unwrap_or(kinds::Kind::Type)))
    }

    /// Record that the variable `id` has the given kind
    fn set_kind(&self, id: TypeVariableId, kind: kinds::Kind) {
        if kind != kinds::Kind::Type {
            self.variable_kinds.borrow_mut().insert(id, kind);
        }
    }

    /// True if the variable `id` has the given kind
    fn has_kind(&self, id: TypeVariableId, kind: &kinds::Kind) -> bool {
        self.variable_kinds.borrow().get(&id).unwrap_or(&kinds::Kind::Type) == kind
    }

    /// Returns each unbound effect or effect row variable within `typ`
    fn unbound_effect_variables(&self, typ: &Type) -> Vec<TypeVariableId> {
        let mut effects = Vec::new();
        typ.for_each_kinded_variable(&kinds::Kind::Type, &self.bindings, self.compiler, &mut |variable, kind| {
            if let Type::Variable(id) = variable
                && *kind == kinds::Kind::Effect
            {
                effects.push(*id);
            }
        });
        effects
    }

    /// Record the kind of each unbound variable within `typ` based on its position, where `typ` is
    /// a type of the given kind converted from the source. Type conversion runs outside of any
    /// [TypeChecker] so this is the first chance we have to give its variables a kind.
    fn record_kinds(&self, typ: &Type, expected: &kinds::Kind) {
        typ.for_each_kinded_variable(expected, &self.bindings, self.compiler, &mut |variable, kind| {
            if let Type::Variable(id) = variable {
                self.set_kind(*id, kind.clone());
            }
        });
    }

    /// Run `f` with a reference to `self.next_type_variable_id`
    fn with_next_id<T>(&self, f: impl FnOnce(&mut u32) -> T) -> T {
        let mut next_id = self.next_type_variable_id.get();
        let result = f(&mut next_id);
        self.next_type_variable_id.set(next_id);
        result
    }

    /// A fresh, open effect row.
    fn fresh_effect_row(&self) -> Type {
        Type::Effects(Some(Shared::new(vec![self.next_variable(kinds::Kind::Effect)])))
    }

    /// Generalize all types in the current SCC.
    /// The returned Vec is in the same order as the SCC.
    ///
    /// Note that NameIds and PatternIds locally within each function will still refer to the
    /// non-generalized version of their types. If you want to retrieve the generalized type of an
    /// item from this SCC, you'll need to go through the generalized results specifically.
    fn generalize_all(&mut self) -> FxHashMap<TopLevelId, FxHashMap<NameId, Type>> {
        let mut items: FxHashMap<_, FxHashMap<_, _>> = FxHashMap::default();

        for (name, typ) in self.item_types.clone().iter() {
            self.current_item = Some(name.top_level_item);
            self.default_unshared_effects_to_pure(typ, typ);
            let typ =
                self.declared_types.get(name).cloned().unwrap_or_else(|| typ.generalize(&self.bindings, self.compiler));
            items.entry(name.top_level_item).or_default().insert(name.local_name_id, typ);
        }

        items
    }

    /// Defaults a `can e` to `pure` when `e` isn't referenced elsewhere in `root`, recursing into nested function types.
    fn default_unshared_effects_to_pure(&mut self, root: &Type, typ: &Type) {
        let Type::Function(function_type) = typ.follow(&self.bindings) else { return };
        let function_type = function_type.clone();

        let effects = self.collect_effects(&function_type.effects, &Default::default());
        for effect in &effects {
            let Type::Variable(id) = effect.follow(&self.bindings) else { continue };
            if root.count_unification_var_occurrences(*id, &self.bindings) <= 1 {
                let location = self.current_context().location().clone();
                self.unify(effect, &Type::pure(), TypeErrorKind::Effects, location);
            }
        }

        for parameter in &function_type.parameters {
            self.default_unshared_effects_to_pure(root, &parameter.typ);
        }
        self.default_unshared_effects_to_pure(root, &function_type.return_type);
    }

    /// Unifies the two types. Returns false on failure
    ///
    /// TODO: Rename. This is actually a subtyping relation of `actual <: expected`
    fn unify(&mut self, actual: &Type, expected: &Type, kind: TypeErrorKind, locator: impl Locateable) -> bool {
        if let Ok(new_bindings) = self.try_unify(actual, expected) {
            self.bindings.extend(new_bindings);
            true
        } else {
            self.report_type_error(actual, expected, kind, locator);
            false
        }
    }

    /// Report that `actual` was found where `expected` was expected
    fn report_type_error(&self, actual: &Type, expected: &Type, kind: TypeErrorKind, locator: impl Locateable) {
        let function_environments_differ = self.only_function_environments_differ(actual, expected);
        let (actual, expected) = self.type_pair_to_error_strings(actual, expected);
        let location = locator.locate(self);
        self.compiler.accumulate(Diagnostic::TypeError {
            actual,
            expected,
            kind,
            function_environments_differ,
            location,
        });
    }

    /// Unifies `effects_var` with the ambient effect row, then re-canonicalizes it as the new ambient row.
    ///
    /// `explicit_implicits` are the `(parameter type, argument type)` of each implicit argument given
    /// explicitly to the call.
    fn propagate_call_effects(
        &mut self, effects_var: &Type, implicits_before_call: implicits::ImplicitsMark,
        explicit_implicits: &[(Type, Type)], locator: impl Locateable + Copy,
    ) {
        // Row subtyping lets an impl like `Eq I32 pure` match `Eq I32 e` without binding `e`,
        // but the impl decides that effect. Close whatever it left open before the call's row is
        // linked to the ambient row below, where an open `e` would absorb the whole row instead.
        // TODO: Remove. This is a workaround for the lack of a constraint-based system.
        let location = locator.locate(self);
        for target in self.resolve_new_delayed_implicits(implicits_before_call) {
            // Copying or dropping a function doesn't depend on which effects calling it performs
            let function = self.copied_or_dropped_function(&target);
            let kept = function.map_or(Vec::new(), |function| self.unbound_effect_variables(&function));
            for id in self.unbound_effect_variables(&target) {
                if !kept.contains(&id) && !self.loop_row_ends.contains(&id) {
                    self.try_default_effect_to_pure(id, location.clone());
                }
            }
        }

        // The same goes for implicits given explicitly, although only the rows left open by matching
        // the argument against the parameter are closed. The argument's own open rows are left alone.
        for (parameter, argument) in explicit_implicits {
            let argument_entries = self.unbound_effect_variables(argument);
            for id in self.unbound_effect_variables(parameter) {
                if !argument_entries.contains(&id) {
                    self.try_default_effect_to_pure(id, location.clone());
                }
            }
        }

        self.propagate_effects(effects_var, locator);
    }

    /// If `target` is `Copy f` or `Drop f e` for a function type `f`, return `f`
    fn copied_or_dropped_function(&mut self, target: &Type) -> Option<Type> {
        let (_, subject) = self.copy_or_drop_subject(target)?;
        matches!(self.follow_type(&subject), Type::Function(_)).then_some(subject)
    }

    /// Adds `effects` to the ambient row, deferring what only the function's boundary can decide
    fn propagate_effects(&mut self, effects: &Type, locator: impl Locateable) {
        let (effects, deferred) = self.split_deferred_effects(effects);
        self.deferred_effects.extend(deferred);
        let current_row = self.current_effect_row.clone();
        self.unify(&effects, &current_row, TypeErrorKind::Effects, locator);

        let mut new_bindings = TypeBindings::default();
        let merged = self.collect_and_merge_effects(&current_row, &mut new_bindings);
        self.current_effect_row = RowKind::Effects.row_from_canonical(merged);
        self.bindings.extend(new_bindings);
    }

    /// True if `a` and `b` are equal except for one or more function environments.
    /// Assumes the two types are not equal to begin with (we only reach here after a failed
    /// unification), so if they unify once every function environment is erased, the
    /// environments must have been the sole cause of the failure.
    fn only_function_environments_differ(&self, a: &Type, b: &Type) -> bool {
        let a = strip_environments(&a.follow_all(&self.bindings));
        let b = strip_environments(&b.follow_all(&self.bindings));
        self.try_unify(&a, &b).is_ok()
    }

    /// Converts two types being compared in an error message to strings, rendering unbound literal
    /// variables as I32 or F64. Each trait's impl argument and each unknown argument which may be
    /// omitted is left out unless the two types would otherwise print the same, in which case
    /// rigid elided generics are numbered to tell them apart.
    fn type_pair_to_error_strings(&self, actual: &Type, expected: &Type) -> (String, String) {
        let to_string = |typ: &Type, omit: bool| {
            let printer = typ
                .display_with_literal_vars(
                    &self.bindings,
                    &self.integer_literal_vars,
                    &self.float_literal_vars,
                    self.current_context(),
                    self.compiler,
                )
                .hiding_environments();
            match omit {
                true => printer.omitting_unknown_and_impl_args().to_string(),
                false => printer.numbering_elided_generics().to_string(),
            }
        };
        let strings = (to_string(actual, true), to_string(expected, true));
        if strings.0 != strings.1 { strings } else { (to_string(actual, false), to_string(expected, false)) }
    }

    /// True if the given type is the `Never` type
    fn diverges(&self, typ: &Type) -> bool {
        matches!(typ.follow(&self.bindings), &Type::NEVER)
    }
}

/// The ways a loop can be exited by calling one of the operations loops desugar to
#[derive(Clone, Copy, PartialEq, Eq)]
enum LoopExit {
    /// `Std.Stream.(break)` or `Std.Stream.(continue)`, which the loop itself handles
    Jump,
    /// `Std.EarlyReturn.early_return`, which returns from the function the loop is in
    Return,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Variance {
    /// `a <: b` Most type relations are covariant
    Covariant,
    /// `b <: a` function parameter types are contravariant
    Contravariant,
    /// `a ~ b` mutable reference elements are invariant
    Invariant,
}

impl Variance {
    fn flip(self) -> Variance {
        match self {
            Variance::Covariant => Variance::Contravariant,
            Variance::Contravariant => Variance::Covariant,
            Variance::Invariant => Variance::Invariant,
        }
    }
}

/// The type or effect `name` exported by the Prelude
fn prelude_type_name(compiler: &DbHandle, name: &str) -> TopLevelName {
    let exported_types = ExportedTypes(SourceFileId::prelude()).get(compiler);
    *exported_types.get(&Arc::new(name.to_string())).unwrap_or_else(|| panic!("{name} not found in Prelude"))
}

/// Rewrite `typ`, erasing every function environment to [`Type::NO_CLOSURE_ENV`].
/// Used to compare two types while ignoring their closure environments.
fn strip_environments(typ: &Type) -> Type {
    match typ {
        Type::Function(function) => Type::Function(Shared::new(FunctionType {
            once: function.once.clone(),
            parameters: mapvec(&function.parameters, |param| {
                ParameterType::new(strip_environments(&param.typ), param.is_implicit)
            }),
            environment: Type::NO_CLOSURE_ENV,
            return_type: strip_environments(&function.return_type),
            effects: function.effects.clone(),
        })),
        Type::Application(constructor, args) => Type::Application(
            Shared::new(strip_environments(constructor)),
            Shared::new(mapvec(args.iter(), strip_environments)),
        ),
        Type::Tuple(elements) => Type::Tuple(Shared::new(mapvec(elements.iter(), strip_environments))),
        Type::Forall(generics, body) => Type::Forall(generics.clone(), Shared::new(strip_environments(body))),
        Type::Primitive(_)
        | Type::Generic(_)
        | Type::Variable(_)
        | Type::UserDefined(_)
        | Type::U32(_)
        | Type::Effects(_)
        | Type::Place(..)
        | Type::Places(_)
        | Type::PlaceProjection(..) => typ.clone(),
    }
}

/// Describes what a [`TypeChecker::try_coercion`] call rewrote, if anything.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum CoercionOutcome {
    /// No coercion was applied.
    None,
    /// The expression at the given `expr` was replaced; the caller should re-check it.
    ReplacedExpr,
    /// The expression at `expr` was wrapped to `ref expr` or `read expr`.
    /// The caller should undo any moves caused by `expr` when this occurs.
    AutoRef,
    /// The Call at `call_expr` had implicit arguments spliced in; the function expression
    /// itself is untouched and should not be re-checked against the reduced expected type.
    InPlaceCall,
    /// The function value at `expr` is coerced to a wider effect row.
    /// We want to create a function wrapper which has the additional effects but does not use them.
    FunctionEffects,
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// Try to apply a coercion between `actual` and `expected`.
    ///
    /// Possible coercions:
    /// - If `actual` is a function type with more implicit parameters than `expected` has,
    /// search for implicit values in scope and either splice them directly into the enclosing
    /// Call (when `call_expr` is `Some`) or wrap `expr` in a new lambda.
    /// - If `actual` is a reference type `r t` and `expected` is `t` (non-reference, non-variable),
    /// insert a `(.*)` call to auto-deref, requiring `t: Copy`.
    ///
    /// Returns a [`CoercionOutcome`] describing what (if anything) was rewritten.
    fn try_coercion(
        &mut self, actual: &Type, expected: &Type, expr: ExprId, call_expr: Option<ExprId>,
    ) -> CoercionOutcome {
        match (self.follow_type(actual), self.follow_type(expected)) {
            // The bottom type is already a subtype of `expected`
            (Type::Primitive(PrimitiveType::Never), _) => CoercionOutcome::None,
            // Fire when the callee needs implicit arguments the call site didn't supply.
            // We also allow a function with only implicit args to be called with `()` as its sole argument.
            (Type::Function(actual_fn), Type::Function(expected_fn))
                if actual_fn.parameters.len() != expected_fn.parameters.len()
                    || (call_expr.is_some_and(|call| self.call_ends_with_unit_arg(call))
                        && actual_fn.parameters.iter().filter(|p| p.is_implicit).count()
                            != expected_fn.parameters.iter().filter(|p| p.is_implicit).count()) =>
            {
                match self.implicit_parameter_coercion(actual_fn.clone(), expected_fn.clone(), expr, call_expr) {
                    Some(implicits::CoercionKind::Wrapper(new_expr)) => {
                        self.current_extended_context_mut().insert_expr(expr, new_expr);
                        if call_expr.is_none() {
                            self.item.coercion_wrapper_exprs.insert(expr);
                        }
                        CoercionOutcome::ReplacedExpr
                    },
                    Some(implicits::CoercionKind::DirectCallInsertion) => CoercionOutcome::InPlaceCall,
                    None => CoercionOutcome::None,
                }
            },
            (actual, expected) => {
                // Auto-deref: coerce `ref-kind t` to `t` by inserting a `(.*) expr` call if `Copy t`
                // can be found and `expected` is a concrete type.
                if let Type::Application(constructor, args) = &actual
                    && args.len() == 2
                    && matches!(self.follow_type(constructor), Type::Primitive(PrimitiveType::Reference))
                    && self.is_concrete_type_or_numeric_typevar(expected)
                    && expected.reference_element(&self.bindings).is_none()
                {
                    let arg = args[1].clone();
                    let expected = expected.clone();
                    if let Ok(bindings) = self.try_unify(&arg, &expected) {
                        self.bindings.extend(bindings);
                        let borrowed = call_expr.is_some() && self.callee_is_borrowed(&expected, expr);
                        let new_expr = self.auto_deref_coercion(expr, expected, borrowed);
                        self.current_extended_context_mut().insert_expr(expr, new_expr);
                        return CoercionOutcome::ReplacedExpr;
                    }
                }
                // Auto-ref: coerce `t` to `ref t` or `read t` by wrapping `expr` in a reference expression.
                // FIXME: simplify these rules. The `|| actual_is_place` allows us to bind to unbound
                // type variables which requires us to do some messy tracking, preventing them from
                // being bound to references themselves later. But without this type inference
                // really suffers.
                let actual_is_place = self.try_build_move_path(expr).is_some();
                let actual_is_concrete = self.is_concrete_type_or_numeric_typevar(actual);
                if let Type::Application(expected_ctor, expected_args) = &expected
                    && expected_args.len() == 2
                    && (actual_is_concrete || actual_is_place)
                    && actual.reference_element(&self.bindings).is_none()
                {
                    let kind = match self.follow_type(expected_ctor) {
                        Type::Primitive(PrimitiveType::Reference) => match self.uniform_mode(&expected_args[0]) {
                            // TODO: Remove distinction between Mut and Read
                            Some(PlaceMode::Mut) => actual_is_concrete.then_some(ReferenceKind::Mut),
                            Some(PlaceMode::Read) => Some(ReferenceKind::Read),
                            None => Some(ReferenceKind::Ref),
                        },
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        let inner = expected_args[1].clone();
                        let actual = actual.clone();
                        // Prevent the id from being bound to a reference later
                        if let Type::Variable(id) = self.follow_type(&actual) {
                            self.value_type_vars.insert(*id);
                        }
                        if let Ok(bindings) = self.try_unify(&actual, &inner) {
                            self.bindings.extend(bindings);
                            let new_expr = self.auto_ref_coercion(expr, kind, actual);
                            self.current_extended_context_mut().insert_expr(expr, new_expr);
                            return CoercionOutcome::AutoRef;
                        }
                    }
                }
                CoercionOutcome::None
            },
        }
    }

    /// Infer `expr` against `expected`, then coerce.
    /// If `allow_deref` is set, this will try to auto-deref `expr` if it is a reference to a Copy type,
    /// regardless of the `expected` type (ie. even if `expected` is a type variable).
    fn infer_and_coerce(&mut self, expr: ExprId, expected: &Type, kind: TypeErrorKind, allow_deref: bool) -> Type {
        let actual = self.infer_expr(expr, expected);
        if allow_deref
            && let Some((_, inner)) = actual.reference_element(&self.bindings)
            && self.type_is_copy(&inner, expr.locate(self))
        {
            let new_expr = self.auto_deref_coercion(expr, inner, false);
            self.current_extended_context_mut().insert_expr(expr, new_expr);
            self.check_expr(expr, expected, kind);
            return actual;
        }
        self.coerce(&actual, expected, expr, None, kind);
        actual
    }

    /// Applies [Self::try_coercion] then [Self::unify].
    /// Returns the actual result type (rather than the expected type)
    fn coerce(
        &mut self, actual: &Type, expected: &Type, expr: ExprId, call_expr: Option<ExprId>, kind: TypeErrorKind,
    ) -> Type {
        self.coerce_with(actual, expected, expr, call_expr, kind, call_expr.is_none())
    }

    /// Like [Self::coerce] but only coerces a function value to a wider effect row if `widen_row` is set
    fn coerce_with(
        &mut self, actual: &Type, expected: &Type, expr: ExprId, call_expr: Option<ExprId>, kind: TypeErrorKind,
        widen_row: bool,
    ) -> Type {
        let once;
        let actual = match self.once_coercion(actual, expected, expr) {
            OnceCoercion::None => actual,
            OnceCoercion::Wrapped => return self.check_expr(expr, expected, kind),
            OnceCoercion::Retyped(typ) => {
                once = typ;
                &once
            },
        };
        let result = if widen_row && self.try_function_row_coercion(actual, expected, expr, kind.clone()) {
            CoercionOutcome::FunctionEffects
        } else {
            self.try_coercion(actual, expected, expr, call_expr)
        };
        match result {
            CoercionOutcome::AutoRef => self.type_autoref_wrapper(expr, expected, kind),
            CoercionOutcome::ReplacedExpr => self.check_expr(expr, expected, kind),
            CoercionOutcome::None => {
                // A dictionary's trait effect args may be narrower than the parameter's, in which
                // case the mir builder adapts the dictionary to it
                if self.trait_has_effect_args(expected) {
                    self.coerce_dictionary(actual, expected, expr, kind);
                } else {
                    self.unify(actual, expected, kind, expr);
                }
                actual.clone()
            },
            // implicit_parameter_coercion already performed the needed unification against the type
            CoercionOutcome::InPlaceCall | CoercionOutcome::FunctionEffects => actual.clone(),
        }
    }

    /// Coerce a function that may be called any number of times into a `once fn`, its supertype.
    ///
    /// A `once fn` is moved into its call so its body is responsible for dropping its captures.
    /// Other functions leave that to whoever holds them, so unless the captures are `Copy` we need
    /// to wrap the function in a `once fn` which holds it, calls it, and then drops it.
    fn once_coercion(&mut self, actual: &Type, expected: &Type, expr: ExprId) -> OnceCoercion {
        let (Type::Function(actual_fn), Type::Function(expected_fn)) =
            (self.follow_type(actual), self.follow_type(expected))
        else {
            return OnceCoercion::None;
        };
        let once = |function: &FunctionType| *self.follow_type(&function.once) == Type::ONCE;
        if !once(expected_fn) || once(actual_fn) || actual_fn.parameters.len() != expected_fn.parameters.len() {
            return OnceCoercion::None;
        }
        let actual_fn = actual_fn.clone();

        let location = expr.locate(self);
        if self.type_is_copy(actual, location.clone()) {
            let retyped = FunctionType { once: Type::ONCE, ..(*actual_fn).clone() };
            return OnceCoercion::Retyped(Type::Function(Shared::new(retyped)));
        }

        let implicits_added = vec![None; actual_fn.parameters.len()];
        let wrapper = if self.local_variable(expr).is_some() {
            self.create_closure_wrapper_for_implicit(expr, implicits_added, actual_fn.parameters.clone())
        } else {
            // The function is evaluated here rather than each time the wrapper is called
            let value = self.current_extended_context()[expr].clone();
            let value = self.push_expr(value, actual.clone(), location.clone());
            let (path, name) = self.fresh_variable("function", actual.clone(), location.clone());
            let definition = self.let_binding(name, value);

            let function = self.push_expr(cst::Expr::Variable(path), actual.clone(), location.clone());
            let wrapper =
                self.create_closure_wrapper_for_implicit(function, implicits_added, actual_fn.parameters.clone());
            let wrapper = self.push_expr(wrapper, Type::ERROR, location);
            let item = |expr| cst::SequenceItem { comments: Vec::new(), expr };
            cst::Expr::Sequence(vec![item(definition), item(wrapper)])
        };
        self.current_extended_context_mut().insert_expr(expr, wrapper);
        OnceCoercion::Wrapped
    }

    /// Coerce a function value to a wider effect row, recording it for the MIR builder to wrap in a
    /// closure of the expected type. That closure captures the value, so the expected environment
    /// is bound to `actual` itself. Returns false if no coercion applies and the caller should unify.
    fn try_function_row_coercion(&mut self, actual: &Type, expected: &Type, expr: ExprId, kind: TypeErrorKind) -> bool {
        let Some((actual_fn, expected_fn)) = self.function_row_candidates(actual, expected) else { return false };

        // A re-check of a coerced expression must keep its coerced environment
        let already_coerced = self.current_extended_context().get_function_coercion(expr).is_some();
        if !already_coerced && self.effect_rows_identical(&actual_fn.effects, &expected_fn.effects) {
            return false;
        }
        let actual_fn = actual_fn.clone();
        let expected_fn = expected_fn.clone();

        // Without an environment to infer (e.g. a bare `fn` annotation) we can't introduce a
        // closure, so the rows need to match slot for slot. See [row::RowRelation::Same].
        if !already_coerced && !matches!(self.follow_type(&expected_fn.environment), Type::Variable(_)) {
            if !self.unify(actual, expected, kind, expr) {
                return true;
            }
            // `unify` already found the actual row within the expected one. If the reverse holds as
            // well then both rows have the same effects, although not necessarily in the same
            // slots.
            let same_effects =
                self.try_unify(&expected_fn.effects, &actual_fn.effects).map(|b| self.bindings.extend(b));
            let mut bindings = TypeBindings::default();
            let same_slots = self.row_unify(&actual_fn.effects, &expected_fn.effects, &mut bindings);
            if same_effects.is_ok() && same_slots.is_ok() {
                self.bindings.extend(bindings);
            } else if same_effects.is_ok() && self.is_top_level_function(expr) {
                // Top-level functions and effect operations can be called without an environment,
                // so we can wrap them in a function which passes each effect from the slot it
                // resolves to
                self.current_extended_context_mut().insert_function_coercion(expr, expected.clone());
            } else {
                let (actual, expected) = self.type_pair_to_error_strings(actual, expected);
                let location = expr.locate(self);
                let function_environments_differ = false;
                self.compiler.accumulate(Diagnostic::TypeError {
                    actual,
                    expected,
                    kind,
                    function_environments_differ,
                    location,
                });
            }
            return true;
        }

        let adapted = Type::Function(Shared::new(FunctionType {
            once: actual_fn.once.clone(),
            parameters: actual_fn.parameters.clone(),
            environment: Type::Function(actual_fn.clone()),
            return_type: actual_fn.return_type.clone(),
            effects: actual_fn.effects.clone(),
        }));
        self.unify(&adapted, expected, kind, expr);
        self.current_extended_context_mut().insert_function_coercion(expr, expected.clone());
        true
    }

    /// True if `expr` names a function defined at the top level or an effect operation
    fn is_top_level_function(&self, expr: ExprId) -> bool {
        let cst::Expr::Variable(path) = &self.current_extended_context()[expr] else { return false };
        matches!(self.path_origin(*path), Some(Origin::TopLevelDefinition(_) | Origin::EffectOperation { .. }))
    }

    /// True if checking `actual` against `expected` requires [Self::try_function_row_coercion]
    fn needs_function_row_coercion(&self, actual: &Type, expected: &Type) -> bool {
        self.function_row_candidates(actual, expected).is_some_and(|(actual_fn, expected_fn)| {
            !self.effect_rows_identical(&actual_fn.effects, &expected_fn.effects)
        })
    }

    /// Returns `actual` and `expected` as functions if they have the same arity
    fn function_row_candidates<'a>(
        &'a self, actual: &'a Type, expected: &'a Type,
    ) -> Option<(&'a Shared<FunctionType>, &'a Shared<FunctionType>)> {
        let (Type::Function(actual_fn), Type::Function(expected_fn)) =
            (self.follow_type(actual), self.follow_type(expected))
        else {
            return None;
        };
        let compatible = actual_fn.parameters.len() == expected_fn.parameters.len()
            // `resume` can't be wrapped in an evidence adapter, see [PrimitiveType::ResumeEnv]
            && !self.is_resume_env(&expected_fn.environment)
            && !self.is_resume_env(&actual_fn.environment);
        compatible.then_some((actual_fn, expected_fn))
    }

    /// True if `env` is the environment of a handler's `resume`
    fn is_resume_env(&self, env: &Type) -> bool {
        matches!(self.follow_type(env), Type::Primitive(PrimitiveType::ResumeEnv))
    }

    /// True if both rows have the same entries in the same order and the same open effect end.
    /// Ids are ignored since unification links them. Order matters since it fixes the evidence layout.
    fn effect_rows_identical(&self, a: &Type, b: &Type) -> bool {
        let no_bindings = TypeBindings::default();
        let a = self.collect_effects(a, &no_bindings);
        let b = self.collect_effects(b, &no_bindings);
        a.len() == b.len()
            && a.iter().zip(b.iter()).all(|(a, b)| a.follow_all(&self.bindings) == b.follow_all(&self.bindings))
    }

    /// Wrap the expression's type in the given expected reference type. `expected` is a full reference
    /// type, and `expr` will be wrapped with the same reference constructor of that type.
    /// Returns the actual type of the expression (rather than the expected type)
    fn type_autoref_wrapper(&mut self, expr: ExprId, expected: &Type, kind: TypeErrorKind) -> Type {
        let cst::Expr::Reference(reference) = self.current_extended_context()[expr].clone() else {
            unreachable!("type_autoref_wrapper called on a non-Reference expr")
        };
        let element = self.expr_types[&reference.rhs].clone();
        if reference.kind == ReferenceKind::Mut {
            self.check_mutable_borrow_of_place(reference.rhs);
        }
        let place = self.infer_place(reference.rhs);
        let typ = self.reference_type(reference.kind, place, element);
        self.expr_types.insert(expr, typ.clone());
        self.unify(&typ, expected, kind, expr);
        typ
    }

    /// False if the type is a non-numeric, unbound type variable. True otherwise.
    /// Non-numeric type variable here refers to one originating from a polymorphic int/float literal.
    fn is_concrete_type_or_numeric_typevar(&self, typ: &Type) -> bool {
        match self.follow_type(typ) {
            Type::Variable(id) => self.integer_literal_vars.contains(id) || self.float_literal_vars.contains(id),
            _ => true,
        }
    }

    /// True if `function` is one of `+ - * / %`,
    fn is_arithmetic_operator(&self, function: ExprId) -> bool {
        let name = match &self.current_extended_context()[function] {
            cst::Expr::Variable(path) => self.current_extended_context()[*path].last_ident(),
            cst::Expr::MemberAccess(access) => self.member_name(access).as_str(),
            _ => return false,
        };
        matches!(name, "+" | "-" | "*" | "/" | "%")
    }

    /// True if the function at `callee`, called through a reference, can be called in place rather
    /// than copied out of the reference. Only calling a `once fn` moves its captures, so if the
    /// function's type doesn't say yet we decide here that it isn't a `once fn`.
    ///
    /// Calling a `once fn` through a reference is an error unless it is `Copy`.
    fn callee_is_borrowed(&mut self, function: &Type, callee: ExprId) -> bool {
        let Type::Function(function_type) = self.follow_type(function).clone() else { return false };
        if !self.callee_is_once(&function_type) {
            return true;
        }
        let location = callee.locate(self);
        if self.type_is_copy(function, location.clone()) {
            return false;
        }
        self.report_unowned_once_call(crate::diagnostics::UnownedHolder::Reference, location);
        true
    }

    /// True if `function` is a `once fn`, so calling it moves it. If its type doesn't say yet, we
    /// decide here that it may be called any number of times.
    pub(super) fn callee_is_once(&mut self, function: &FunctionType) -> bool {
        self.bind_if_unbound(&function.once, Type::MANY);
        *self.follow_type(&function.once) == Type::ONCE
    }

    /// Report an error for calling a non-`Copy` `once fn` held by `holder` since it cannot be moved
    /// out of it
    pub(super) fn report_unowned_once_call(
        &mut self, holder: crate::diagnostics::UnownedHolder, location: crate::diagnostics::Location,
    ) {
        self.compiler.accumulate(Diagnostic::CallOfUnownedOnceFn { holder, location });
    }

    /// Returns the type of `expr` if it is a function type
    pub(super) fn expr_function_type(&self, expr: ExprId) -> Option<Shared<FunctionType>> {
        match self.follow_type(self.expr_types.get(&expr)?) {
            Type::Function(function) => Some(function.clone()),
            _ => None,
        }
    }

    /// True if `expr` is a call to the Prelude's `deref`, which reads a value in place
    pub(super) fn is_deref_call(&mut self, expr: ExprId) -> bool {
        let function = match self.resolved_expr(expr).as_ref() {
            cst::Expr::Call(call) => call.function,
            _ => return false,
        };
        let path = match self.resolved_expr(function).as_ref() {
            cst::Expr::Variable(path) => *path,
            _ => return false,
        };
        let origin = self.path_origin(path);
        origin.is_some() && origin == Some(self.prelude_definition_origin("deref"))
    }

    /// Synthesize a `(.*) expr` call expression for auto-deref coercion.
    /// The original expression at `expr` is copied to a new ExprId, and this returns
    /// a Call expression wrapping it with the Prelude's `(.*)` function.
    ///
    /// If `in_place` is set the value isn't copied so it doesn't need a `Copy` impl.
    fn auto_deref_coercion(&mut self, expr: ExprId, element_type: Type, in_place: bool) -> cst::Expr {
        let location = expr.locate(self);

        // Copy original expression to a new ExprId since we're replacing `expr`
        let original_expr = self.current_extended_context()[expr].clone();
        let original_type = self.expr_types[&expr].clone();
        let arg_id = self.push_expr(original_expr, original_type.clone(), location.clone());

        // A trivially `Copy` value, such as a trait dictionary, may have no `Copy` impl to call.
        // The Prelude's `deref` copies any value out of a reference.
        let deref_name = if in_place || self.is_trivially_copy(&element_type) { "deref" } else { ".*" };
        let function_type = Type::Function(Shared::new(FunctionType {
            once: Type::MANY,
            parameters: vec![ParameterType::explicit(original_type)],
            environment: Type::NO_CLOSURE_ENV,
            return_type: element_type,
            effects: Type::pure(),
        }));
        let func_expr = self.prelude_variable(deref_name, function_type, location);
        cst::Expr::Call(cst::Call { function: func_expr, arguments: vec![cst::Argument::explicit(arg_id)] })
    }

    /// Synthesize a `ref`/`read` expression wrapping the original expression for
    /// auto-ref coercion. The original expression at `expr` is copied to a new
    /// ExprId and the returned expression references it.
    fn auto_ref_coercion(&mut self, expr: ExprId, kind: ReferenceKind, element_type: Type) -> cst::Expr {
        let location = expr.locate(self);
        let original_expr = self.current_extended_context()[expr].clone();
        let rhs = self.push_expr(original_expr, element_type, location);
        self.copy_expr_metadata(expr, rhs);
        self.request_temporary_impls(rhs, false);
        cst::Expr::Reference(cst::Reference { kind, rhs, implicit: false })
    }

    /// Copy everything recorded about the expression `from` onto `to`, which it was moved to
    fn copy_expr_metadata(&mut self, from: ExprId, to: ExprId) {
        self.current_extended_context_mut().copy_expr_metadata(from, to);
        self.copy_value_impls(value_impls::OwnedRoot::Value(from), value_impls::OwnedRoot::Value(to));
        let item = self.current_item.expect("expressions are within an item");
        if let Some(pending) = self.pending_dictionary_coercions.get(&(item, from)).cloned() {
            self.pending_dictionary_coercions.insert((item, to), pending);
        }
    }

    /// Convert a type to a string for an error message, omitting any optional arguments which are unknown
    pub(crate) fn type_to_string(&self, typ: &Type) -> String {
        typ.display(&self.bindings, self.current_context(), self.compiler)
            .hiding_environments()
            .omitting_unknown_args()
            .to_string()
    }

    /// Check `actual <: expected`, returning `Err(())` on failure without pushing a Diagnostic.
    ///
    /// Returns any new bindings created on success.
    fn try_unify(&self, actual: &Type, expected: &Type) -> Result<TypeBindings, ()> {
        let mut bindings = TypeBindings::default();
        self.subtype(actual, expected, Variance::Covariant, RowMode::Coercible, &mut bindings).map(|_| bindings)
    }

    /// Is `a` a subtype of `b`? (iff `variance == Covariant`)
    ///
    /// `row_mode` controls whether effects will be treated as covariant or invariant.
    fn subtype(
        &self, a: &Type, b: &Type, variance: Variance, row_mode: RowMode, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        if variance == Variance::Contravariant {
            return self.subtype(b, a, Variance::Covariant, row_mode, new_bindings);
        }

        match (a, b) {
            (Type::Variable(a_id), b) => {
                if let Some(a) = self.bindings.get(a_id) {
                    self.subtype(a, b, variance, row_mode, new_bindings)
                } else if let Some(a) = new_bindings.get(a_id).cloned() {
                    self.subtype(&a, b, variance, row_mode, new_bindings)
                } else {
                    let b = b.follow_two(&self.bindings, new_bindings);

                    // If both are type variables, bind the higher id
                    // TODO: Investigate further, this fixes some Mir panics but is indicative of
                    // problems elsewhere
                    match b {
                        Type::Variable(b_id) if b_id > *a_id => {
                            self.try_bind_type_variable(b_id, Type::Variable(*a_id), new_bindings)
                        },
                        // `a` may already be an entry of `b`'s row
                        b @ Type::Places(_) if self.places_entry_is(&b, *a_id, new_bindings) => {
                            self.relate_places(a, &b, row_mode, variance, new_bindings)
                        },
                        b => self.try_bind_type_variable(*a_id, b, new_bindings),
                    }
                }
            },
            (a, Type::Variable(b_id)) => {
                if let Some(b) = self.bindings.get(b_id) {
                    self.subtype(a, b, variance, row_mode, new_bindings)
                } else if let Some(b) = new_bindings.get(b_id).cloned() {
                    self.subtype(a, &b, variance, row_mode, new_bindings)
                } else {
                    let a = a.follow_two(&self.bindings, new_bindings);

                    // `b` may already be an entry of `a`'s row
                    if matches!(a, Type::Places(_)) && self.places_entry_is(&a, *b_id, new_bindings) {
                        return self.relate_places(&a, b, row_mode, variance, new_bindings);
                    }

                    // Avoid b aliasing an open row to get subtype-like semantics instead of row polymorphism semantics
                    let coercible = variance == Variance::Covariant && row_mode == RowMode::Coercible;
                    let reopened = coercible.then(|| self.reopen_covariant_places(&a, new_bindings)).flatten();

                    self.try_bind_type_variable(*b_id, reopened.unwrap_or(a), new_bindings)
                }
            },
            // The bottom type is a subtype of every type in covariant position.
            // In invariant position only `Never == Never`.
            (Type::Primitive(PrimitiveType::Never), _) if matches!(variance, Variance::Covariant) => Ok(()),

            // The error type matches everything to prevent cascading errors.
            (Type::Primitive(PrimitiveType::Error), _) | (_, Type::Primitive(PrimitiveType::Error)) => Ok(()),
            (Type::Function(a_fn), Type::Function(b_fn)) => {
                if a_fn.parameters.len() != b_fn.parameters.len() {
                    return Err(());
                }

                // Parameters are contravariant, the return type is covariant.
                for (a_param, b_param) in a_fn.parameters.iter().zip(b_fn.parameters.iter()) {
                    self.subtype(&a_param.typ, &b_param.typ, variance.flip(), RowMode::Exact, new_bindings)?;
                }

                self.subtype(&a_fn.once, &b_fn.once, Variance::Invariant, RowMode::Exact, new_bindings)?;
                self.subtype(&a_fn.environment, &b_fn.environment, Variance::Invariant, RowMode::Exact, new_bindings)?;
                self.subtype(&a_fn.return_type, &b_fn.return_type, variance, RowMode::Exact, new_bindings)?;
                self.subtype(&a_fn.effects, &b_fn.effects, variance, row_mode, new_bindings)
            },
            (Type::Application(a_constructor, a_args), Type::Application(b_constructor, b_args)) => {
                if a_args.len() != b_args.len() {
                    return Err(());
                }

                // TODO: References are special-cased for now (mutable elements are invariant) but
                // users should be able to specify variance on their types in the future.
                // We can combine both branches by querying the constructor to get arg variances.
                if a_constructor.is_reference_constructor(&self.bindings)
                    && b_constructor.is_reference_constructor(&self.bindings)
                    && a_args.len() == 2
                {
                    self.subtype(&a_args[0], &b_args[0], variance, row_mode, new_bindings)?;

                    // Elements are only covariant through a reference that can never be mutable,
                    // otherwise a store through it could widen the original.
                    let read_only = self.places_mutability(&a_args[0], new_bindings) == Mutability::ReadOnly
                        || self.places_mutability(&b_args[0], new_bindings) == Mutability::ReadOnly;
                    // A reference to `Never` is unreachable so it may be used as any reference
                    let never = matches!(
                        a_args[1].follow_two(&self.bindings, new_bindings),
                        Type::Primitive(PrimitiveType::Never)
                    );
                    let element_variance = if read_only || never { variance } else { Variance::Invariant };
                    self.subtype(&a_args[1], &b_args[1], element_variance, RowMode::Exact, new_bindings)
                } else {
                    // A trait dictionary may only be adapted to wider effect args where the adaptation is
                    // recorded for the mir builder. See [Self::subtype_widening].
                    self.subtype(a_constructor, b_constructor, Variance::Invariant, RowMode::Exact, new_bindings)?;
                    for (a_arg, b_arg) in a_args.iter().zip(b_args.iter()) {
                        self.subtype(a_arg, b_arg, Variance::Invariant, RowMode::Exact, new_bindings)?;
                    }
                    Ok(())
                }
            },
            (Type::Forall(a_generics, a_body), Type::Forall(b_generics, b_body)) => {
                if a_generics.len() != b_generics.len() {
                    return Err(());
                }
                for (a_generic, b_generic) in a_generics.iter().zip(b_generics.iter()) {
                    let (a_generic, b_generic) = (a_generic.as_type(), b_generic.as_type());
                    self.subtype(&a_generic, &b_generic, Variance::Invariant, RowMode::Exact, new_bindings)?;
                }
                self.subtype(a_body, b_body, Variance::Invariant, RowMode::Exact, new_bindings)
            },
            (Type::UserDefined(a), Type::UserDefined(b)) => {
                if a == b {
                    Ok(())
                } else {
                    Err(())
                }
            },
            (Type::Generic(a), Type::Generic(b)) => {
                if a == b {
                    Ok(())
                } else {
                    Err(())
                }
            },
            // Prevents infinite recursion in row_subtype below which calls into try_unify
            (Type::UserDefined(_), Type::Application(..)) | (Type::Application(..), Type::UserDefined(_)) => Err(()),
            // Any of these variants can be an effect row or a places row
            (
                Type::UserDefined(_)
                | Type::Application(..)
                | Type::Effects(..)
                | Type::Generic(_)
                | Type::Places(..)
                | Type::Place(..)
                | Type::PlaceProjection(..),
                Type::UserDefined(_)
                | Type::Application(..)
                | Type::Effects(..)
                | Type::Generic(_)
                | Type::Places(..)
                | Type::Place(..)
                | Type::PlaceProjection(..),
            ) => {
                // TODO: Remove this check and combine the if branches below
                if a.is_place_row() || b.is_place_row() {
                    self.relate_places(a, b, row_mode, variance, new_bindings)
                } else {
                    match row_mode {
                        RowMode::Coercible => self.row_subtype(a, b, new_bindings),
                        RowMode::Exact => self.row_unify(a, b, new_bindings),
                    }
                }
            },
            (a, b) if a == b => Ok(()),
            _ => Err(()),
        }
    }

    /// True if `typ` is a trait applied to at least one effect row, the only dictionaries whose
    /// rows the mir builder can widen.
    pub(super) fn trait_has_effect_args(&self, typ: &Type) -> bool {
        let Some((trait_, Some(_))) = typ.trait_application(&self.bindings, self.compiler) else { return false };
        !trait_.effect_generics().all_false()
    }

    /// Find a non-skipped head-matching candidate that subtypes `target` per `variance`.
    pub(super) fn subtype_matching_effect(
        &self, candidates: &[Type], skip: impl Fn(usize) -> bool, target: &Type, variance: Variance,
        new_bindings: &mut TypeBindings,
    ) -> Option<usize> {
        for (i, candidate) in candidates.iter().enumerate() {
            if skip(i) || std::mem::discriminant(candidate) != std::mem::discriminant(target) {
                continue;
            }

            let mut trial = new_bindings.clone();
            if self.subtype(candidate, target, variance, RowMode::Exact, &mut trial).is_ok() {
                *new_bindings = trial;
                return Some(i);
            }
        }
        None
    }

    /// Flatten `effects` into a zonked, sorted list of effects
    pub(super) fn collect_and_merge_effects(&self, effects: &Type, new_bindings: &mut TypeBindings) -> Vec<Type> {
        let mut effects = self.collect_effects(effects, new_bindings);
        Type::follow_effects(&mut effects, &self.bindings, new_bindings);
        Type::sort_effects(&mut effects);
        effects
    }

    /// Flatten `effect` into a list of effects
    fn collect_effects(&self, effect: &Type, new_bindings: &TypeBindings) -> Vec<Type> {
        match effect.follow_two(&self.bindings, new_bindings) {
            Type::Effects(row) => {
                let mut found = Vec::new();
                if let Some(row) = row {
                    Type::flatten_effects_into(&row, &mut found, &self.bindings, new_bindings);
                }
                found
            },
            // A single effect, or an unbound variable standing in for the rest of the row.
            // TODO: Expand aliases or verify that we don't need to
            typ @ (Type::Application(..) | Type::UserDefined(..) | Type::Generic(_) | Type::Variable(_)) => vec![typ],
            // Any remaining variant should be a kind error, emitted elsewhere (TODO: verify)
            _ => Vec::new(),
        }
    }

    /// Try to bind a type variable, possibly erroring instead if the binding would lead
    /// to a recursive type. Inserts the binding into `new_bindings` on success.
    ///
    /// Before calling this function its argument must be zonked! `binding == binding.follow(...)`
    fn try_bind_type_variable(
        &self, id: TypeVariableId, binding: Type, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        if binding == Type::Variable(id) {
            // Already equal, don't recursively bind self to self
            Ok(())
        } else if self.occurs(&binding, id, new_bindings) {
            // Recursive type error
            Err(())
        } else if self.is_integer_type_variable(id) {
            self.try_bind_integer_or_float_type_variable(true, id, binding, new_bindings)
        } else if self.is_float_type_variable(id) {
            self.try_bind_integer_or_float_type_variable(false, id, binding, new_bindings)
        } else if self.value_type_vars.contains(&id) {
            self.try_bind_value_type_variable(id, binding, new_bindings)
        } else {
            new_bindings.insert(id, binding);
            Ok(())
        }
    }

    /// Bind a variable that was auto-ref'd as a value. Binding to a reference type is
    /// rejected so the wrapper never becomes a nested reference. This is needed to preserve
    /// type soundness currently.
    fn try_bind_value_type_variable(
        &self, id: TypeVariableId, binding: Type, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        match &binding {
            Type::Variable(other) if !self.is_integer_type_variable(*other) && !self.is_float_type_variable(*other) => {
                new_bindings.insert(*other, Type::Variable(id));
                return Ok(());
            },
            Type::Primitive(PrimitiveType::Error | PrimitiveType::Never) => return Ok(()),
            _ if binding.reference_element(&self.bindings).is_some() => return Err(()),
            _ => (),
        }
        new_bindings.insert(id, binding);
        Ok(())
    }

    /// Restrict polymorphic literal variables to types matching their literal kind.
    fn try_bind_integer_or_float_type_variable(
        &self, is_int: bool, id: TypeVariableId, binding: Type, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        let is_float = !is_int;

        match &binding {
            Type::Variable(other) => {
                let other_is_int = self.is_integer_type_variable(*other);
                let other_is_float = self.is_float_type_variable(*other);
                if (is_int && other_is_float) || (is_float && other_is_int) {
                    return Err(());
                }
                if !other_is_int && !other_is_float {
                    // Bind the unconstrained variable to the literal variable instead so
                    // we don't lose the int/float constraint on id by binding over it.
                    new_bindings.insert(*other, Type::Variable(id));
                    return Ok(());
                }
            },
            Type::Primitive(PrimitiveType::Int(_)) if is_int => (),
            Type::Primitive(PrimitiveType::Float(_)) if is_float => (),
            // Avoid binding to Error or Never to avoid leaking these types
            Type::Primitive(PrimitiveType::Error | PrimitiveType::Never) => return Ok(()),
            _ => return Err(()),
        }
        new_bindings.insert(id, binding);
        Ok(())
    }

    /// True if `variable` occurs within `typ`.
    /// Used to prevent the creation of infinitely recursive types when binding type variables.
    fn occurs(&self, typ: &Type, variable: TypeVariableId, new_bindings: &TypeBindings) -> bool {
        match typ {
            Type::Primitive(_) | Type::Generic(_) | Type::UserDefined(_) | Type::U32(_) | Type::Place(..) => false,
            Type::Variable(candidate_id) => {
                if let Some(binding) = self.bindings.get(candidate_id) {
                    self.occurs(binding, variable, new_bindings)
                } else if let Some(binding) = new_bindings.get(candidate_id) {
                    self.occurs(binding, variable, new_bindings)
                } else {
                    *candidate_id == variable
                }
            },
            Type::Function(function_type) => {
                self.occurs(&function_type.once, variable, new_bindings)
                    || function_type.parameters.iter().any(|param| self.occurs(&param.typ, variable, new_bindings))
                    || self.occurs(&function_type.environment, variable, new_bindings)
                    || self.occurs(&function_type.return_type, variable, new_bindings)
                    || self.occurs(&function_type.effects, variable, new_bindings)
            },
            Type::Application(constructor, args) => {
                self.occurs(constructor, variable, new_bindings)
                    || args.iter().any(|arg| self.occurs(arg, variable, new_bindings))
            },
            Type::Forall(_, typ) => self.occurs(typ, variable, new_bindings),
            Type::Tuple(elements) => elements.iter().any(|element| self.occurs(element, variable, new_bindings)),
            row @ Type::Effects(_) => {
                row.effect_entries().iter().any(|effect| self.occurs(effect, variable, new_bindings))
            },
            Type::Places(places) => places
                .as_ref()
                .is_some_and(|places| places.iter().any(|place| self.occurs(place, variable, new_bindings))),
            Type::PlaceProjection(base, ..) => self.occurs(base, variable, new_bindings),
        }
    }

    /// Retrieve a Type then follow all its type variable bindings so that we only return
    /// `Type::Variable` if the type variable is unbound. Note that this may still return
    /// a composite type such as `Type::Application` with bound type variables within.
    fn follow_type<'a>(&'a self, typ: &'a Type) -> &'a Type {
        typ.follow(&self.bindings)
    }

    /// Convert a [cst::Type] into a [Type]. If `allow_implicit_type_vars` is true, we'll
    /// insert type variables to make functions automatically polymorphic over effects or
    /// their closure environment. If false, we'll assume these to be pure or empty.
    ///
    /// Generally, `allow_implicit_type_vars` should be false in type definitions and true
    /// in function signatures or expressions.
    fn from_cst_type(&mut self, typ: &cst::Type, allow_implicit_type_vars: bool) -> Type {
        let mut local_kinds = LocalKinds::default();
        let implicit = allow_implicit_type_vars;
        self.from_cst_type_with_local_kinds(typ, kinds::Kind::Type, implicit, implicit, &mut local_kinds)
    }

    /// Build an initial [LocalKinds] map seeded from the explicit kind annotations on
    /// `generics`. Unannotated parameters default to [Kind::Type].
    pub(crate) fn local_kinds_from_generics(generics: &cst::Generics) -> LocalKinds {
        use crate::type_inference::{kinds::Kind, types::kind_from_annotation};
        map_btree(generics, |g| (g.name, g.kind.map(kind_from_annotation).unwrap_or(Kind::Type)))
    }

    /// Like [from_cst_type], but threads the caller's `local_kinds` map so that kinds inferred
    /// for type variables in this type are shared with sibling types in the same scope
    /// (e.g., multiple fields of one constructor).
    fn from_cst_type_with_local_kinds(
        &mut self, typ: &cst::Type, expected: kinds::Kind, allow_implicit_type_vars: bool,
        open_effects_by_default: bool, local_kinds: &mut LocalKinds,
    ) -> Type {
        let converted = self.with_next_id(|next_id| {
            Type::from_cst_type_with_kind(
                typ,
                expected.clone(),
                self.current_resolve(),
                self.compiler,
                next_id,
                local_kinds,
                allow_implicit_type_vars,
                open_effects_by_default,
                Omissions::Fresh,
            )
        });
        self.record_kinds(&converted, &expected);
        converted
    }

    /// Like [Self::from_cst_type_with_local_kinds] but for types within a type definition, where
    /// omitting a type argument defaulting to `_` is an error since there is nothing to infer it
    /// from. Effects are closed by default.
    fn from_cst_type_in_type_definition(
        &mut self, typ: &cst::Type, allow_implicit_type_vars: bool, local_kinds: &mut LocalKinds,
    ) -> Type {
        let converted = self.with_next_id(|next_id| {
            Type::from_cst_type_with_kind(
                typ,
                kinds::Kind::Type,
                self.current_resolve(),
                self.compiler,
                next_id,
                local_kinds,
                allow_implicit_type_vars,
                false,
                Omissions::Error,
            )
        });
        self.record_kinds(&converted, &kinds::Kind::Type);
        converted
    }

    /// See [Type::from_cst_constructor_type]
    fn from_cst_constructor_type(&mut self, typ: &cst::Type) -> Type {
        let converted = self.with_next_id(|next_id| {
            Type::from_cst_constructor_type(typ, self.current_resolve(), self.compiler, next_id)
        });
        self.record_kinds(&converted, &kinds::Kind::Type);
        converted
    }

    /// Try to retrieve the types of each field of the given type.
    /// Returns an empty map if unsuccessful.
    ///
    /// The map maps from the field name to a pair of (field type, field index).
    fn get_field_types(&mut self, typ: &Type, generic_args: Option<&[Type]>) -> BTreeMap<Name, (Type, u32)> {
        match self.follow_type(typ) {
            Type::Application(constructor, arguments) => {
                // TODO: Error if `generic_args` is non-empty
                let constructor = constructor.clone();
                let arguments = arguments.clone();
                // If the constructor is a reference or `Ptr`,
                // look up the fields of the inner type and wrap each field type in the
                // same constructor (so the field is accessible as an offset).
                let is_reference = matches!(self.follow_type(&constructor), Type::Primitive(PrimitiveType::Reference));
                let is_pointer = matches!(self.follow_type(&constructor), Type::Primitive(PrimitiveType::Pointer));
                if is_reference || is_pointer {
                    let (lifetime, inner) = if is_reference {
                        (Some(arguments[0].clone()), arguments[1].clone())
                    } else {
                        (None, arguments[0].clone())
                    };
                    let inner_fields = self.get_field_types(&inner, None);

                    // A reference to a reference reads through to the inner reference, and these
                    // are already its fields. They can only be mutated if both references allow it.
                    if inner.reference_or_pointer_element(&self.bindings).is_some() {
                        let none = TypeBindings::default();
                        let read_only = lifetime
                            .as_ref()
                            .is_some_and(|places| self.places_mutability(places, &none) != Mutability::Mutable);
                        return inner_fields
                            .into_iter()
                            .map(|(name, (field_type, index))| {
                                let field_type = match field_type.reference_element(&self.bindings) {
                                    Some((places, element)) if read_only => {
                                        self.reference_type(cst::ReferenceKind::Read, places, element)
                                    },
                                    _ => field_type,
                                };
                                (name, (field_type, index))
                            })
                            .collect();
                    }
                    return inner_fields
                        .into_iter()
                        .map(|(name, (field_type, index))| {
                            let wrapped_args = match &lifetime {
                                Some(lifetime) => {
                                    let field = [cst::PlaceSegment::Field(name.clone())];
                                    let none = TypeBindings::default();
                                    vec![Type::project_place(lifetime, &field, &self.bindings, &none), field_type]
                                },
                                None => vec![field_type],
                            };
                            let wrapped = Type::Application(constructor.clone(), Shared::new(wrapped_args));
                            (name, (wrapped, index))
                        })
                        .collect();
                }
                self.get_field_types(&constructor, Some(&arguments))
            },
            Type::UserDefined(origin) => {
                if let Origin::TopLevelDefinition(id) = origin {
                    let arguments = generic_args.and_then(|args| self.with_method_generics(*id, args, false));
                    let body = id.type_body(arguments.as_deref().or(generic_args), self.compiler);
                    match body {
                        TypeBody::Product { fields, .. } => {
                            let fields = fields.into_iter().enumerate();
                            return fields.map(|(i, (name, typ))| (name, (typ, i as u32))).collect();
                        },
                        TypeBody::Sum(variants) => {
                            // Common fields from a `with` clause are the trailing `k` entries of every variant
                            let k = id.common_field_count(self.compiler);
                            if k > 0 && !variants.is_empty() {
                                let (_, _, fields) = &variants[0];
                                let start = fields.len() - k;
                                return fields[start..]
                                    .iter()
                                    .enumerate()
                                    .map(|(i, (name, typ))| (name.clone(), (typ.clone(), i as u32)))
                                    .collect();
                            }
                        },
                    }
                }
                BTreeMap::default()
            },
            _ => BTreeMap::default(),
        }
    }

    /// The types of the members of the trait dictionary `typ`, applied to any supplied generics.
    fn impl_member_types(&mut self, typ: &Type) -> BTreeMap<Name, (Type, u32)> {
        if let Type::Application(constructor, arguments) = self.follow_type(typ)
            && let Type::UserDefined(Origin::TopLevelDefinition(name)) = self.follow_type(constructor)
            && let Some(arguments) = self.with_method_generics(*name, arguments, true)
        {
            return self.get_field_types(&constructor.clone(), Some(&arguments));
        }
        self.get_field_types(typ, None)
    }

    /// `arguments` of the trait `name`, followed by an argument for each generic its constructor
    /// promoted from its method signatures. A place in the signature itself is rigid if `rigid`
    /// is set, but one within a function parameter never is. Returns None if `name` is not a
    /// trait or it has no such generics.
    /// TODO: Rework this when rank-2 types are better implemented. Places should generally be
    /// rank-2 by default on a trait function.
    fn with_method_generics(&self, name: TopLevelName, arguments: &[Type], rigid: bool) -> Option<Vec<Type>> {
        let definition = TraitDefinition::get(name, self.compiler)?;
        let result = TypeCheck(name.top_level_item).get(self.compiler);
        let constructor = result.get_generalized(definition.definition().name);
        let Type::Forall(generics, body) = &constructor else { return None };
        let promoted = generics.get(arguments.len()..).filter(|promoted| !promoted.is_empty())?;

        let mut signature_places = FxHashSet::default();
        let mut all_places = FxHashSet::default();
        body.collect_method_places(false, &mut signature_places);
        body.collect_method_places(true, &mut all_places);

        let promoted = promoted.iter().map(|generic| {
            if rigid && signature_places.contains(generic) {
                Type::Generic(Generic::Elided(self.next_type_variable_id()))
            } else if all_places.contains(generic) {
                let none = TypeBindings::default();
                Type::places(&[self.next_variable(kinds::Kind::Place)], &none, &none)
            } else {
                self.next_type_variable()
            }
        });
        Some(arguments.iter().cloned().chain(promoted).collect())
    }

    /// Returns a set of substitutions for a user-defined type to replace instances of its generics
    /// with the given types. Care should be taken with the resulting substitutions map since the
    /// Generics within will each be `Origin::Local(name_id)` with a `name_id` local to the given
    /// TypeDefinition, which is likely in a different context than the rest of the TypeChecker.
    ///
    /// Typically, these substitutions can be used on a type within the given TypeDefinition via
    /// a combination of `convert_foreign_type` and `substitute_generics`.
    ///
    /// Does nothing if `replacements.len() != definition.generics.len()`
    fn datatype_generic_substitutions(
        definition: &cst::TypeDefinition, replacements: &[Type],
    ) -> FxHashMap<Generic, Type> {
        let mut substitutions = FxHashMap::default();
        if definition.generics.len() == replacements.len() {
            for (generic, replacement) in definition.generics.iter().zip(replacements) {
                substitutions.insert(Generic::Named(Origin::Local(generic.name)), replacement.clone());
            }
        }
        substitutions
    }

    /// If the current item is the main function, ensure it has the expected signature `fn Unit -> Unit pure`
    fn check_for_main(&mut self, pattern: PatternId, typ: &Type) {
        let Some(item) = self.current_item else { return };

        if item.source_file.crate_id != CrateId::LOCAL {
            return;
        }

        let cst::Pattern::Variable(name) = *self.pattern_of(pattern) else { return };
        if self.current_extended_context()[name].as_str() != "main" {
            return;
        }

        self.default_unshared_effects_to_pure(typ, typ);

        let main_effects = self.next_variable(kinds::Kind::Effect);

        let expected = Type::Function(Shared::new(FunctionType {
            once: Type::MANY,
            parameters: vec![ParameterType::explicit(Type::UNIT)],
            environment: Type::NO_CLOSURE_ENV,
            return_type: Type::UNIT,
            effects: main_effects.clone(),
        }));

        // TODO: Apply this type (and the following effect type) to main beforehand so
        // that errors can be in the main body where the incorrect effect is used rather
        // than on main after the inferred type is closed.
        self.unify(typ, &expected, TypeErrorKind::MainFn, pattern);

        let io = self.get_io_effects();
        self.unify(&main_effects, &io, TypeErrorKind::MainEffects, pattern);
    }
}

/// Returns each argument of the given function type.
/// If the given type is not a function, an empty Vec is returned.
impl Type {
    fn function_parameter_types(&self) -> impl ExactSizeIterator<Item = Type> {
        let parameters = match self {
            Type::Function(function) => function.parameters.as_slice(),
            _ => &[],
        };
        parameters.iter().map(|param| param.typ.clone())
    }
}
