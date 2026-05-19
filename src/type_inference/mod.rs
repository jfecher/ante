use std::{cell::Cell, collections::BTreeMap, rc::Rc, sync::Arc};

use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::Diagnostic,
    incremental::{
        self, DbHandle, ExportedDefinitions, ExportedTypes, GetItem, Resolve, TargetPointerSize, TypeCheckSCC,
    },
    iterator_extensions::mapvec,
    lexer::token::IntegerKind,
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
        types::{FunctionType, ParameterType, PrimitiveType, Type, TypeBindings, TypeVariableId},
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
mod type_body;
mod type_definitions;
pub mod types;

pub use get_type::get_type_impl;
pub use type_body::TypeBody;

/// Actually type check a statement and its contents.
/// Unlike `get_type_impl`, this always type checks the expressions inside a statement
/// to ensure they type check correctly.
pub fn type_check_impl(context: &TypeCheckSCC, compiler: &DbHandle) -> TypeCheckSCCResult {
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
            TopLevelItemKind::AbilityDefinition(_) => {
                unreachable!("Abilities should be desugared into types by this point")
            },
            TopLevelItemKind::AbilityImpl(_) => {
                unreachable!("AbilityImpls should be desugared into definitions by this point")
            },
            TopLevelItemKind::Comptime(comptime) => checker.check_comptime(comptime),
        };

        checker.pop_implicits_scope();
        (*item_id, checker.finish_item())
    });

    incremental::exit_query();
    checker.finish(items)
}

/// A `TypeCheckSCCResult` holds the `IndividualTypeCheckResult` of every item in
/// the SCC for a particular TopLevelId
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeCheckSCCResult {
    pub items: BTreeMap<TopLevelId, IndividualTypeCheckResult>,
    pub bindings: TypeBindings,
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

    /// Cached TopLevelName for the Prelude's `Copy` type, lazily resolved on first use.
    copy_type_name: Option<TopLevelName>,

    /// Names defined with `var` or as mutable parameters. Used by closure capture analysis
    /// to wrap mutable captures in a reference type so the closure shares the outer scope's storage.
    mutable_definitions: FxHashSet<NameId>,
}

/// Map from each TopLevelId to a tuple of (the item, parse context, resolution context)
type ItemContexts = FxHashMap<TopLevelId, (Arc<TopLevelItem>, Arc<DesugarContext>, ResolutionResult)>;

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
            item_contexts,
            id_contexts,
            implicits: Vec::new(),
            coercion_wrapper_exprs: Default::default(),
            string_type: None,
            deref_name: None,
            move_tracker: Default::default(),
            suppress_move_check: false,
            suppress_move_record: false,
            copy_type_name: None,
            mutable_definitions: Default::default(),
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
        &self.item_contexts[&item].2
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
        let (top_level_name, _kind) =
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
                context.extend_from_resolution_result(&item_context.2);
                (id, IndividualTypeCheckResult { maps, generalized, context })
            })
            .collect();

        TypeCheckSCCResult { items, bindings: self.bindings }
    }

    /// Check if the integer fits in the given kind, error if not
    fn check_int_fits(&self, value: u64, kind: IntegerKind, locator: impl Locateable) {
        let ptr_size = TargetPointerSize.get(self.compiler);
        let bit_size = 8 * kind.size_in_bytes(ptr_size);
        if bit_size == 64 {
            return;
        }

        // TODO: Change `value` repr from u64 to a type that fits negatives
        // so we can give more accurate ranges. As-is, u64::MAX fits into i64.
        if value > 2u64.pow(bit_size) - 1 {
            let location = locator.locate(self);
            self.compiler.accumulate(Diagnostic::IntegerTooLarge { value, kind, location });
        }
    }

    /// Prepare the TypeChecker to type check another item.
    fn start_item(&mut self, item_id: TopLevelId) {
        self.current_item = Some(item_id);
        self.move_tracker = Default::default();

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
            let typ = typ.generalize(&self.bindings);
            items.entry(name.top_level_item).or_default().insert(name.local_name_id, typ);
        }

        items
    }

    /// Unifies the two types. Returns false on failure
    fn unify(&mut self, actual: &Type, expected: &Type, kind: TypeErrorKind, locator: impl Locateable) -> bool {
        if let Ok(new_bindings) = self.try_unify(actual, expected) {
            self.bindings.extend(new_bindings);
            true
        } else {
            let actual = self.type_to_string(actual);
            let expected = self.type_to_string(expected);
            let location = locator.locate(self);
            self.compiler.accumulate(Diagnostic::TypeError { actual, expected, kind, location });
            false
        }
    }
}

