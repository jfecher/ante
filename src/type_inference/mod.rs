use std::{collections::BTreeMap, rc::Rc, sync::Arc};

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use types::{Type, TypeBindings};

use crate::{
    diagnostics::Diagnostic,
    incremental::{self, DbHandle, GetItem, Resolve, TypeCheckSCC},
    iterator_extensions::vecmap,
    lexer::token::{FloatKind, IntegerKind},
    name_resolution::{builtin::Builtin, Origin, ResolutionResult},
    parser::{
        context::TopLevelContext,
        cst::{self, TopLevelItem, TopLevelItemKind, TypeDefinitionBody},
        ids::{ExprId, NameId, PathId, TopLevelId, TopLevelName},
    },
    type_inference::{
        errors::{Locateable, TypeErrorKind},
        generics::Generic,
        type_context::TypeContext,
        type_id::TypeId,
        types::{GeneralizedType, TopLevelType, TypeVariableId},
    },
};

mod cst_traversal;
pub mod dependency_graph;
pub mod errors;
mod generics;
mod get_type;
pub mod patterns;
pub mod type_context;
pub mod type_id;
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
        checker.current_item = Some(*item_id);

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
    pub types: TypeContext,
    pub bindings: TypeBindings,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndividualTypeCheckResult {
    #[serde(flatten)]
    pub maps: TypeMaps,

    /// One or more names may be externally visible outside this top-level item.
    /// Each of these names will be generalized and placed in this map.
    /// Ex: in `foo = (bar = 1; bar + 2)` only `foo: I32` will be generalized,
    /// but in `a, b = 1, 2`, both `a` and `b` will be.
    /// Ex2: in `type Foo = | A | B`, `A` and `B` will both be generalized, and
    /// there is no need to generalize `Foo` itself.
    pub generalized: BTreeMap<NameId, GeneralizedType>,
}

struct TypeChecker<'local, 'inner> {
    compiler: &'local DbHandle<'inner>,
    types: TypeContext,
    name_types: BTreeMap<NameId, TypeId>,
    path_types: BTreeMap<PathId, TypeId>,
    expr_types: BTreeMap<ExprId, TypeId>,
    bindings: TypeBindings,
    next_id: u32,
    item_contexts: &'local ItemContexts,
    current_item: Option<TopLevelId>,

    /// The TypeChecker also resolves any paths with Origin::TypeResolution to
    /// a more specific origin (a union variant) if possible.
    path_origins: BTreeMap<PathId, Origin>,

