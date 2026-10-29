use std::{borrow::Cow, cell::Cell, collections::BTreeMap, rc::Rc, sync::Arc};

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
pub mod errors;
mod free_variables;
pub mod fresh_expr;
pub mod generics;
pub mod get_type;
mod implicits;
pub mod kinds;
pub mod patterns;
mod places;
mod recursive_types;
mod row;
pub(crate) mod type_body;
mod type_definitions;
pub mod types;

pub use get_type::get_type_impl;
pub use type_body::TypeBody;

/// Actually type check a statement and its contents.
/// Unlike `get_type_impl`, this always type checks the expressions inside a statement
/// to ensure they type check correctly.
pub fn type_check_impl(context: &TypeCheckSCC, compiler: &DbHandle) -> Arc<TypeCheckSCCResult> {
    incremental::enter_query();
    let items = TypeChecker::item_contexts(&context.0, compiler);
    let mut checker = TypeChecker::new(&items, compiler);

    let items = mapvec(context.0.iter(), |item_id| {
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
        checker.check_impl_parameters_are_copy();

        if let TopLevelItemKind::Definition(definition) = &item.kind {
            checker.check_borrows(definition, depth);
        }
        (*item_id, checker.finish_item())
    });

    // Whether a dictionary coercion widens any effects may depend on later items in the SCC
    checker.resolve_dictionary_coercions();

    incremental::exit_query();
    Arc::new(checker.finish(items))
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

    /// Cached origins of Prelude definitions, by name. See [Self::prelude_definition_origin].
    prelude_origins: FxHashMap<&'static str, Origin>,

    /// Cached TopLevelName for the Prelude's `Copy` trait and its number of generics, lazily
    /// resolved on first use.
    copy_trait: Option<(TopLevelName, usize)>,

    /// The Prelude's `Mutate` effect
    mutate_name: TopLevelName,

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

    /// The `Copy` impl requested for each variable or member access expression in the current
    /// item that may move a local place.
    copy_witnesses: FxHashMap<ExprId, ExprId>,

    /// Names defined with `var` or as mutable parameters. Used by closure capture analysis
    /// to wrap mutable captures in a reference type so the closure shares the outer scope's storage.
    mutable_definitions: FxHashSet<NameId>,

    /// Failed implicit searches by destination
    failed_implicit_searches: FxHashMap<ExprId, implicits::ImplicitError>,

    /// Set while checking the constructor of an impl of a trait. See [ImplState].
    impl_: Option<ImplState>,
}

/// An impl of a trait whose constructor is being checked. Each of the impl's methods takes a
/// reference to the impl's data, which holds the impl's parameters, so a method reads them from
/// there rather than capturing them.
struct ImplState {
    /// The lambda of each of the impl's methods, see [TypeChecker::check_impl_method]
    methods: FxHashSet<ExprId>,

    /// The `Copy` impl requested for each of the impl's parameters which aren't trivially `Copy`.
    /// See [TypeChecker::check_impl_parameters_are_copy].
    copy_witnesses: Vec<(PatternId, ExprId)>,
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
            prelude_origins: Default::default(),
            copy_trait: None,
            mutate_name: prelude_type_name(compiler, "Mutate"),
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
        if let Some(origin) = self.prelude_origins.get(name) {
            return *origin;
        }
        let exported = ExportedDefinitions(SourceFileId::prelude()).get(self.compiler);
        let top_level_name = *exported
            .definitions
            .get(&Arc::new(name.to_string()))
            .unwrap_or_else(|| panic!("`{name}` not found in Prelude"));
        let (item, _) = GetItem(top_level_name.top_level_item).get(self.compiler);
        let origin = crate::name_resolution::origin_of_top_level_definition(top_level_name, &item.kind);
        self.prelude_origins.insert(name, origin);
        origin
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
        self.current_item = Some(item_id);
        self.item = ItemState::default();

        let (item, context, resolve) = &self.item_contexts[&item_id];
        self.current_impl = match &item.kind {
            TopLevelItemKind::Definition(definition) => {
                get_type::trait_impl_of(definition, context, resolve, self.compiler)
            },
            _ => None,
        };

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