/// Describes what a [`TypeChecker::try_coercion`] call rewrote, if anything.
pub(super) enum CoercionOutcome {
    /// No coercion was applied.
    None,
    /// The expression at the given `expr` was replaced; the caller should re-check it.
    ReplacedExpr,
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
            (Type::Function(actual_fn), Type::Function(expected_fn))
                if actual_fn.parameters.len() != expected_fn.parameters.len() =>
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
                // can be found.
                if let Type::Application(constructor, args) = &actual {
                    if args.len() == 1
                        && matches!(self.follow_type(constructor), Type::Primitive(PrimitiveType::Reference(_)))
                        && !matches!(&expected, Type::Variable(_))
                        && expected.reference_element(&self.bindings).is_none()
                    {
                        let arg = args[0].clone();
                        let expected = expected.clone();
                        if let Ok(bindings) = self.try_unify(&arg, &expected) {
                            self.bindings.extend(bindings);
                            let new_expr = self.auto_deref_coercion(expr, expected);
                            self.current_extended_context_mut().insert_expr(expr, new_expr);
                            return CoercionOutcome::ReplacedExpr;
                        }
                    }
                }
                // Auto-ref: coerce `t` to `ref t` or `imm t` by wrapping `expr` in a reference
                // expression. Only fires for `Ref`/`Imm` kinds; `Mut`/`Uniq` must be written
                // explicitly because of their aliasing/affine semantics. Skip when `actual`
                // is itself a reference (subtyping through `try_unify` handles those cases).
                if let Type::Application(expected_ctor, expected_args) = &expected {
                    if expected_args.len() == 1
                        && !matches!(&actual, Type::Variable(_))
                        && actual.reference_element(&self.bindings).is_none()
                    {
                        let kind = match self.follow_type(expected_ctor) {
                            Type::Primitive(PrimitiveType::Reference(kind)) => Some(*kind),
                            _ => None,
                        };
                        if let Some(kind) = kind {
                            if matches!(kind, ReferenceKind::Ref | ReferenceKind::Imm) {
                                let inner = expected_args[0].clone();
                                let actual = actual.clone();
                                if let Ok(bindings) = self.try_unify(&actual, &inner) {
                                    self.bindings.extend(bindings);
                                    let new_expr = self.auto_ref_coercion(expr, kind, actual);
                                    self.current_extended_context_mut().insert_expr(expr, new_expr);
                                    return CoercionOutcome::ReplacedExpr;
                                }
                            }
                        }
                    }
                }
                CoercionOutcome::None
            },
        }
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
        cst::Expr::Reference(cst::Reference { kind, rhs })
    }

    pub(crate) fn type_to_string(&self, typ: &Type) -> String {
        typ.to_string(&self.bindings, self.current_context(), self.compiler)
    }

    /// Try to unify the given types, returning `Err(())` on error without pushing a Diagnostic.
    ///
    /// Returns any new bindings created on success
    fn try_unify(&self, actual: &Type, expected: &Type) -> Result<TypeBindings, ()> {
        let mut bindings = TypeBindings::default();
        self.try_unify_with_bindings(actual, expected, &mut bindings).map(|_| bindings)
    }

    /// Same as [Self::try_unify] but carries the new type bindings as an argument instead of
    /// a return value.
    fn try_unify_with_bindings(
        &self, actual: &Type, expected: &Type, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        match (actual, expected) {
            (Type::Variable(actual_id), expected) => {
                if let Some(actual) = self.bindings.get(actual_id).cloned() {
                    self.try_unify_with_bindings(&actual, &expected, new_bindings)
                } else if let Some(actual) = new_bindings.get(actual_id).cloned() {
                    self.try_unify_with_bindings(&actual, &expected, new_bindings)
                } else {
                    let expected = expected.follow_two(&self.bindings, new_bindings);
                    self.try_bind_type_variable(*actual_id, expected, new_bindings)
                }
            },
            (actual, Type::Variable(expected_id)) => {
                if let Some(expected) = self.bindings.get(expected_id).cloned() {
                    self.try_unify_with_bindings(actual, &expected, new_bindings)
                } else if let Some(expected) = new_bindings.get(expected_id).cloned() {
                    self.try_unify_with_bindings(actual, &expected, new_bindings)
                } else {
                    let actual = actual.follow_two(&self.bindings, new_bindings);
                    self.try_bind_type_variable(*expected_id, actual, new_bindings)
                }
            },
            // The bottom type should unify with any expected type
            // FIXME: This is unsound since we don't do a true subtyping test. E.g. we don't
            // track variance on any types, even function parameters/returns.
            (Type::Primitive(PrimitiveType::Never), _) => Ok(()),

            // And the error type should unify with everything to prevent future errors
            (Type::Primitive(PrimitiveType::Error), _) | (_, Type::Primitive(PrimitiveType::Error)) => Ok(()),
            (Type::Function(actual), Type::Function(expected)) => {
                if actual.parameters.len() != expected.parameters.len() {
                    return Err(());
                }

                for (actual, expected) in actual.parameters.iter().zip(expected.parameters.iter()) {
                    self.try_unify_with_bindings(&actual.typ, &expected.typ, new_bindings)?;
                }

                // Ability methods carry a `Ptr Unit` env so every ability value has a uniform
                // `(fn_ptr, env_ptr)` size. A bare function (env = NoClosureEnv) is treated as
                // compatible with such a slot: the MIR builder wraps it with a null pointer env.
                let actual_env = actual.environment.follow_two(&self.bindings, new_bindings);
                let expected_env = expected.environment.follow_two(&self.bindings, new_bindings);
                let no_env = |t: &Type| matches!(t, Type::Primitive(PrimitiveType::NoClosureEnv));
                let is_ptr_env = |t: &Type| match t {
                    Type::Primitive(PrimitiveType::Pointer) => true,
                    Type::Application(c, _) => {
                        matches!(c.follow_two(&self.bindings, new_bindings), Type::Primitive(PrimitiveType::Pointer))
                    },
                    _ => false,
                };
                let env_skip = (no_env(&actual_env) && is_ptr_env(&expected_env))
                    || (no_env(&expected_env) && is_ptr_env(&actual_env));
                if !env_skip {
                    self.try_unify_with_bindings(&actual.environment, &expected.environment, new_bindings)?;
                }
                self.try_unify_with_bindings(&actual.return_type, &expected.return_type, new_bindings)
            },
            (
                Type::Application(actual_constructor, actual_args),
                Type::Application(expected_constructor, expected_args),
            ) => {
                if actual_args.len() != expected_args.len() {
                    return Err(());
                }
                self.try_unify_with_bindings(actual_constructor, expected_constructor, new_bindings)?;
                for (actual, expected) in actual_args.iter().zip(expected_args.iter()) {
                    self.try_unify_with_bindings(actual, expected, new_bindings)?;
                }
                Ok(())
            },
            (Type::Forall(actual_generics, actual), Type::Forall(expected_generics, expected)) => {
                if actual_generics.len() != expected_generics.len() {
                    return Err(());
                }
                for (actual, expected) in actual_generics.iter().zip(expected_generics.iter()) {
                    self.try_unify_with_bindings(&actual.as_type(), &expected.as_type(), new_bindings)?;
                }
                self.try_unify_with_bindings(actual, expected, new_bindings)
            },
            (
                Type::Primitive(PrimitiveType::Reference(actual)),
                Type::Primitive(PrimitiveType::Reference(expected)),
            ) => {
                // Allow coercions between reference kinds: any ref type coerces to `ref`,
                // and `uniq` also coerces to `mut`.
                match (actual, expected) {
                    (_, ReferenceKind::Ref) => Ok(()),
                    (ReferenceKind::Uniq, ReferenceKind::Mut) => Ok(()),
                    (actual, expected) if actual == expected => Ok(()),
                    _ => Err(()),
                }
            },
            (actual, other) if actual == other => Ok(()),
            _ => Err(()),
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
        } else {
            new_bindings.insert(id, binding);
            Ok(())
        }
    }

    /// True if `variable` occurs within `typ`.
    /// Used to prevent the creation of infinitely recursive types when binding type variables.
    fn occurs(&self, typ: &Type, variable: TypeVariableId, new_bindings: &TypeBindings) -> bool {
        match typ {
            Type::Primitive(_) | Type::Generic(_) | Type::UserDefined(_) => false,
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
            },
            Type::Application(constructor, args) => {
                self.occurs(constructor, variable, new_bindings)
                    || args.iter().any(|arg| self.occurs(arg, variable, new_bindings))
            },
            Type::Forall(_, typ) => self.occurs(typ, variable, new_bindings),
            Type::Tuple(elements) => elements.iter().any(|element| self.occurs(element, variable, new_bindings)),
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
        let mut next_id = self.next_type_variable_id.get();
        let typ =
            Type::from_cst_type(typ, self.current_resolve(), self.compiler, &mut next_id, allow_implicit_type_vars);
        self.next_type_variable_id.set(next_id);
        typ
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
                if matches!(
                    self.follow_type(&constructor),
                    Type::Primitive(PrimitiveType::Reference(_)) | Type::Primitive(PrimitiveType::Pointer),
                ) {
                    let inner = arguments[0].clone();
                    let inner_fields = self.get_field_types(&inner, None);
                    return inner_fields
                        .into_iter()
                        .map(|(name, (field_type, index))| {
                            let wrapped = Type::Application(constructor.clone(), Arc::new(vec![field_type]));
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
                substitutions.insert(Generic::Named(Origin::Local(*generic)), replacement.clone());
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