    /// Types of each top-level item in the current SCC being worked on
    item_types: Rc<FxHashMap<TopLevelName, TypeId>>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeMaps {
    pub name_types: BTreeMap<NameId, TypeId>,
    pub path_types: BTreeMap<PathId, TypeId>,
    pub expr_types: BTreeMap<ExprId, TypeId>,
    pub path_origins: BTreeMap<PathId, Origin>,
}

/// Map from each TopLevelId to a tuple of (the item, parse context, resolution context)
type ItemContexts = FxHashMap<TopLevelId, (Arc<TopLevelItem>, Arc<TopLevelContext>, ResolutionResult)>;

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    fn new(item_contexts: &'local ItemContexts, compiler: &'local DbHandle<'inner>) -> Self {
        let mut this = Self {
            compiler,
            types: TypeContext::new(),
            bindings: Default::default(),
            next_id: 0,
            name_types: Default::default(),
            path_types: Default::default(),
            expr_types: Default::default(),
            item_types: Default::default(),
            path_origins: Default::default(),
            current_item: None,
            item_contexts,
        };

        let mut item_types = FxHashMap::default();
        for (item_id, (_, _, resolution)) in item_contexts.iter() {
            for name in resolution.top_level_names.iter() {
                let variable = this.next_type_variable();
                item_types.insert(TopLevelName::named(*item_id, *name), variable);
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

    fn current_context(&self) -> &'local TopLevelContext {
        let item = self.current_item.expect("TypeChecker: Expected current_item to be set");
        &self.item_contexts[&item].1
    }

    fn current_resolve(&self) -> &'local ResolutionResult {
        let item = self.current_item.expect("TypeChecker: Expected current_item to be set");
        &self.item_contexts[&item].2
    }

    fn finish(mut self, items: Vec<TypeMaps>) -> TypeCheckSCCResult {
        let items = self
            .generalize_all()
            .into_iter()
            .zip(items)
            .map(|((id, generalized), maps)| (id, IndividualTypeCheckResult { maps, generalized }))
            .collect();

        TypeCheckSCCResult { items, types: self.types, bindings: self.bindings }
    }

    /// Finishes the current item, adding all bindings to the relevant entry in
    /// `self.finished_items`, clearing them out in preparation for resolving the next item.
    fn finish_item(&mut self) -> TypeMaps {
        self.current_item = None;
        TypeMaps {
            name_types: std::mem::take(&mut self.name_types),
            path_types: std::mem::take(&mut self.path_types),
            expr_types: std::mem::take(&mut self.expr_types),
            path_origins: std::mem::take(&mut self.path_origins),
        }
    }

    fn next_type_variable(&mut self) -> TypeId {
        let id = TypeVariableId(self.next_id);
        self.next_id += 1;
        self.types.get_or_insert_type(Type::Variable(id))
    }

    /// Generalize all types in the current SCC.
    /// The returned Vec is in the same order as the SCC.
    fn generalize_all(&mut self) -> BTreeMap<TopLevelId, BTreeMap<NameId, GeneralizedType>> {
        let mut items: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();

        for (name, typ) in self.item_types.clone().iter() {
            self.current_item = Some(name.top_level_item);
            let typ = self.generalize(*typ);
            items.entry(name.top_level_item).or_default().insert(name.local_name_id, typ);
        }

        items
    }

    /// Generalize a type, making it generic. Any holes in the type become generic types.
    fn generalize(&mut self, typ: TypeId) -> GeneralizedType {
        let free_vars = self.free_vars(typ);
        let substitutions = free_vars
            .into_iter()
            .map(|var| (var, self.types.get_or_insert_type(Type::Generic(Generic::Inferred(var)))))
            .collect();

        let typ = self.substitute(typ, &substitutions);
        self.promote_to_top_level_type(typ).generalize()
    }

    fn substitute(&mut self, typ: TypeId, bindings: &TypeBindings) -> TypeId {
        match self.follow_type(typ) {
            Type::Primitive(_) | Type::Generic(_) | Type::Reference(..) | Type::UserDefined(_) => typ,
            Type::Variable(id) => match bindings.get(id) {
                Some(binding) => *binding,
                None => typ,
            },
            Type::Function(function) => {
                let function = function.clone();
                let parameters = vecmap(&function.parameters, |param| self.substitute(*param, bindings));
                let return_type = self.substitute(function.return_type, bindings);
                let effects = self.substitute(function.effects, bindings);
                let function = Type::Function(types::FunctionType { parameters, return_type, effects });
                self.types.get_or_insert_type(function)
            },
            Type::Application(constructor, args) => {
                let (constructor, args) = (*constructor, args.clone());
                let constructor = self.substitute(constructor, bindings);
                let args = vecmap(args, |arg| self.substitute(arg, bindings));
                self.types.get_or_insert_type(Type::Application(constructor, args))
            },
        }
    }

    /// Similar to substitute, but substitutes `Type::Generic` instead of `Type::TypeVariable`
    fn substitute_generics(&mut self, typ: TypeId, bindings: &FxHashMap<Generic, TypeId>) -> TypeId {
        match self.follow_type(typ) {
            Type::Primitive(_) | Type::Variable(_) | Type::Reference(..) | Type::UserDefined(_) => typ,
            Type::Generic(generic) => match bindings.get(generic) {
                Some(binding) => *binding,
                None => typ,
            },
            Type::Function(function) => {
                let function = function.clone();
                let parameters = vecmap(&function.parameters, |param| self.substitute_generics(*param, bindings));
                let return_type = self.substitute_generics(function.return_type, bindings);
                let effects = self.substitute_generics(function.effects, bindings);
                let function = Type::Function(types::FunctionType { parameters, return_type, effects });
                self.types.get_or_insert_type(function)
            },
            Type::Application(constructor, args) => {
                let (constructor, args) = (*constructor, args.clone());
                let constructor = self.substitute_generics(constructor, bindings);
                let args = vecmap(args, |arg| self.substitute_generics(arg, bindings));
                self.types.get_or_insert_type(Type::Application(constructor, args))
            },
        }
    }

    /// Promotes a type to a top-level type.
    /// Panics if the typ contains an unbound type variable.
    fn promote_to_top_level_type(&self, typ: TypeId) -> TopLevelType {
        match self.follow_type(typ) {
            Type::Primitive(primitive) => TopLevelType::Primitive(*primitive),
            Type::Generic(name) => TopLevelType::Generic(*name),
            Type::UserDefined(origin) => TopLevelType::UserDefined(*origin),
            Type::Variable(_) => {
                panic!("promote_to_top_level_type called with type containing an unbound type variable")
            },
            Type::Function(function_type) => {
                let parameters = vecmap(&function_type.parameters, |typ| self.promote_to_top_level_type(*typ));
                let return_type = Box::new(self.promote_to_top_level_type(function_type.return_type));
                TopLevelType::Function { parameters, return_type }
            },
            Type::Application(constructor, args) => {
                let constructor = Box::new(self.promote_to_top_level_type(*constructor));
                let args = vecmap(args, |arg| self.promote_to_top_level_type(*arg));
                TopLevelType::TypeApplication(constructor, args)
            },
            Type::Reference(..) => {
                todo!("convert Type::Reference to TopLevelType")
            },
        }
    }

    /// Return the list of unbound type variables within this type
    fn free_vars(&self, typ: TypeId) -> Vec<TypeVariableId> {
        fn free_vars_helper(this: &TypeChecker, typ: TypeId, free_vars: &mut Vec<TypeVariableId>) {
            match this.follow_type(typ) {
                Type::Primitive(_) | Type::Reference(..) | Type::Generic(_) | Type::UserDefined(_) => (),
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
                        free_vars_helper(this, *parameter, free_vars);
                    }
                    free_vars_helper(this, function.return_type, free_vars);
                    free_vars_helper(this, function.effects, free_vars);
                },
                Type::Application(constructor, args) => {
                    free_vars_helper(this, *constructor, free_vars);
                    for arg in args {
                        free_vars_helper(this, *arg, free_vars);
                    }
                },
            }
        }

