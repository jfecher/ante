use std::{collections::BTreeMap, rc::Rc, sync::Arc};

use inc_complete::DbGet;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::Diagnostic,
    incremental::{self, DbHandle, GetItem, Resolve, TypeCheck, TypeCheckSCC},
    iterator_extensions::vecmap,
    lexer::token::{FloatKind, IntegerKind},
    name_resolution::{Origin, ResolutionResult, builtin::Builtin},
    parser::{
        context::TopLevelContext,
        cst::{self, Name, TopLevelItem, TopLevelItemKind},
        ids::{ExprId, NameId, PathId, PatternId, TopLevelId, TopLevelName},
    },
    type_inference::{
        errors::{Locateable, TypeErrorKind},
        fresh_expr::ExtendedTopLevelContext,
        generics::Generic,
        top_level_types::{GeneralizedType, TopLevelType},
        types::{Type, TypeBindings, TypeVariableId},
    },
};

mod cst_traversal;
pub mod dependency_graph;
pub mod errors;
pub mod fresh_expr;
pub mod generics;
mod get_type;
pub mod patterns;
pub mod top_level_types;
pub mod types;

pub use get_type::get_type_impl;

/// Actually type check a statement and its contents.
/// Unlike `get_type_impl`, this always type checks the expressions inside a statement
/// to ensure they type check correctly.
pub fn type_check_impl(context: &TypeCheckSCC, compiler: &DbHandle) -> TypeCheckSCCResult {
    incremental::enter_query();
    let items = TypeChecker::item_contexts(&context.0, compiler);
    let mut checker = TypeChecker::new(&items, compiler);

    let items = vecmap(context.0.iter(), |item_id| {
        incremental::println(format!("Type checking {item_id:?}"));
        checker.start_item(*item_id);

        let item = &checker.item_contexts[item_id].0;
        match &item.kind {
            TopLevelItemKind::Definition(definition) => checker.check_definition(definition),
            TopLevelItemKind::TypeDefinition(type_definition) => checker.check_type_definition(type_definition),
            TopLevelItemKind::TraitDefinition(_) => unreachable!("Traits should be desugared into types by this point"),
            TopLevelItemKind::TraitImpl(_) => unreachable!("Impls should be simplified into definitions by this point"),
            TopLevelItemKind::EffectDefinition(_) => (), // TODO
            TopLevelItemKind::Extern(extern_) => checker.check_extern(extern_),
            TopLevelItemKind::Comptime(comptime) => checker.check_comptime(comptime),
        };

        checker.finish_item()
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
    pub generalized: BTreeMap<NameId, GeneralizedType>,
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
    name_types: BTreeMap<NameId, Type>,
    path_types: BTreeMap<PathId, Type>,
    pattern_types: BTreeMap<PatternId, Type>,
    expr_types: BTreeMap<ExprId, Type>,

    bindings: TypeBindings,

    /// Type inference is the first pass where type variables are introduced.
    /// This field starts from 0 to give each a unique ID within the current inference group.
    next_type_variable_id: u32,

    /// Contains the ItemContext for each item in the TypeChecker's type check group.
    /// Most often, this is just a single item. In the case of mutually recursive type
    /// inference however, it will include every item in the recursive SCC to infer.
    item_contexts: &'local ItemContexts,

    /// The type checker may output new expression, path, or name IDs so we
    /// extend each [TopLevelContext] with these new ids.
    id_contexts: FxHashMap<TopLevelId, ExtendedTopLevelContext>,

    current_item: Option<TopLevelId>,

    /// Types of each top-level item in the current SCC being worked on
    item_types: Rc<FxHashMap<TopLevelName, Type>>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeMaps {
    pub name_types: BTreeMap<NameId, Type>,
    pub path_types: BTreeMap<PathId, Type>,
    pub expr_types: BTreeMap<ExprId, Type>,
    pub pattern_types: BTreeMap<PatternId, Type>,
}

/// Map from each TopLevelId to a tuple of (the item, parse context, resolution context)
type ItemContexts = FxHashMap<TopLevelId, (Arc<TopLevelItem>, Arc<TopLevelContext>, ResolutionResult)>;

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    fn new(item_contexts: &'local ItemContexts, compiler: &'local DbHandle<'inner>) -> Self {
        let id_contexts = item_contexts
            .iter()
            .map(|(id, (_, context, _))| (*id, ExtendedTopLevelContext::new(context.clone())))
            .collect();

        let mut this = Self {
            compiler,
            bindings: Default::default(),
            next_type_variable_id: 0,
            name_types: Default::default(),
            path_types: Default::default(),
            expr_types: Default::default(),
            pattern_types: Default::default(),
            item_types: Default::default(),
            current_item: None,
            item_contexts,
            id_contexts,
        };

        let mut item_types = FxHashMap::default();
        for (item_id, (_, _, resolution)) in item_contexts.iter() {
            for name in resolution.top_level_names.iter() {
                let variable = this.next_type_variable();
                item_types.insert(TopLevelName::new(*item_id, *name), variable);
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
    fn current_context(&self) -> &'local TopLevelContext {
        let item = self.current_item.expect("TypeChecker: Expected current_item to be set");
        &self.item_contexts[&item].1
    }

    fn current_resolve(&self) -> &'local ResolutionResult {
        let item = self.current_item.expect("TypeChecker: Expected current_item to be set");
        &self.item_contexts[&item].2
    }

    fn current_extended_context(&self) -> &ExtendedTopLevelContext {
        let item = self.current_item.expect("TypeChecker: Expected current_item to be set");
        self.id_contexts.get(&item).expect("Expected TopLevelId to be in id_contexts")
    }

    fn current_extended_context_mut(&mut self) -> &mut ExtendedTopLevelContext {
        let item = self.current_item.expect("TypeChecker: Expected current_item to be set");
        self.id_contexts.get_mut(&item).expect("Expected TopLevelId to be in id_contexts")
    }

    fn finish(mut self, items: Vec<TypeMaps>) -> TypeCheckSCCResult {
        let items = self
            .generalize_all()
            .into_iter()
            .zip(items)
            .map(|((id, generalized), maps)| {
                let mut context = self.id_contexts.remove(&id).unwrap();
                let item_context = self.item_contexts.get(&id).unwrap();
                context.extend_from_resolution_result(&item_context.2);
                (id, IndividualTypeCheckResult { maps, generalized, context })
            })
            .collect();

        TypeCheckSCCResult { items, bindings: self.bindings }
    }

    /// Prepare the TypeChecker to type check another item.
    fn start_item(&mut self, item_id: TopLevelId) {
        self.current_item = Some(item_id);

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

    fn next_type_variable(&mut self) -> Type {
        let id = TypeVariableId(self.next_type_variable_id);
        self.next_type_variable_id += 1;
        Type::Variable(id)
    }

    /// Generalize all types in the current SCC.
    /// The returned Vec is in the same order as the SCC.
    fn generalize_all(&mut self) -> BTreeMap<TopLevelId, BTreeMap<NameId, GeneralizedType>> {
        let mut items: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();

        for (name, typ) in self.item_types.clone().iter() {
            self.current_item = Some(name.top_level_item);
            let typ = self.generalize(typ);
            items.entry(name.top_level_item).or_default().insert(name.local_name_id, typ);
        }

        items
    }

    /// Generalize a type, making it generic. Any holes in the type become generic types.
    fn generalize(&mut self, typ: &Type) -> GeneralizedType {
        let free_vars = self.free_vars(&typ);
        let substitutions = free_vars.into_iter().map(|var| (var, Type::Generic(Generic::Inferred(var)))).collect();

        let typ = self.substitute(&typ, &substitutions);
        self.promote_to_top_level_type(&typ).generalize()
    }

    fn substitute(&mut self, typ: &Type, bindings: &TypeBindings) -> Type {
        match self.follow_type(typ) {
            Type::Primitive(_) | Type::Generic(_) | Type::UserDefined(_) => typ.clone(),
            Type::Variable(id) => match bindings.get(id) {
                Some(binding) => binding.clone(),
                None => typ.clone(),
            },
            Type::Function(function) => {
                let function = function.clone();
                let parameters = vecmap(&function.parameters, |param| self.substitute(param, bindings));
                let return_type = self.substitute(&function.return_type, bindings);
                let effects = self.substitute(&function.effects, bindings);
                Type::Function(Arc::new(types::FunctionType { parameters, return_type, effects }))
            },
            Type::Application(constructor, args) => {
                let (constructor, args) = (constructor.clone(), args.clone());
                let constructor = self.substitute(&constructor, bindings);
                let args = vecmap(args.iter(), |arg| self.substitute(arg, bindings));
                Type::Application(Arc::new(constructor), Arc::new(args))
            },
        }
    }

    /// Similar to substitute, but substitutes `Type::Generic` instead of `Type::TypeVariable`
    fn substitute_generics(&mut self, typ: &Type, bindings: &FxHashMap<Generic, Type>) -> Type {
        match self.follow_type(typ) {
            Type::Primitive(_) | Type::Variable(_) | Type::UserDefined(_) => typ.clone(),
            Type::Generic(generic) => match bindings.get(generic) {
                Some(binding) => binding.clone(),
                None => typ.clone(),
            },
            Type::Function(function) => {
                let function = function.clone();
                let parameters = vecmap(&function.parameters, |param| self.substitute_generics(param, bindings));
                let return_type = self.substitute_generics(&function.return_type, bindings);
                let effects = self.substitute_generics(&function.effects, bindings);
                Type::Function(Arc::new(types::FunctionType { parameters, return_type, effects }))
            },
            Type::Application(constructor, args) => {
                let (constructor, args) = (constructor.clone(), args.clone());
                let constructor = self.substitute_generics(&constructor, bindings);
                let args = vecmap(args.iter(), |arg| self.substitute_generics(arg, bindings));
                Type::Application(Arc::new(constructor), Arc::new(args))
            },
        }
    }

    /// Promotes a type to a top-level type.
    /// Panics if the typ contains an unbound type variable.
    fn promote_to_top_level_type(&self, typ: &Type) -> TopLevelType {
        match self.follow_type(&typ) {
            Type::Primitive(primitive) => TopLevelType::Primitive(*primitive),
            Type::Generic(name) => TopLevelType::Generic(*name),
            Type::UserDefined(origin) => TopLevelType::UserDefined(*origin),
            Type::Variable(_) => {
                panic!("promote_to_top_level_type called with type containing an unbound type variable")
            },
            Type::Function(function_type) => {
                let parameters = vecmap(&function_type.parameters, |typ| self.promote_to_top_level_type(typ));
                let return_type = Box::new(self.promote_to_top_level_type(&function_type.return_type));
                TopLevelType::Function { parameters, return_type }
            },
            Type::Application(constructor, args) => {
                let constructor = Box::new(self.promote_to_top_level_type(constructor));
                let args = vecmap(args.iter(), |arg| self.promote_to_top_level_type(arg));
                TopLevelType::Application(constructor, args)
            },
        }
    }

    /// Return the list of unbound type variables within this type
    fn free_vars(&self, typ: &Type) -> Vec<TypeVariableId> {
        fn free_vars_helper(this: &TypeChecker, typ: &Type, free_vars: &mut Vec<TypeVariableId>) {
            match this.follow_type(typ) {
                Type::Primitive(_) | Type::Generic(_) | Type::UserDefined(_) => (),
                Type::Variable(id) => {
                    // The number of free vars is expected to remain too small so we're
                    // not too worried about asymptotic behavior. It is more important we
                    // maintain the ordering of insertion.
                    if !free_vars.contains(id) {
                        free_vars.push(*id);
                    }
                },
                Type::Function(function) => {
                    for parameter in &function.parameters {
                        free_vars_helper(this, parameter, free_vars);
                    }
                    free_vars_helper(this, &function.return_type, free_vars);
                    free_vars_helper(this, &function.effects, free_vars);
                },
                Type::Application(constructor, args) => {
                    free_vars_helper(this, constructor, free_vars);
                    for arg in args.iter() {
                        free_vars_helper(this, arg, free_vars);
                    }
                },
            }
        }

        let mut free_vars = Vec::new();
        free_vars_helper(self, typ, &mut free_vars);
        free_vars
    }

    fn instantiate(&mut self, typ: &GeneralizedType) -> Type {
        let substitutions = typ.generics.iter().map(|generic| (*generic, self.next_type_variable())).collect();
        typ.typ.substitute(&substitutions)
    }

    /// Unifies the two types. Returns false on failure
    fn unify(&mut self, actual: &Type, expected: &Type, kind: TypeErrorKind, locator: impl Locateable) -> bool {
        if self.try_unify(actual, expected).is_err() {
            let actual = self.type_to_string(actual);
            let expected = self.type_to_string(expected);
            let location = locator.locate(self);
            self.compiler.accumulate(Diagnostic::TypeError { actual, expected, kind, location });
            false
        } else {
            true
        }
    }

    fn type_to_string(&self, typ: &Type) -> String {
        typ.to_string(&self.bindings, &self.current_context().names, self.compiler)
    }

    /// Try to unify the given types, returning `Err(())` on error without pushing a Diagnostic.
    ///
    /// Note that any type variable bindings will remain bound.
    fn try_unify(&mut self, actual: &Type, expected: &Type) -> Result<(), ()> {
        if actual == expected {
            return Ok(());
        }

        match (actual, expected) {
            (Type::Variable(actual_id), _) => {
                if let Some(actual) = self.bindings.get(actual_id).cloned() {
                    self.try_unify(&actual, expected)
                } else {
                    self.try_bind_type_variable(*actual_id, actual, expected.clone())
                }
            },
            (_, Type::Variable(expected_id)) => {
                if let Some(expected) = self.bindings.get(expected_id).cloned() {
                    self.try_unify(actual, &expected)
                } else {
                    self.try_bind_type_variable(*expected_id, expected, actual.clone())
                }
            },
            (Type::Primitive(types::PrimitiveType::Error), _) | (_, Type::Primitive(types::PrimitiveType::Error)) => {
                Ok(())
            },
            (Type::Function(actual), Type::Function(expected)) => {
                if actual.parameters.len() != expected.parameters.len() {
                    return Err(());
                }
                for (actual, expected) in actual.parameters.iter().zip(&expected.parameters) {
                    self.try_unify(actual, expected)?;
                }
                self.try_unify(&actual.effects, &expected.effects)?;
                self.try_unify(&actual.return_type, &expected.return_type)
            },
            (
                Type::Application(actual_constructor, actual_args),
                Type::Application(expected_constructor, expected_args),
            ) => {
                if actual_args.len() != expected_args.len() {
                    return Err(());
                }
                self.try_unify(actual_constructor, expected_constructor)?;
                for (actual, expected) in actual_args.iter().zip(expected_args.iter()) {
                    self.try_unify(actual, expected)?;
                }
                Ok(())
            },
            (actual, other) if actual == other => Ok(()),
            _ => Err(()),
        }
    }

    /// Try to bind a type variable, possibly erroring instead if the binding would lead
    /// to a recursive type.
    fn try_bind_type_variable(
        &mut self, id: TypeVariableId, type_variable_type_id: &Type, binding: Type,
    ) -> Result<(), ()> {
        // This should be prevented by the `actual_id == expected_id` check in `unify`
        // Otherwise we need to ensure this case would not issue an `occurs` error.
        assert_ne!(type_variable_type_id, &binding);

        if self.occurs(&binding, id) {
            // Recursive type error
            Err(())
        } else {
            self.bindings.insert(id, binding);
            Ok(())
        }
    }

    /// True if `variable` occurs within `typ`.
    /// Used to prevent the creation of infinitely recursive types when binding type variables.
    fn occurs(&self, typ: &Type, variable: TypeVariableId) -> bool {
        match typ {
            Type::Primitive(_) | Type::Generic(_) | Type::UserDefined(_) => false,
            Type::Variable(candidate_id) => {
                if let Some(binding) = self.bindings.get(candidate_id) {
                    self.occurs(binding, variable)
                } else {
                    *candidate_id == variable
                }
            },
            Type::Function(function_type) => {
                function_type.parameters.iter().any(|param| self.occurs(param, variable))
                    || self.occurs(&function_type.return_type, variable)
                    || self.occurs(&function_type.effects, variable)
            },
            Type::Application(constructor, args) => {
                self.occurs(constructor, variable) || args.iter().any(|arg| self.occurs(arg, variable))
            },
        }
    }

    /// Retrieve a Type then follow all its type variable bindings so that we only return
    /// `Type::Variable` if the type variable is unbound. Note that this may still return
    /// a composite type such as `Type::Application` with bound type variables within.
    fn follow_type<'a>(&'a self, typ: &'a Type) -> &'a Type {
        typ.follow_type(&self.bindings)
    }

    fn convert_origin_to_type(&mut self, origin: Option<Origin>, make_type: impl FnOnce(Origin) -> Type) -> Type {
        match origin {
            Some(Origin::Builtin(builtin)) => {
                match builtin {
                    Builtin::Unit => Type::UNIT,
                    Builtin::Int => Type::ERROR, // TODO: Polymorphic integers
                    Builtin::Char => Type::CHAR,
                    Builtin::Float => Type::ERROR, // TODO: Polymorphic floats
                    Builtin::String => Type::STRING,
                    Builtin::Ptr => Type::POINTER,
                    Builtin::PairType => Type::PAIR,
                    Builtin::PairConstructor => {
                        // TODO: Error
                        Type::ERROR
                    },
                }
            },
            Some(origin) => {
                if !origin.may_be_a_type() {
                    // TODO: Error
                }
                make_type(origin)
            },
            // Assume name resolution has already issued an error for this case
            None => Type::ERROR,
        }
    }

    /// Try to retrieve the types of each field of the given type.
    /// Returns an empty map if unsuccessful.
    ///
    /// The map maps from the field name to a pair of (field type, field index).
    fn get_field_types(&mut self, typ: &Type, generic_args: Option<&[Type]>) -> BTreeMap<Arc<String>, (Type, u32)> {
        match self.follow_type(typ) {
            Type::Application(constructor, arguments) => {
                // TODO: Error if `generic_args` is non-empty
                let constructor = constructor.clone();
                let arguments = arguments.clone();
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
            Type::Primitive(types::PrimitiveType::String) => {
                let mut fields = BTreeMap::default();

                let c_string_type = Type::Application(Arc::new(Type::POINTER), Arc::new(vec![Type::CHAR]));

                // TODO: Hide these and only expose them as unsafe builtins
                fields.insert(Arc::new("c_string".into()), (c_string_type, 0));
                fields.insert(Arc::new("length".into()), (Type::U32, 1));
                fields
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

    /// Convert an ast type to a Type as closely as possible.
    /// This method does not emit any errors and relies on name resolution
    /// to emit errors when resolving types.
    /// Convert the given Origin to a type, issuing an error if the origin is not a type
    fn convert_ast_type(&mut self, typ: &crate::parser::cst::Type) -> Type {
        match typ {
            crate::parser::cst::Type::Integer(kind) => match kind {
                IntegerKind::I8 => Type::I8,
                IntegerKind::I16 => Type::I16,
                IntegerKind::I32 => Type::I32,
                IntegerKind::I64 => Type::I64,
                IntegerKind::Isz => Type::ISZ,
                IntegerKind::U8 => Type::U8,
                IntegerKind::U16 => Type::U16,
                IntegerKind::U32 => Type::U32,
                IntegerKind::U64 => Type::U64,
                IntegerKind::Usz => Type::USZ,
            },
            crate::parser::cst::Type::Float(kind) => match kind {
                FloatKind::F32 => Type::F32,
                FloatKind::F64 => Type::F64,
            },
            crate::parser::cst::Type::String => Type::STRING,
            crate::parser::cst::Type::Char => Type::CHAR,
            crate::parser::cst::Type::Named(path) => {
                // TODO: is `current_resolve` sufficient or do we need the [ExtendedTopLevelContext]?
                let origin = self.current_resolve().path_origins.get(path).copied();
                self.convert_origin_to_type(origin, Type::UserDefined)
            },
            crate::parser::cst::Type::Variable(name) => {
                // TODO: is `current_resolve` sufficient or do we need the [ExtendedTopLevelContext]?
                let origin = self.current_resolve().name_origins.get(name).copied();
                self.convert_origin_to_type(origin, |origin| Type::Generic(Generic::Named(origin)))
            },
            crate::parser::cst::Type::Function(function) => {
                let parameters = vecmap(&function.parameters, |typ| self.convert_ast_type(typ));
                let return_type = self.convert_ast_type(&function.return_type);
                // TODO: Effects
                let effects = Type::UNIT;
                Type::Function(Arc::new(types::FunctionType { parameters, return_type, effects }))
            },
            crate::parser::cst::Type::Error => Type::ERROR,
            crate::parser::cst::Type::Unit => Type::UNIT,
            crate::parser::cst::Type::Pair => Type::PAIR,
            crate::parser::cst::Type::Application(f, args) => {
                let f = self.convert_ast_type(f);
                let args = vecmap(args, |typ| self.convert_ast_type(typ));
                Type::Application(Arc::new(f), Arc::new(args))
            },
            crate::parser::cst::Type::Reference(mutability, sharedness) => {
                Type::Primitive(types::PrimitiveType::Reference(*mutability, *sharedness))
            },
        }
    }
}

pub enum TypeBody {
    Product { type_name: Name, fields: Vec<(Name, Type)> },
    Sum(Vec<(Name, Vec<Type>)>),
}

impl TopLevelId {
    /// Returns the body of this user-defined type (the part after the `=` when declared).
    /// The given [TopLevelId] should refer to a [TypeDefinition] or something which desugars to
    /// one.
    ///
    /// If specified, `arguments` will be used to substitute any generics of the type.
    /// Panics if the arguments are specified and differ in length to the type's generics.
    ///
    /// Note that if `arguments` are not provided, the type will be instantiated and thus
    /// any fields may refer to type type variables that have not been tracked.
    ///
    /// - For a struct: returns each field name & type
    /// - For a union: returns each variant with its name and arguments
    ///
    /// TODO: This function is called somewhat often but is a lot of work to redo each time.
    pub fn type_body<Db>(self, arguments: Option<&[Type]>, compiler: &Db) -> TypeBody
    where
        Db: DbGet<TypeCheck> + DbGet<GetItem>,
    {
        let result = TypeCheck(self).get(compiler);
        let (item, item_context) = GetItem(self).get(compiler);

        let TopLevelItemKind::TypeDefinition(type_definition) = &item.kind else {
            panic!("type_body: passed type_id is not a type!")
        };

        match &type_definition.body {
            cst::TypeDefinitionBody::Struct(fields) => {
                // This'd be easier with an explicit type data field
                let constructor_type = &result.result.generalized[&type_definition.name];
                let constructor = maybe_apply_type(constructor_type, arguments);
                let field_types = constructor.function_parameter_types();
                let fields = vecmap(fields.iter().zip(field_types), |((field_name, _), typ)| {
                    (item_context.names[*field_name].clone(), typ.clone())
                });

                let type_name = item_context.names[type_definition.name].clone();
                TypeBody::Product { type_name, fields }
            },
            cst::TypeDefinitionBody::Enum(variants) => {
                let variants = vecmap(variants, |(name, _)| {
                    let constructor_type = &result.result.generalized[name];
                    let constructor = maybe_apply_type(constructor_type, arguments);
                    let fields = constructor.function_parameter_types();
                    (item_context.names[*name].clone(), fields.to_vec())
                });
                TypeBody::Sum(variants)
            },
            // TODO: Type aliases
            cst::TypeDefinitionBody::Alias(_) | cst::TypeDefinitionBody::Error => {
                // Just make some filler value - ideally we should return an error flag here
                // to prevent future errors
                let type_name = item_context.names[type_definition.name].clone();
                TypeBody::Product { type_name, fields: Vec::new() }
            },
        }
    }
}

fn maybe_apply_type(typ: &GeneralizedType, args: Option<&[Type]>) -> Type {
    match args {
        Some(args) => typ.apply_type(args),
        None => {
            // This should be an error if `!typ.generics.is_empty()`
            let args = vecmap(&typ.generics, |_| Type::ERROR);
            typ.apply_type(&args)
        },
    }
}

/// Returns each argument of the given function type.
/// If the given type is not a function, an empty Vec is returned.
impl Type {
    fn function_parameter_types(&self) -> &[Type] {
        match self {
            Type::Function(function) => &function.parameters,
            _ => &[],
        }
    }
}

impl GeneralizedType {
    /// Apply a GeneralizedType to the given arguments. The given [Type]s of the arguments should
    /// be [Type]s in the current context, and the returned [Type] will be in the current
    /// context as well.
    ///
    /// Panics if `arguments.len() != self.generics.len()`
    fn apply_type(&self, arguments: &[Type]) -> Type {
        assert_eq!(arguments.len(), self.generics.len());
        let substitutions =
            self.generics.iter().zip(arguments).map(|(generic, argument)| (*generic, argument.clone())).collect();
        self.typ.substitute(&substitutions)
    }
}