    /// Finishes the current item, adding all bindings to the relevant entry in
    /// `self.finished_items`, clearing them out in preparation for resolving the next item.
    fn finish_item(&mut self) -> TypeMaps {
        self.current_item = None;
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

    /// Run `f` with a reference to `self.next_type_variable_id`
    fn with_next_id<T>(&self, f: impl FnOnce(&mut u32) -> T) -> T {
        let mut next_id = self.next_type_variable_id.get();
        let result = f(&mut next_id);
        self.next_type_variable_id.set(next_id);
        result
    }

    /// A fresh, open effect row.
    fn fresh_effect_row(&self) -> Type {
        Type::Effects(Some(Shared::new(vec![self.next_type_variable()])))
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
            let typ = self.declared_types.get(name).cloned().unwrap_or_else(|| typ.generalize(&self.bindings));
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

    /// Records every unbound effect row entry in `typ` so that [Self::try_default_effect_to_pure]
    /// can close it if nothing binds it by the end of the scope.
    ///
    /// TODO: Record these variables when created in `self.next_type_variable_id` instead of finding
    /// them here after the fact.
    fn record_fresh_effect_variables(&mut self, typ: &Type) {
        let ends = typ.unbound_effect_entries(&self.bindings);
        if ends.is_empty() {
            return;
        }
        let location = self.current_context().location().clone();
        for id in ends {
            self.push_inferred_effect(id, location.clone());
        }
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
        &mut self, effects_var: &Type, implicit_count_before_call: usize, explicit_implicits: &[(Type, Type)],
        locator: impl Locateable + Copy,
    ) {
        // Row subtyping lets an impl like `Eq I32 pure` match `Eq I32 e` without binding `e`,
        // but the impl decides that effect. Close whatever it left open before the call's row is
        // linked to the ambient row below, where an open `e` would absorb the whole row instead.
        // TODO: Remove. This is a workaround for the lack of a constraint-based system.
        let location = locator.locate(self);
        for target in self.resolve_new_delayed_implicits(implicit_count_before_call) {
            for id in target.unbound_effect_entries(&self.bindings) {
                self.try_default_effect_to_pure(id, location.clone());
            }
        }

        // The same goes for implicits given explicitly, although only the rows left open by matching
        // the argument against the parameter are closed. The argument's own open rows are left alone.
        for (parameter, argument) in explicit_implicits {
            let argument_entries = argument.unbound_effect_entries(&self.bindings);
            for id in parameter.unbound_effect_entries(&self.bindings) {
                if !argument_entries.contains(&id) {
                    self.try_default_effect_to_pure(id, location.clone());
                }
            }
        }

        self.propagate_effects(effects_var, locator);
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
        | Type::Place(_)
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
    /// The expression at `expr` was wrapped to `ref expr` or `imm expr`.
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
                    && matches!(self.follow_type(constructor), Type::Primitive(PrimitiveType::Reference(_)))
                    && self.is_concrete_type_or_numeric_typevar(expected)
                    && expected.reference_element(&self.bindings).is_none()
                {
                    let arg = args[1].clone();
                    let expected = expected.clone();
                    if let Ok(bindings) = self.try_unify(&arg, &expected) {
                        self.bindings.extend(bindings);
                        let new_expr = self.auto_deref_coercion(expr, expected);
                        self.current_extended_context_mut().insert_expr(expr, new_expr);
                        return CoercionOutcome::ReplacedExpr;
                    }
                }
                // Auto-ref: coerce `t` to `ref t` or `imm t` by wrapping `expr` in a reference expression.
                // FIXME: simplify these rules. The `|| actual_is_place` allows us to bind to unbound
                // type variables which requires us to do some messy tracking, preventing them from
                // being bound to references themselves later. But without this type inference
                // really suffers.
                let actual_is_place = self.try_build_move_path(expr).is_some();
                if let Type::Application(expected_ctor, expected_args) = &expected
                    && expected_args.len() == 2
                    && (self.is_concrete_type_or_numeric_typevar(actual) || actual_is_place)
                    && actual.reference_element(&self.bindings).is_none()
                {
                    let kind = match self.follow_type(expected_ctor) {
                        Type::Primitive(PrimitiveType::Reference(kind)) => Some(*kind),
                        _ => None,
                    };
                    if let Some(kind) = kind
                        && matches!(kind, ReferenceKind::Ref | ReferenceKind::Imm)
                    {
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
            let new_expr = self.auto_deref_coercion(expr, inner);
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

        // Without an environment to infer (e.g. a bare `fn` annotation) no closure can be introduced
        if !already_coerced && !matches!(self.follow_type(&expected_fn.environment), Type::Variable(_)) {
            if self.unify(actual, expected, kind, expr)
                && let Err(()) =
                    self.try_unify(&expected_fn.effects, &actual_fn.effects).map(|b| self.bindings.extend(b))
            {
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
            parameters: actual_fn.parameters.clone(),
            environment: Type::Function(actual_fn.clone()),
            return_type: actual_fn.return_type.clone(),
            effects: actual_fn.effects.clone(),
        }));
        self.unify(&adapted, expected, kind, expr);
        self.current_extended_context_mut().insert_function_coercion(expr, expected.clone());
        true
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
        let place = self.infer_place(reference.rhs);
        let constructor = Type::reference(reference.kind);
        let typ = Type::Application(Shared::new(constructor), Shared::new(vec![place, element]));
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

    /// Synthesize a `(.*) expr` call expression for auto-deref coercion.
    /// The original expression at `expr` is copied to a new ExprId, and this returns
    /// a Call expression wrapping it with the Prelude's `(.*)` function.
    fn auto_deref_coercion(&mut self, expr: ExprId, element_type: Type) -> cst::Expr {
        let location = expr.locate(self);

        // Copy original expression to a new ExprId since we're replacing `expr`
        let original_expr = self.current_extended_context()[expr].clone();
        let original_type = self.expr_types[&expr].clone();
        let arg_id = self.push_expr(original_expr, original_type.clone(), location.clone());

        // A trivially `Copy` value, such as a trait dictionary, may have no `Copy` impl to call.
        // The Prelude's `deref` copies any value out of a reference.
        let deref_name = if self.is_trivially_copy(&element_type) { "deref" } else { ".*" };
        let deref_origin = self.prelude_definition_origin(deref_name);
        let deref_path = self.push_path(
            cst::Path::ident(Arc::new(deref_name.to_string()), location.clone()),
            Type::ERROR, // overwritten when check_expr re-checks the synthesized call
            location.clone(),
        );
        self.current_extended_context_mut().insert_path_origin(deref_path, deref_origin);

        let function_type = Type::Function(Shared::new(FunctionType {
            parameters: vec![ParameterType::explicit(original_type)],
            environment: Type::NO_CLOSURE_ENV,
            return_type: element_type,
            effects: Type::pure(),
        }));
        let func_expr = self.push_expr(cst::Expr::Variable(deref_path), function_type, location);
        cst::Expr::Call(cst::Call { function: func_expr, arguments: vec![cst::Argument::explicit(arg_id)] })
    }

    /// Synthesize a `ref`/`imm` expression wrapping the original expression for
    /// auto-ref coercion. The original expression at `expr` is copied to a new
    /// ExprId and the returned expression references it.
    fn auto_ref_coercion(&mut self, expr: ExprId, kind: ReferenceKind, element_type: Type) -> cst::Expr {
        let location = expr.locate(self);
        let original_expr = self.current_extended_context()[expr].clone();
        let rhs = self.push_expr(original_expr, element_type, location);
        self.copy_expr_metadata(expr, rhs);
        cst::Expr::Reference(cst::Reference { kind, rhs })
    }

    /// Copy everything recorded about the expression `from` onto `to`, which it was moved to
    fn copy_expr_metadata(&mut self, from: ExprId, to: ExprId) {
        self.current_extended_context_mut().copy_expr_metadata(from, to);
        if let Some(witness) = self.item.copy_witnesses.get(&from).copied() {
            self.item.copy_witnesses.insert(to, witness);
        }
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
                            self.relate_places(a, &b, row_mode, new_bindings)
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

                self.subtype(&a_fn.environment, &b_fn.environment, Variance::Invariant, RowMode::Exact, new_bindings)?;
                self.subtype(&a_fn.return_type, &b_fn.return_type, variance, RowMode::Exact, new_bindings)?;
                self.subtype(&a_fn.effects, &b_fn.effects, variance, row_mode, new_bindings)
            },
            (Type::Application(a_constructor, a_args), Type::Application(b_constructor, b_args)) => {
                if a_args.len() != b_args.len() {
                    return Err(());
                }

                // TODO: References are special-cased for now (mut args are invariant) but
                // users should be able to specify variance on their types in the future.
                // We can combine both branches by querying the constructor to get arg variances.
                let a_kind = a_constructor.reference_constructor(&self.bindings);
                let b_kind = b_constructor.reference_constructor(&self.bindings);
                if let (Some(_), Some(b_kind)) = (a_kind, b_kind) {
                    // Reference-kind subtyping (e.g. `uniq <: mut`) is handled by the arm below.
                    self.subtype(a_constructor, b_constructor, variance, RowMode::Exact, new_bindings)?;

                    // `mut`/`uniq` elements are invariant
                    let read_only = matches!(b_kind, ReferenceKind::Imm | ReferenceKind::Ref);
                    let element_variance = if !read_only { Variance::Invariant } else { variance };

                    for (index, (a_arg, b_arg)) in a_args.iter().zip(b_args.iter()).enumerate() {
                        let (arg_variance, row_mode) = match index {
                            0 => (variance, row_mode),
                            _ => (element_variance, RowMode::Exact),
                        };
                        self.subtype(a_arg, b_arg, arg_variance, row_mode, new_bindings)?;
                    }
                    Ok(())
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
            (Type::Primitive(PrimitiveType::Reference(a_kind)), Type::Primitive(PrimitiveType::Reference(b_kind))) => {
                match (a_kind, b_kind) {
                    (_, ReferenceKind::Ref) => Ok(()),
                    (ReferenceKind::Uniq, ReferenceKind::Mut) => Ok(()),
                    (ReferenceKind::Uniq, ReferenceKind::Imm) => Ok(()),
                    (_, _) if a_kind == b_kind => Ok(()),
                    _ => Err(()),
                }
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
                | Type::Place(_)
                | Type::PlaceProjection(..),
                Type::UserDefined(_)
                | Type::Application(..)
                | Type::Effects(..)
                | Type::Generic(_)
                | Type::Places(..)
                | Type::Place(_)
                | Type::PlaceProjection(..),
            ) => {
                // TODO: Remove this check and combine the if branches below
                if a.is_place_row() || b.is_place_row() {
                    self.relate_places(a, b, row_mode, new_bindings)
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

    /// Flatten `effects` into a zonked, sorted, deduplicated list of effects
    pub(super) fn collect_and_merge_effects(&self, effects: &Type, new_bindings: &mut TypeBindings) -> Vec<Type> {
        let mut effects = self.collect_effects(effects, new_bindings);
        Type::follow_effects(&mut effects, &self.bindings, new_bindings);
        Type::sort_and_dedup_effects(&mut effects);
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
            Type::Primitive(_) | Type::Generic(_) | Type::UserDefined(_) | Type::U32(_) | Type::Place(_) => false,
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
                function_type.parameters.iter().any(|param| self.occurs(&param.typ, variable, new_bindings))
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
            Type::PlaceProjection(base, _) => self.occurs(base, variable, new_bindings),
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
        self.with_next_id(|next_id| {
            Type::from_cst_type_with_kind(
                typ,
                expected,
                self.current_resolve(),
                self.compiler,
                next_id,
                local_kinds,
                allow_implicit_type_vars,
                open_effects_by_default,
                Omissions::Fresh,
            )
        })
    }

    /// Like [Self::from_cst_type_with_local_kinds] but for types within a type definition, where
    /// omitting a type argument defaulting to `_` is an error since there is nothing to infer it
    /// from. Effects are closed by default.
    fn from_cst_type_in_type_definition(
        &mut self, typ: &cst::Type, allow_implicit_type_vars: bool, local_kinds: &mut LocalKinds,
    ) -> Type {
        self.with_next_id(|next_id| {
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
        })
    }

    /// See [Type::from_cst_constructor_type]
    fn from_cst_constructor_type(&mut self, typ: &cst::Type) -> Type {
        self.with_next_id(|next_id| {
            Type::from_cst_constructor_type(typ, self.current_resolve(), self.compiler, next_id)
        })
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
                // If the constructor is a reference kind (mut, ref, imm, uniq) or `Ptr`,
                // look up the fields of the inner type and wrap each field type in the
                // same constructor (so the field is accessible as an offset).
                let is_reference =
                    matches!(self.follow_type(&constructor), Type::Primitive(PrimitiveType::Reference(_)));
                let is_pointer = matches!(self.follow_type(&constructor), Type::Primitive(PrimitiveType::Pointer));
                if is_reference || is_pointer {
                    let (lifetime, inner) = if is_reference {
                        (Some(arguments[0].clone()), arguments[1].clone())
                    } else {
                        (None, arguments[0].clone())
                    };
                    let inner_fields = self.get_field_types(&inner, None);
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
                    let body = id.type_body(generic_args, self.compiler);
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

        let context = self.current_context();
        let cst::Pattern::Variable(name) = context[pattern] else { return };
        if context[name].as_str() != "main" {
            return;
        }

        self.default_unshared_effects_to_pure(typ, typ);

        let main_effects = self.next_type_variable();

        let expected = Type::Function(Shared::new(FunctionType {
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