        let mut free_vars = Vec::new();
        free_vars_helper(self, typ, &mut free_vars);
        free_vars
    }

    fn instantiate(&mut self, typ: &GeneralizedType) -> TypeId {
        let substitutions = typ.generics.iter().map(|generic| (*generic, self.next_type_variable())).collect();
        typ.typ.substitute(&mut self.types, &substitutions)
    }

    /// Unifies the two types. Returns false on failure
    fn unify(&mut self, actual_id: TypeId, expected_id: TypeId, kind: TypeErrorKind, locator: impl Locateable) -> bool {
        if self.try_unify(actual_id, expected_id).is_err() {
            let actual = self.type_to_string(actual_id);
            let expected = self.type_to_string(expected_id);
            let location = locator.locate(self);
            self.compiler.accumulate(Diagnostic::TypeError { actual, expected, kind, location });
            false
        } else {
            true
        }
    }

    fn type_to_string(&self, typ: TypeId) -> String {
        typ.to_string(&self.types, &self.bindings, &self.current_context().names, self.compiler)
    }

    /// Try to unify the given types, returning `Err(())` on error without pushing a Diagnostic.
    ///
    /// Note that any type variable bindings will remain bound.
    fn try_unify(&mut self, actual_id: TypeId, expected_id: TypeId) -> Result<(), ()> {
        if actual_id == expected_id {
            return Ok(());
        }

        match (self.types.get_type(actual_id), self.types.get_type(expected_id)) {
            (Type::Variable(actual), _) => {
                if let Some(actual) = self.bindings.get(actual) {
                    self.try_unify(*actual, expected_id)
                } else {
                    self.try_bind_type_variable(*actual, actual_id, expected_id)
                }
            },
            (_, Type::Variable(expected)) => {
                if let Some(expected) = self.bindings.get(expected) {
                    self.try_unify(actual_id, *expected)
                } else {
                    self.try_bind_type_variable(*expected, expected_id, actual_id)
                }
            },
            (Type::Primitive(types::PrimitiveType::Error), _) | (_, Type::Primitive(types::PrimitiveType::Error)) => {
                Ok(())
            },
            (Type::Function(actual), Type::Function(expected)) => {
                if actual.parameters.len() != expected.parameters.len() {
                    return Err(());
                }
                let actual = actual.clone();
                let expected = expected.clone();
                for (actual, expected) in actual.parameters.into_iter().zip(expected.parameters) {
                    self.try_unify(actual, expected)?;
                }
                self.try_unify(actual.effects, expected.effects)?;
                self.try_unify(actual.return_type, expected.return_type)
            },
            (
                Type::Application(actual_constructor, actual_args),
                Type::Application(expected_constructor, expected_args),
            ) => {
                if actual_args.len() != expected_args.len() {
                    return Err(());
                }
                let actual_args = actual_args.clone();
                let expected_args = expected_args.clone();
                self.try_unify(*actual_constructor, *expected_constructor)?;
                for (actual, expected) in actual_args.into_iter().zip(expected_args) {
                    self.try_unify(actual, expected)?;
                }
                Ok(())
            },
            (
                Type::Reference(actual_mutability, actual_sharedness),
                Type::Reference(expected_mutability, expected_sharedness),
            ) => {
                if actual_mutability == expected_mutability && actual_sharedness == expected_sharedness {
                    Ok(())
                } else {
                    Err(())
                }
            },
            (actual, other) if actual == other => Ok(()),
            _ => Err(()),
        }
    }

    /// Try to bind a type variable, possibly erroring instead if the binding would lead
    /// to a recursive type.
    fn try_bind_type_variable(
        &mut self, id: TypeVariableId, type_variable_type_id: TypeId, binding: TypeId,
    ) -> Result<(), ()> {
        // This should be prevented by the `actual_id == expected_id` check in `unify`
        // Otherwise we need to ensure this case would not issue an `occurs` error.
        assert_ne!(type_variable_type_id, binding);

        if self.occurs(binding, id) {
            // Recursive type error
            Err(())
        } else {
            self.bindings.insert(id, binding);
            Ok(())
        }
    }

    /// True if `variable` occurs within `typ`.
    /// Used to prevent the creation of infinitely recursive types when binding type variables.
    fn occurs(&self, typ: TypeId, variable: TypeVariableId) -> bool {
        match self.types.get_type(typ) {
            Type::Primitive(_) | Type::Reference(..) | Type::Generic(_) | Type::UserDefined(_) => false,
            Type::Variable(candidate_id) => {
                if let Some(binding) = self.bindings.get(candidate_id) {
                    self.occurs(*binding, variable)
                } else {
                    *candidate_id == variable
                }
            },
            Type::Function(function_type) => {
                function_type.parameters.iter().any(|param| self.occurs(*param, variable))
                    || self.occurs(function_type.return_type, variable)
                    || self.occurs(function_type.effects, variable)
            },
            Type::Application(constructor, args) => {
                self.occurs(*constructor, variable) || args.iter().any(|arg| self.occurs(*arg, variable))
            },
        }
    }

    /// Retrieve a Type then follow all its type variable bindings so that we only return
    /// `Type::Variable` if the type variable is unbound. Note that this may still return
    /// a composite type such as `Type::Application` with bound type variables within.
    fn follow_type(&self, id: TypeId) -> &Type {
        match self.types.get_type(id) {
            typ @ Type::Variable(id) => match self.bindings.get(&id) {
                Some(binding) => self.follow_type(*binding),
                None => typ,
            },
            other => other,
        }
    }

    /// Convert an ast type to a TypeId as closely as possible.
    /// This method does not emit any errors and relies on name resolution
    /// to emit errors when resolving types.
    pub fn convert_ast_type(&mut self, typ: &crate::parser::cst::Type) -> TypeId {
        let resolve = self.current_resolve();
        self.convert_foreign_type(typ, resolve)
    }

    /// Convert the given Origin to a type, issuing an error if the origin is not a type
    fn convert_origin_to_type(&mut self, origin: Option<Origin>, make_type: impl FnOnce(Origin) -> Type) -> TypeId {
        match origin {
            Some(Origin::Builtin(builtin)) => {
                match builtin {
                    Builtin::Unit => TypeId::UNIT,
                    Builtin::Int => TypeId::ERROR, // TODO: Polymorphic integers
                    Builtin::Char => TypeId::CHAR,
                    Builtin::Float => TypeId::ERROR, // TODO: Polymorphic floats
                    Builtin::String => TypeId::STRING,
                    Builtin::Ptr => TypeId::POINTER,
                    Builtin::PairType => TypeId::PAIR,
                    Builtin::PairConstructor => {
                        // TODO: Error
                        TypeId::ERROR
                    },
                }
            },
            Some(origin) => {
                if !origin.may_be_a_type() {
                    // TODO: Error
                }
                self.types.get_or_insert_type(make_type(origin))
            },
            // Assume name resolution has already issued an error for this case
            None => TypeId::ERROR,
        }
    }

    /// Try to retrieve the types of each field of the given type.
    /// Returns an empty map if unsuccessful.
    fn get_field_types(&mut self, typ: TypeId, generic_args: Option<&[TypeId]>) -> BTreeMap<Arc<String>, TypeId> {
        match self.follow_type(typ) {
            Type::Application(constructor, arguments) => {
                // TODO: Error if `generics` is non-empty
                let constructor = *constructor;
                let arguments = arguments.clone();
                self.get_field_types(constructor, Some(&arguments))
            },
            Type::UserDefined(origin) => {
                if let Origin::TopLevelDefinition(id) = origin {
                    let (item, item_context) = GetItem(id.top_level_item).get(self.compiler);
                    if let TopLevelItemKind::TypeDefinition(definition) = &item.kind {
                        let mut substitutions = FxHashMap::default();
                        if let Some(generics) = generic_args {
                            substitutions = Self::datatype_generic_substitutions(definition, generics);
                        }

                        let resolve = Resolve(item.id).get(self.compiler);
                        return match &definition.body {
                            TypeDefinitionBody::Error => todo!(),
                            TypeDefinitionBody::Struct(items) => items
                                .iter()
                                .map(|(name, typ)| {
                                    let name = item_context.names[*name].clone();
                                    let typ2 = self.convert_foreign_type(typ, &resolve);
                                    let typ3 = self.substitute_generics(typ2, &substitutions);
                                    (name, typ3)
                                })
                                .collect(),
                            TypeDefinitionBody::Enum(_) => BTreeMap::default(),
                            TypeDefinitionBody::Alias(_) => todo!("Type aliases"),
                        };
                    }
                }
                BTreeMap::default()
            },
            Type::Primitive(types::PrimitiveType::String) => {
                let mut fields = BTreeMap::default();

                let c_string_type =
                    self.types.get_or_insert_type(Type::Application(TypeId::POINTER, vec![TypeId::CHAR]));

                // TODO: Hide these and only expose them as unsafe builtins
                fields.insert(Arc::new("c_string".into()), c_string_type);
                fields.insert(Arc::new("length".into()), TypeId::U32);
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
        definition: &cst::TypeDefinition, replacements: &[TypeId],
    ) -> FxHashMap<Generic, TypeId> {
        let mut substitutions = FxHashMap::default();
        if definition.generics.len() == replacements.len() {
            for (generic, replacement) in definition.generics.iter().zip(replacements) {
                substitutions.insert(Generic::Named(Origin::Local(*generic)), *replacement);
            }
        }
        substitutions
    }

    /// Converts a 'foreign' type to a TypeId.
    /// A foreign type here is defined as a `cst::Type` with a TopLevelContext/ResolutionResult different to the one
    /// in `self`, hence we need to take the other context as an argument.
    fn convert_foreign_type(&mut self, typ: &crate::parser::cst::Type, resolve: &ResolutionResult) -> TypeId {
        match typ {
            crate::parser::cst::Type::Integer(kind) => match kind {
                IntegerKind::I8 => TypeId::I8,
                IntegerKind::I16 => TypeId::I16,
                IntegerKind::I32 => TypeId::I32,
                IntegerKind::I64 => TypeId::I64,
                IntegerKind::Isz => TypeId::ISZ,
                IntegerKind::U8 => TypeId::U8,
                IntegerKind::U16 => TypeId::U16,
                IntegerKind::U32 => TypeId::U32,
                IntegerKind::U64 => TypeId::U64,
                IntegerKind::Usz => TypeId::USZ,
            },
            crate::parser::cst::Type::Float(kind) => match kind {
                FloatKind::F32 => TypeId::F32,
                FloatKind::F64 => TypeId::F64,
            },
            crate::parser::cst::Type::String => TypeId::STRING,
            crate::parser::cst::Type::Char => TypeId::CHAR,
            crate::parser::cst::Type::Named(path) => {
                let origin = resolve.path_origins.get(path).copied();
                self.convert_origin_to_type(origin, Type::UserDefined)
            },
            crate::parser::cst::Type::Variable(name) => {
                let origin = resolve.name_origins.get(name).copied();
                self.convert_origin_to_type(origin, |origin| Type::Generic(Generic::Named(origin)))
            },
            crate::parser::cst::Type::Function(function) => {
                let parameters = vecmap(&function.parameters, |typ| self.convert_foreign_type(typ, resolve));
                let return_type = self.convert_foreign_type(&function.return_type, resolve);
                // TODO: Effects
                let effects = TypeId::UNIT;
                let typ = Type::Function(types::FunctionType { parameters, return_type, effects });
                self.types.get_or_insert_type(typ)
            },
            crate::parser::cst::Type::Error => TypeId::ERROR,
            crate::parser::cst::Type::Unit => TypeId::UNIT,
            crate::parser::cst::Type::Pair => TypeId::PAIR,
            crate::parser::cst::Type::Application(f, args) => {
                let f = self.convert_foreign_type(f, resolve);
                let args = vecmap(args, |typ| self.convert_foreign_type(typ, resolve));
                let typ = Type::Application(f, args);
                self.types.get_or_insert_type(typ)
            },
            crate::parser::cst::Type::Reference(mutability, sharedness) => {
                let typ = Type::Reference(*mutability, *sharedness);
                self.types.get_or_insert_type(typ)
            },
        }
    }
}
