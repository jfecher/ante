use std::{cell::Cell, collections::BTreeMap, rc::Rc, sync::Arc};

use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::Diagnostic,
    incremental::{
        self, DbHandle, ExportedDefinitions, ExportedTypes, GetItem, Resolve, TargetPointerSize, TypeCheckSCC,
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
        ids::{ExprId, NameId, PathId, PatternId, TopLevelId, TopLevelName},
    },
    type_inference::{
        errors::{Locateable, TypeErrorKind},
        fresh_expr::ExtendedTopLevelContext,
        generics::Generic,
        implicits::ImplicitsContext,
        types::{FunctionType, LocalKinds, ParameterType, PrimitiveType, Type, TypeBindings, TypeVariableId},
    },
};

mod affine;
mod cst_traversal;
pub mod dependency_graph;
pub mod errors;
mod free_variables;
pub mod fresh_expr;
pub mod generics;
pub mod get_type;
mod implicits;
pub mod kinds;
pub mod patterns;
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
        incremental::println(format!("Type checking {item_id:?}"));
        checker.start_item(*item_id);
        checker.push_implicits_scope();

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

        checker.pop_implicits_scope();
        (*item_id, checker.finish_item())
    });

    incremental::exit_query();
    Arc::new(checker.finish(items))
}

/// A `TypeCheckSCCResult` holds the `IndividualTypeCheckResult` of every item in
/// the SCC for a particular TopLevelId
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeCheckSCCResult {
    pub items: BTreeMap<TopLevelId, IndividualTypeCheckResult>,
    pub bindings: Arc<TypeBindings>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndividualTypeCheckResult {
    #[serde(flatten)]
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

    /// Types of each top-level item in the current SCC being worked on
    item_types: Rc<FxHashMap<TopLevelName, Type>>,

    /// The outer Vec represents each scope (roughly each block of code),
    /// while the inner Vec is the implicits context for that scope. This contains
    implicits: Vec<ImplicitsContext>,

    /// Tracks ExprIds for which `check_lambda` was called due to an implicit parameter coercion
    /// wrapper. For these, `check_for_closure` is deferred until after `pop_implicits_scope` of
    /// the enclosing lambda resolves the delayed implicits that fill in the wrapper's free vars.
    coercion_wrapper_exprs: FxHashSet<ExprId>,

    /// Cached type for the Prelude's `String` struct, lazily resolved on first use.
    string_type: Option<Type>,

    /// Cached TopLevelName for the Prelude's `(.*)` (deref/Copy) function, lazily resolved on first use.
    deref_name: Option<TopLevelName>,

    /// Tracks which local variables (and their sub-paths) have been moved.
    /// Used for affine type checking: non-Copy values may only be used once.
    move_tracker: affine::MoveTracker,

    /// When true, suppresses the "is this path already moved" check in `check_path`.
    /// Kept `false` inside `check_reference` - `ref x` must still verify `x` is valid
    /// even though it doesn't itself record a move.
    suppress_move_check: bool,

    /// When true, suppresses recording a move in `check_path`.
    /// Set by `check_reference` (ref doesn't move), the member-access / method-call
    /// object probes (partial-move tracking is done at the field level), and the
    /// plain `x := v` LHS (reassignment reads nothing from `x`).
    suppress_move_record: bool,

    /// Keep track of which variable pattern aliases alias to catch double or partial
    /// moves when both an alias and the original name are moved.
    binding_places: FxHashMap<NameId, affine::MovePath>,

    /// Cached TopLevelName for the Prelude's `Copy` type, lazily resolved on first use.
    copy_type_name: Option<TopLevelName>,

    /// Names defined with `var` or as mutable parameters. Used by closure capture analysis
    /// to wrap mutable captures in a reference type so the closure shares the outer scope's storage.
    mutable_definitions: FxHashSet<NameId>,

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

/// Map from each TopLevelId to a tuple of (the item, parse context, resolution context)
type ItemContexts = FxHashMap<TopLevelId, (Arc<TopLevelItem>, Arc<DesugarContext>, Arc<ResolutionResult>)>;

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
            current_item: None,
            function_return_type: None,
            current_effect_row: Type::pure(),
            item_contexts,
            id_contexts,
            implicits: Vec::new(),
            coercion_wrapper_exprs: Default::default(),
            string_type: None,
            deref_name: None,
            move_tracker: Default::default(),
            suppress_move_check: false,
            suppress_move_record: false,
            binding_places: Default::default(),
            copy_type_name: None,
            mutable_definitions: Default::default(),
            integer_literal_vars: Default::default(),
            float_literal_vars: Default::default(),
            value_type_vars: Default::default(),
        };

        let mut item_types = FxHashMap::default();
        for (item_id, (item, context, resolution)) in item_contexts.iter() {
            for name in resolution.top_level_names.iter() {
                let typ = if let TopLevelItemKind::Definition(definition) = &item.kind {
                    let next_id = &mut this.next_type_variable_id.get();

                    let typ = get_type::get_partial_type(definition, context.as_ref(), resolution, compiler, next_id);

                    this.next_type_variable_id.set(*next_id);
                    typ
                } else {
                    this.next_type_variable()
                };
                item_types.insert(TopLevelName::new(*item_id, *name), typ);
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

    /// Return the current extended context.
    /// Note that this context only includes new items added by this type checker, it does
    /// not contain any existing items from the resolver until the type checker finishes
    /// and inserts the pre-existing items.
    fn current_extended_context(&self) -> &ExtendedTopLevelContext {
        let item = self.current_item.expect("TypeChecker: Expected current_item to be set");
        self.id_contexts.get(&item).expect("Expected TopLevelId to be in id_contexts")
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
        let exported_types = ExportedTypes(SourceFileId::prelude()).get(self.compiler);
        let top_level_name =
            exported_types.get(&Arc::new("String".to_string())).expect("String type not found in Prelude");
        let typ = Type::UserDefined(Origin::TopLevelDefinition(*top_level_name));
        self.string_type = Some(typ.clone());
        typ
    }

    /// Returns the TopLevelName for the Prelude's `(.*)` (deref/Copy) function, caching it.
    fn get_deref_name(&mut self) -> TopLevelName {
        if let Some(name) = self.deref_name {
            return name;
        }
        let exported = ExportedDefinitions(SourceFileId::prelude()).get(self.compiler);
        let top_level_name = exported.definitions.get(&Arc::new(".*".to_string())).expect("(.*) not found in Prelude");
        self.deref_name = Some(*top_level_name);
        *top_level_name
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
                (id, IndividualTypeCheckResult { maps, generalized, context })
            })
            .collect();

        TypeCheckSCCResult { items, bindings: Arc::new(self.bindings) }
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
        self.move_tracker = Default::default();
        self.binding_places = Default::default();

        // Iterating over every item type here should be fine for performance.
        // The expected length of `self.item_types` is 1 in the vast majority of cases,
        // and is only a bit longer with mutually recursive type-inferred definitions
        // and definitions defining multiple names (e.g. `a, b = 1, 2`).
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

    /// A fresh, open effect row.
    fn fresh_effect_row(&self) -> Type {
        Type::effects(Vec::new(), Some(self.next_type_variable()))
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
            let typ = typ.generalize(&self.bindings);
            items.entry(name.top_level_item).or_default().insert(name.local_name_id, typ);
        }

        items
    }

    /// Defaults a `can e` to `pure` when `e` isn't referenced elsewhere in `root`, recursing into nested function types.
    fn default_unshared_effects_to_pure(&mut self, root: &Type, typ: &Type) {
        let Type::Function(function_type) = typ.follow(&self.bindings) else { return };
        let parameter_types: Vec<Type> = function_type.parameters.iter().map(|p| p.typ.clone()).collect();
        let return_type = function_type.return_type.clone();

        if let Type::Effects(_, Some(tail)) = function_type.effects.follow_all(&self.bindings)
            && let Type::Variable(tail_id) = *tail
        {
            let occurrences = root.count_unification_var_occurrences(tail_id, &self.bindings);
            if occurrences <= 1 {
                self.unify(&tail, &Type::pure(), TypeErrorKind::Effects, self.current_context().location().clone());
            }
        }

        for parameter_type in &parameter_types {
            self.default_unshared_effects_to_pure(root, parameter_type);
        }
        self.default_unshared_effects_to_pure(root, &return_type);
    }

    /// Unifies the two types. Returns false on failure
    ///
    /// TODO: Rename. This is actually a subtyping relation of `actual <: expected`
    fn unify(&mut self, actual: &Type, expected: &Type, kind: TypeErrorKind, locator: impl Locateable) -> bool {
        if let Ok(new_bindings) = self.try_unify(actual, expected) {
            self.bindings.extend(new_bindings);
            true
        } else {
            let function_environments_differ = self.only_function_environments_differ(actual, expected);
            let actual = self.type_to_error_string(actual);
            let expected = self.type_to_error_string(expected);
            let location = locator.locate(self);
            self.compiler.accumulate(Diagnostic::TypeError {
                actual,
                expected,
                kind,
                function_environments_differ,
                location,
            });
            false
        }
    }

    /// Unifies `effects_var` with the ambient effect row, then re-canonicalizes it as the new ambient row.
    fn thread_call_effects(&mut self, effects_var: &Type, locator: impl Locateable) {
        let current_row = self.current_effect_row.clone();
        self.unify(effects_var, &current_row, TypeErrorKind::Effects, locator);
        self.current_effect_row = self.canonical_effects_row(&current_row, &TypeBindings::default());
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

    /// Like [Self::type_to_string] but renders unbound literal variables as I32 or F64.
    fn type_to_error_string(&self, typ: &Type) -> String {
        typ.display_with_literal_vars(
            &self.bindings,
            &self.integer_literal_vars,
            &self.float_literal_vars,
            self.current_context(),
            self.compiler,
        )
        .hiding_environments()
        .to_string()
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

/// Rewrite `typ`, erasing every function environment to [`Type::NO_CLOSURE_ENV`].
/// Used to compare two types while ignoring their closure environments.
fn strip_environments(typ: &Type) -> Type {
    match typ {
        Type::Function(function) => Type::Function(Arc::new(FunctionType {
            parameters: mapvec(&function.parameters, |param| {
                ParameterType::new(strip_environments(&param.typ), param.is_implicit)
            }),
            environment: Type::NO_CLOSURE_ENV,
            return_type: strip_environments(&function.return_type),
            effects: function.effects.clone(),
        })),
        Type::Application(constructor, args) => Type::Application(
            Arc::new(strip_environments(constructor)),
            Arc::new(mapvec(args.iter(), strip_environments)),
        ),
        Type::Tuple(elements) => Type::Tuple(Arc::new(mapvec(elements.iter(), strip_environments))),
        Type::Forall(generics, body) => Type::Forall(generics.clone(), Arc::new(strip_environments(body))),
        Type::Primitive(_)
        | Type::Generic(_)
        | Type::Variable(_)
        | Type::UserDefined(_)
        | Type::U32(_)
        | Type::Effects(_, _) => typ.clone(),
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
                            self.coercion_wrapper_exprs.insert(expr);
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
        let saved = self
            .try_build_move_path(expr)
            .map(|path| affine::SavedMove { location: self.move_tracker.save_move(&path), path });

        let actual = self.infer_expr(expr, expected);
        if allow_deref
            && let Some((_, inner)) = actual.reference_element(&self.bindings)
            && self.type_is_copy(&inner)
        {
            let new_expr = self.auto_deref_coercion(expr, inner);
            self.current_extended_context_mut().insert_expr(expr, new_expr);
            let old_check = std::mem::replace(&mut self.suppress_move_check, true);
            let old_record = std::mem::replace(&mut self.suppress_move_record, true);
            self.check_expr(expr, expected, kind);
            self.suppress_move_check = old_check;
            self.suppress_move_record = old_record;
            return actual;
        }
        self.coerce(&actual, expected, expr, None, kind, saved);
        actual
    }

    /// Applies [Self::try_coercion] then [Self::unify].
    /// Returns the actual result type (rather than the expected type)
    fn coerce(
        &mut self, actual: &Type, expected: &Type, expr: ExprId, call_expr: Option<ExprId>, kind: TypeErrorKind,
        saved: Option<affine::SavedMove>,
    ) -> Type {
        let result = self.try_coercion(actual, expected, expr, call_expr);
        let actual = match result {
            CoercionOutcome::AutoRef => self.type_autoref_wrapper(expr, expected, kind),
            CoercionOutcome::ReplacedExpr => {
                // Re-check the wrapper but ignore moves since they were already recorded.
                let old_check = std::mem::replace(&mut self.suppress_move_check, true);
                let old_record = std::mem::replace(&mut self.suppress_move_record, true);
                let actual = self.check_expr(expr, expected, kind);
                self.suppress_move_check = old_check;
                self.suppress_move_record = old_record;
                actual
            },
            CoercionOutcome::None => {
                self.unify(actual, expected, kind, expr);
                actual.clone()
            },
            // implicit_parameter_coercion already performed the needed unification against the type
            CoercionOutcome::InPlaceCall => actual.clone(),
        };
        // Undo any moves if an auto-ref occurred
        if let (CoercionOutcome::AutoRef, Some(saved)) = (&result, saved) {
            self.move_tracker.restore_move(&saved.path, saved.location);
        }
        actual
    }

    /// Wrap the expression's type in the given expected reference type. `expected` is a full reference
    /// type, and `expr` will be wrapped with the same reference constructor of that type.
    /// Returns the actual type of the expression (rather than the expected type)
    fn type_autoref_wrapper(&mut self, expr: ExprId, expected: &Type, kind: TypeErrorKind) -> Type {
        let cst::Expr::Reference(reference) = self.current_extended_context()[expr].clone() else {
            unreachable!("type_autoref_wrapper called on a non-Reference expr")
        };
        let element = self.expr_types[&reference.rhs].clone();
        let lifetime = self.next_type_variable();
        let constructor = Type::reference(reference.kind);
        let typ = Type::Application(Arc::new(constructor), Arc::new(vec![lifetime, element]));
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

        let deref_name = self.get_deref_name();
        let deref_path = self.push_path(
            cst::Path::ident(".*".to_string(), location.clone()),
            Type::ERROR, // overwritten when check_expr re-checks the synthesized call
            location.clone(),
        );
        self.current_extended_context_mut().insert_path_origin(deref_path, Origin::TopLevelDefinition(deref_name));

        let function_type = Type::Function(Arc::new(FunctionType {
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
        self.current_extended_context_mut().copy_expr_metadata(expr, rhs);
        cst::Expr::Reference(cst::Reference { kind, rhs })
    }

    pub(crate) fn type_to_string(&self, typ: &Type) -> String {
        typ.display(&self.bindings, self.current_context(), self.compiler).hiding_environments().to_string()
    }

    /// Check `actual <: expected`, returning `Err(())` on failure without pushing a Diagnostic.
    ///
    /// Returns any new bindings created on success.
    fn try_unify(&self, actual: &Type, expected: &Type) -> Result<TypeBindings, ()> {
        let mut bindings = TypeBindings::default();
        self.subtype(actual, expected, Variance::Covariant, &mut bindings).map(|_| bindings)
    }

    /// Is `a` a subtype of `b`? (iff `variance == Covariant`)
    fn subtype(&self, a: &Type, b: &Type, variance: Variance, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        if variance == Variance::Contravariant {
            return self.subtype(b, a, Variance::Covariant, new_bindings);
        }

        match (a, b) {
            (Type::Variable(a_id), b) => {
                if let Some(a) = self.bindings.get(a_id) {
                    self.subtype(a, b, variance, new_bindings)
                } else if let Some(a) = new_bindings.get(a_id).cloned() {
                    self.subtype(&a, b, variance, new_bindings)
                } else {
                    let b = b.follow_two(&self.bindings, new_bindings);
                    self.try_bind_type_variable(*a_id, b, new_bindings)
                }
            },
            (a, Type::Variable(b_id)) => {
                if let Some(b) = self.bindings.get(b_id) {
                    self.subtype(a, b, variance, new_bindings)
                } else if let Some(b) = new_bindings.get(b_id).cloned() {
                    self.subtype(a, &b, variance, new_bindings)
                } else {
                    let a = a.follow_two(&self.bindings, new_bindings);
                    self.try_bind_type_variable(*b_id, a, new_bindings)
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
                    self.subtype(&a_param.typ, &b_param.typ, variance.flip(), new_bindings)?;
                }

                // Hack: Ability methods carry a `Ptr Unit` env so every ability value has a uniform
                // `(fn_ptr, env_ptr)` size. A bare function (env = NoClosureEnv) is treated as
                // compatible with such a slot: the MIR builder wraps it with a null pointer env.
                let a_env = a_fn.environment.follow_two(&self.bindings, new_bindings);
                let b_env = b_fn.environment.follow_two(&self.bindings, new_bindings);
                let no_env = |t: &Type| matches!(t, Type::Primitive(PrimitiveType::NoClosureEnv));
                let is_ptr_env = |t: &Type| match t {
                    Type::Primitive(PrimitiveType::Pointer) => true,
                    Type::Application(c, _) => {
                        matches!(c.follow_two(&self.bindings, new_bindings), Type::Primitive(PrimitiveType::Pointer))
                    },
                    _ => false,
                };
                let env_skip = (no_env(&a_env) && is_ptr_env(&b_env)) || (no_env(&b_env) && is_ptr_env(&a_env));
                if !env_skip {
                    self.subtype(&a_fn.environment, &b_fn.environment, Variance::Invariant, new_bindings)?;
                }
                self.subtype(&a_fn.return_type, &b_fn.return_type, variance, new_bindings)?;
                self.subtype(&a_fn.effects, &b_fn.effects, variance, new_bindings)
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
                    self.subtype(a_constructor, b_constructor, variance, new_bindings)?;

                    // `mut`/`uniq` elements are invariant
                    let read_only = matches!(b_kind, ReferenceKind::Imm | ReferenceKind::Ref);
                    let element_variance = if !read_only { Variance::Invariant } else { variance };

                    for (index, (a_arg, b_arg)) in a_args.iter().zip(b_args.iter()).enumerate() {
                        let arg_variance = if index == 1 { element_variance } else { Variance::Invariant };
                        self.subtype(a_arg, b_arg, arg_variance, new_bindings)?;
                    }
                    Ok(())
                } else {
                    self.subtype(a_constructor, b_constructor, Variance::Invariant, new_bindings)?;
                    for (a_arg, b_arg) in a_args.iter().zip(b_args.iter()) {
                        self.subtype(a_arg, b_arg, Variance::Invariant, new_bindings)?;
                    }
                    Ok(())
                }
            },
            (Type::Forall(a_generics, a_body), Type::Forall(b_generics, b_body)) => {
                if a_generics.len() != b_generics.len() {
                    return Err(());
                }
                for (a_generic, b_generic) in a_generics.iter().zip(b_generics.iter()) {
                    self.subtype(&a_generic.as_type(), &b_generic.as_type(), Variance::Invariant, new_bindings)?;
                }
                self.subtype(a_body, b_body, Variance::Invariant, new_bindings)
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
            (Type::Effects(..), Type::Effects(..)) => self.row_subtype(a, b, new_bindings),
            (a, b) if a == b => Ok(()),
            _ => Err(()),
        }
    }

    /// Find a non-skipped head-matching candidate that subtypes `target` per `variance`, trying each speculatively.
    fn subtype_matching_effect(
        &self, candidates: &[Type], skip: impl Fn(usize) -> bool, target: &Type, variance: Variance,
        new_bindings: &mut TypeBindings,
    ) -> Result<Option<usize>, ()> {
        for (i, candidate) in candidates.iter().enumerate() {
            if skip(i) || !Self::effect_heads_match(candidate, target) {
                continue;
            }
            let mut trial = new_bindings.clone();
            if self.subtype(candidate, target, variance, &mut trial).is_ok() {
                *new_bindings = trial;
                return Ok(Some(i));
            }
        }
        Ok(None)
    }

    /// Row-subtype two effect rows: is `a`'s actual set of effects permitted by `b`'s expected set?
    fn row_subtype(&self, a: &Type, b: &Type, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let a = self.canonical_effects_row(a, new_bindings);
        let b = self.canonical_effects_row(b, new_bindings);
        let (Type::Effects(a_list, a_tail), Type::Effects(b_list, b_tail)) = (&a, &b) else {
            unreachable!("row_subtype called with non-Effects type");
        };

        let is_error = |tail: &Option<Arc<Type>>| tail.as_deref().is_some_and(Type::is_error);
        if is_error(a_tail) || is_error(b_tail) {
            return Ok(());
        }

        let mut a_leftover = Vec::new();
        let mut b_matched = vec![false; b_list.len()];

        for a_effect in a_list.iter() {
            let matched = self.subtype_matching_effect(
                b_list,
                |i| b_matched[i],
                a_effect,
                Variance::Contravariant,
                new_bindings,
            )?;
            match matched {
                Some(pos) => b_matched[pos] = true,
                None => a_leftover.push(a_effect.clone()),
            }
        }

        if !a_leftover.is_empty() {
            let fresh_tail = self.next_type_variable();
            self.bind_open_tail(b_tail.as_deref(), a_leftover, Some(fresh_tail), true, new_bindings)?;
        }

        let b_unmatched: Vec<Type> =
            b_list.iter().enumerate().filter(|(i, _)| !b_matched[*i]).map(|(_, t)| t.clone()).collect();

        let new_tail = b_tail.as_deref().cloned();
        self.bind_open_tail(a_tail.as_deref(), b_unmatched, new_tail, false, new_bindings)?;

        Ok(())
    }

    fn bind_open_tail(
        &self, tail: Option<&Type>, new_list: Vec<Type>, new_tail: Option<Type>, error_if_not_variable: bool,
        new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        match tail {
            Some(Type::Variable(id)) => {
                let is_self_bind = new_list.is_empty() && matches!(&new_tail, Some(Type::Variable(t)) if t == id);
                if is_self_bind {
                    Ok(())
                } else {
                    self.try_bind_type_variable(*id, Type::effects(new_list, new_tail), new_bindings)
                }
            },
            _ if error_if_not_variable => Err(()),
            _ => Ok(()),
        }
    }

    /// Follow `effects` to the inner [Type::Effects] variant holding the entire effect row.
    fn canonical_effects_row(&self, effects: &Type, new_bindings: &TypeBindings) -> Type {
        match effects.follow_two(&self.bindings, new_bindings) {
            Type::Effects(list, tail) => {
                let list = list.iter().map(|t| t.follow_two(&self.bindings, new_bindings)).collect();
                let tail = tail.as_deref().map(|t| self.canonical_effects_row(t, new_bindings));
                Type::effects(list, tail)
            },
            // A bare variable/generic tail with no accumulated effects yet.
            other => Type::effects(Vec::new(), Some(other)),
        }
    }

    /// True if two effect-row entries share the same effect constructor ignoring type arguments.
    fn effect_heads_match(a: &Type, b: &Type) -> bool {
        let head = |effect: &Type| {
            effect.as_user_defined().copied().or_else(|| effect.as_application()?.0.as_user_defined().copied())
        };
        match (head(a), head(b)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
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
            Type::Primitive(_) | Type::Generic(_) | Type::UserDefined(_) | Type::U32(_) => false,
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
            Type::Effects(list, tail) => {
                list.iter().any(|effect| self.occurs(effect, variable, new_bindings))
                    || tail.as_ref().is_some_and(|tail| self.occurs(tail, variable, new_bindings))
            },
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
        let mut local_kinds = crate::type_inference::types::LocalKinds::default();
        self.from_cst_type_with_local_kinds(typ, allow_implicit_type_vars, allow_implicit_type_vars, &mut local_kinds)
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
        &mut self, typ: &cst::Type, allow_implicit_type_vars: bool, open_effects_by_default: bool,
        local_kinds: &mut LocalKinds,
    ) -> Type {
        let mut next_id = self.next_type_variable_id.get();
        let typ = Type::from_cst_type(
            typ,
            self.current_resolve(),
            self.compiler,
            &mut next_id,
            local_kinds,
            allow_implicit_type_vars,
            open_effects_by_default,
        );
        self.next_type_variable_id.set(next_id);
        typ
    }

    /// Like [Self::from_cst_type] but does not require the converted type to be of kind
    /// [Kind::Type]. Instead, the type's kind is returned alongside it so the caller can
    /// decide how to handle type constructors.
    fn from_cst_type_and_kind(
        &mut self, typ: &cst::Type, allow_implicit_type_vars: bool,
    ) -> (Type, crate::type_inference::kinds::Kind) {
        let mut local_kinds = crate::type_inference::types::LocalKinds::default();
        let mut next_id = self.next_type_variable_id.get();
        let result = Type::from_cst_type_helper(
            typ,
            None,
            self.current_resolve(),
            self.compiler,
            &mut next_id,
            &mut local_kinds,
            allow_implicit_type_vars,
            allow_implicit_type_vars,
        );
        self.next_type_variable_id.set(next_id);
        result
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
                                Some(lifetime) => vec![lifetime.clone(), field_type],
                                None => vec![field_type],
                            };
                            let wrapped = Type::Application(constructor.clone(), Arc::new(wrapped_args));
                            (name, (wrapped, index))
                        })
                        .collect();
                }
                self.get_field_types(&constructor, Some(&arguments))
            },
            Type::UserDefined(origin) => {
                if let Origin::TopLevelDefinition(id) = origin {
                    let body = id.top_level_item.type_body(generic_args, self.compiler);
                    if let TypeBody::Product { fields, .. } = body {
                        let fields = fields.into_iter().enumerate();
                        return fields.map(|(i, (name, typ))| (name, (typ, i as u32))).collect();
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

        let expected = Type::Function(Arc::new(FunctionType {
            parameters: vec![ParameterType::explicit(Type::UNIT)],
            environment: Type::NO_CLOSURE_ENV,
            return_type: Type::UNIT,
            effects: Type::pure(),
        }));

        self.unify(typ, &expected, TypeErrorKind::MainFn, pattern);
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
