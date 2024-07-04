use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use crate::cache::{DefinitionInfoId, DefinitionKind, ImplInfoId, ModuleCache, VariableId};
use crate::error::TypeErrorKind as TE;
use crate::hir;
use crate::lexer::token::FloatKind;
use crate::nameresolution::builtin::BUILTIN_ID;
use crate::parser::ast;
use crate::parser::ast::ClosureEnvironment;
use crate::types::traits::{Callsite, RequiredImpl, TraitConstraintId};
use crate::types::typechecker::{self, TypeBindings};
use crate::types::typed::Typed;
use crate::types::{self, TypeInfoId, TypeVariableId};
use crate::util::{fmap, trustme};

use super::definitions::Definitions;
use super::types::{IntegerKind, Type};

const DEFAULT_INTEGER: hir::Type = hir::Type::Primitive(hir::types::PrimitiveType::Integer(IntegerKind::I32));
const DEFAULT_FLOAT: hir::Type = hir::Type::Primitive(hir::types::PrimitiveType::Float(FloatKind::F64));

/// The type to bind most typevars to if they are still unbound when we codegen them.
const UNBOUND_TYPE: types::Type = types::Type::Primitive(types::PrimitiveType::UnitType);

/// Arbitrary recursion limit for following type variable mappings
const RECURSION_LIMIT: u32 = 500;

/// Monomorphise this ast, simplifying it by removing all generics, traits,
/// and unneeded ast constructs.
pub fn monomorphise<'c>(ast: &ast::Ast<'c>, cache: ModuleCache<'c>) -> hir::Ast {
    let mut context = Context::new(cache);
    context.monomorphise(ast)
}

pub struct Context<'c> {
    monomorphisation_bindings: Vec<Rc<TypeBindings>>,
    pub cache: ModuleCache<'c>,

    /// Monomorphisation can result in what was 1 DefinitionInfoId being split into
    /// many different monomorphised variants, each represented by a unique hir::DefinitionId.
    pub definitions: Definitions,

    types: HashMap<(types::TypeInfoId, Vec<types::Type>), Type>,

    /// Compile-time mapping of variable -> definition for impls that were resolved
    /// after type inference. This is needed for definitions that are polymorphic in
    /// the impls they may use within.
    impl_mappings: Vec<Impls>,

    next_id: usize,
}

type Impls = HashMap<VariableId, Impl>;

#[derive(Debug, Default)]
struct Impl {
    direct_binding: Option<DefinitionInfoId>,
    indirect: HashMap<TraitConstraintId, Vec<(Vec<TraitConstraintId>, ImplInfoId)>>,
}

impl Impl {
    fn find_indirect(&self, id: TraitConstraintId) -> &[(Vec<TraitConstraintId>, ImplInfoId)] {
        expect_opt!(self.indirect.get(&id), "No impl for key {:?}. Current mapping:\n{:?}", id, self)
    }
}

#[derive(Debug, Clone)]
pub enum Definition {
    /// A Macro definition is one that should be substituted for its rhs
    /// each time it is used. An example is non-function type constructors
    /// like 'None'. If 'None' were a Normal definition it would be forced
    /// to be a global variable to be shared across all funcitons, which
    /// would be less efficient than recreating the value 0 on each use.
    Macro(hir::Ast),
    Normal(hir::DefinitionInfo),
}

impl Definition {
    fn reference(self) -> hir::Ast {
        match self {
            Definition::Macro(ast) => ast,
            Definition::Normal(def) => hir::Ast::Variable(def),
        }
    }
}

impl<'c> Context<'c> {
    fn new(cache: ModuleCache) -> Context {
        Context {
            monomorphisation_bindings: vec![],
            definitions: Definitions::new(),
            types: HashMap::new(),
            impl_mappings: vec![HashMap::new()],
            next_id: 0,
            cache,
        }
    }

    pub fn next_unique_id(&mut self) -> hir::DefinitionId {
        let id = self.next_id;
        self.next_id += 1;
        hir::DefinitionId(id)
    }

    pub fn monomorphise(&mut self, ast: &ast::Ast<'c>) -> hir::Ast {
        use ast::Ast::*;
        match ast {
            Literal(literal) => self.monomorphise_literal(literal, literal.typ.as_ref().unwrap()),
            Variable(variable) => self.monomorphise_variable(variable),
            Lambda(lambda) => self.monomorphise_lambda(lambda),
            FunctionCall(call) => self.monomorphise_call(call),
            Definition(definition) => self.monomorphise_definition(definition),
            If(if_) => self.monomorphise_if(if_),
            Match(match_) => self.monomorphise_match(match_),
            TypeDefinition(_) => unit_literal(),
            TypeAnnotation(annotation) => self.monomorphise(&annotation.lhs),
            Import(_) => unit_literal(),
            TraitDefinition(_) => unit_literal(),
            TraitImpl(_) => unit_literal(),
            Return(return_) => self.monomorphise_return(return_),
            Sequence(sequence) => self.monomorphise_sequence(sequence),
            Extern(_) => unit_literal(),
            MemberAccess(member_access) => self.monomorphise_member_access(member_access),
            Assignment(assignment) => self.monomorphise_assignment(assignment),
            EffectDefinition(_) => todo!(),
            Handle(_) => todo!(),
            NamedConstructor(constructor) => self.monomorphise_named_constructor(constructor),
        }
    }

    /// Follow the bindings as far as possible.
    /// Returns a non-type variable on success.
    /// Returns the last type variable found on failure.
    fn find_binding(&self, id: TypeVariableId, fuel: u32) -> Result<&types::Type, TypeVariableId> {
        use types::Type::*;
        use types::TypeBinding::*;

        if fuel == 0 {
            panic!("Recursion limit reached in find_binding");
        }

        let fuel = fuel - 1;
        match &self.cache.type_bindings[id.0] {
            Bound(TypeVariable(id2) | Ref(_, _, id2)) => self.find_binding(*id2, fuel),
            Bound(binding) => Ok(binding),
            Unbound(..) => {
                for bindings in self.monomorphisation_bindings.iter().rev() {
                    match bindings.get(&id) {
                        Some(TypeVariable(id2) | Ref(_, _, id2)) => return self.find_binding(*id2, fuel),
                        Some(binding) => return Ok(binding),
                        None => (),
                    }
                }
                Err(id)
            },
        }
    }

    /// If this type is a type variable, follow what it is bound to
    /// until we find the first type that isn't also a type variable.
    fn follow_bindings_shallow<'a>(&'a self, typ: &'a types::Type) -> Result<&'a types::Type, TypeVariableId> {
        use types::Type::*;

        match typ {
            TypeVariable(id) => self.find_binding(*id, RECURSION_LIMIT),
            _ => Ok(typ),
        }
    }

    /// Recursively follow all type variables in this type such that all Bound
    /// type variables are replaced with whatever they are bound to.
    pub fn follow_all_bindings<'a>(&'a self, typ: &'a types::Type) -> types::Type {
        self.follow_all_bindings_inner(typ, RECURSION_LIMIT)
    }

    fn follow_all_bindings_inner<'a>(&'a self, typ: &'a types::Type, fuel: u32) -> types::Type {
        use types::Type::*;

        if fuel == 0 {
            panic!("Recursion limit reached in convert_type");
        }

        let fuel = fuel - 1;
        match typ {
            TypeVariable(id) => match self.find_binding(*id, fuel) {
                Ok(binding) => self.follow_all_bindings_inner(binding, fuel),
                Err(id) => TypeVariable(id),
            },
            Primitive(_) => typ.clone(),
            Function(f) => {
                let f = types::FunctionType {
                    parameters: fmap(&f.parameters, |param| self.follow_all_bindings_inner(param, fuel)),
                    return_type: Box::new(self.follow_all_bindings_inner(&f.return_type, fuel)),
                    environment: Box::new(self.follow_all_bindings_inner(&f.environment, fuel)),
                    effects: Box::new(self.follow_all_bindings_inner(&f.effects, fuel)),
                    is_varargs: f.is_varargs,
                };
                Function(f)
            },
            UserDefined(_) => typ.clone(),
            TypeApplication(con, args) => {
                let con = self.follow_all_bindings_inner(con, fuel);
                let args = fmap(args, |arg| self.follow_all_bindings_inner(arg, fuel));
                TypeApplication(Box::new(con), args)
            },
            Ref(..) => typ.clone(),
            Struct(fields, id) => match self.find_binding(*id, fuel) {
                Ok(binding) => self.follow_all_bindings_inner(binding, fuel),
                Err(_) => {
                    let fields = fields
                        .iter()
                        .map(|(name, typ)| (name.clone(), self.follow_all_bindings_inner(typ, fuel)))
                        .collect();

                    Struct(fields, *id)
                },
            },
            Effects(effects) => self.follow_all_effect_bindings_inner(effects, fuel),
        }
    }

    fn follow_all_effect_bindings_inner<'a>(
        &'a self, effects: &'a types::effects::EffectSet, fuel: u32,
    ) -> types::Type {
        let replacement = match self.find_binding(effects.replacement, fuel) {
            Ok(binding) => return self.follow_all_bindings_inner(binding, fuel),
            Err(id) => id,
        };

        let effects = fmap(&effects.effects, |(id, args)| {
            let args = fmap(args, |arg| self.follow_all_bindings_inner(arg, fuel));
            (*id, args)
        });

        types::Type::Effects(types::effects::EffectSet { effects, replacement })
    }

    fn size_of_struct_type(&mut self, info: &types::TypeInfo, fields: &[types::Field], args: &[types::Type]) -> usize {
        let bindings = typechecker::type_application_bindings(info, args, &self.cache);

        fields
            .iter()
            .map(|field| {
                let field_type = typechecker::bind_typevars(&field.field_type, &bindings, &self.cache);
                self.size_of_type(&field_type)
            })
            .sum()
    }

    fn size_of_union_type(
        &mut self, info: &types::TypeInfo, variants: &[types::TypeConstructor<'c>], args: &[types::Type],
    ) -> usize {
        let bindings = typechecker::type_application_bindings(info, args, &self.cache);

        match self.find_largest_union_variant(variants, &bindings) {
            None => 0, // Void type
            Some(variant) => {
                // The size of a union is the size of its largest field, plus 1 byte for the tag
                variant.iter().map(|field| self.size_of_type(field)).sum::<usize>() + 1
            },
        }
    }

    fn size_of_user_defined_type(&mut self, id: TypeInfoId, args: &[types::Type]) -> usize {
        let info = &self.cache[id];
        assert!(info.args.len() == args.len(), "Kind error during llvm code generation");

        use types::TypeInfoBody::*;
        match &info.body {
            // TODO: Need to split out self.types and self.cache parameters to be able to remove this
            Union(variants) => trustme::make_mut_ref(self).size_of_union_type(info, variants, args),
            Struct(fields) => trustme::make_mut_ref(self).size_of_struct_type(info, fields, args),

            // Aliases should be desugared prior to codegen
            Alias(_) => unreachable!(),
            Unknown => unreachable!(),
        }
    }

    /// TODO: Adjust based on target architecture
    fn ptr_size() -> usize {
        std::mem::size_of::<*const i8>()
    }

    /// Returns the size in bits of this integer.
    ///
    /// Will bind the integer to an i32 if this integer is an IntegerKind::Inferred
    /// that has not already been bound to a concrete type.
    fn integer_bit_count(&mut self, kind: crate::lexer::token::IntegerKind) -> u32 {
        use IntegerKind::*;
        match self.convert_integer_kind(kind) {
            I8 | U8 => 8,
            I16 | U16 => 16,
            I32 | U32 => 32,
            I64 | U64 => 64,
            Isz | Usz => Self::ptr_size() as u32 * 8,
        }
    }

    fn size_of_type(&mut self, typ: &types::Type) -> usize {
        use types::PrimitiveType::*;
        use types::Type::*;
        match typ {
            Primitive(IntegerTag(kind)) => self.integer_bit_count(*kind) as usize / 8,
            Primitive(FloatTag(FloatKind::F32)) => 4,
            Primitive(FloatTag(FloatKind::F64)) => 8,
            Primitive(CharType) => 1,
            Primitive(BooleanType) => 1,
            Primitive(UnitType) => 1,
            Primitive(Ptr) => Self::ptr_size(),
            Primitive(IntegerType) => {
                unreachable!("'Int' type constructor without arguments found during size_of_type")
            },
            Primitive(FloatType) => {
                unreachable!("'Float' type constructor without arguments found during size_of_type")
            },

            Function(..) => Self::ptr_size(),

            TypeVariable(id) => {
                let binding = self.find_binding(*id, RECURSION_LIMIT).unwrap_or(&UNBOUND_TYPE).clone();
                self.size_of_type(&binding)
            },

            UserDefined(id) => self.size_of_user_defined_type(*id, &[]),

            TypeApplication(typ, args) => match self.follow_bindings_shallow(typ.as_ref()) {
                Ok(UserDefined(id)) => {
                    let id = *id;
                    self.size_of_user_defined_type(id, args)
                },
                Ok(Primitive(Ptr)) => Self::ptr_size(),
                Ok(Primitive(IntegerType)) => {
                    match self.follow_bindings_shallow(&args[0]) {
                        Ok(typ) => self.size_of_type(&typ.clone()),
                        Err(_) => 4, // size_of DEFAULT_INTEGER
                    }
                },
                Ok(Primitive(FloatType)) => {
                    match self.follow_bindings_shallow(&args[0]) {
                        Ok(typ) => self.size_of_type(&typ.clone()),
                        Err(_) => 8, // size_of DEFAULT_FLOAT
                    }
                },
                _ => unreachable!("Kind error inside size_of_type"),
            },

            Ref(..) => Self::ptr_size(),
            Struct(fields, rest) => {
                if let Ok(binding) = self.find_binding(*rest, RECURSION_LIMIT) {
                    let binding = binding.clone();
                    self.size_of_type(&binding)
                } else {
                    fields.iter().map(|(_, field)| self.size_of_type(field)).sum()
                }
            },
            Effects(_) => unreachable!(),
        }
    }

    fn convert_primitive_type(&mut self, typ: &types::PrimitiveType) -> Type {
        use types::PrimitiveType::*;
        Type::Primitive(match typ {
            IntegerTag(kind) => {
                let kind = self.convert_integer_kind(*kind);
                hir::types::PrimitiveType::Integer(kind)
            },
            FloatTag(kind) => hir::types::PrimitiveType::Float(*kind),
            IntegerType => unreachable!("Unbound Int type in convert_primitive_type"),
            FloatType => unreachable!("Unbound Float type in convert_primitive_type"),
            CharType => hir::types::PrimitiveType::Char,
            BooleanType => hir::types::PrimitiveType::Boolean,
            UnitType => hir::types::PrimitiveType::Unit,
            Ptr => hir::types::PrimitiveType::Pointer,
        })
    }

    fn convert_struct_type(
        &mut self, id: TypeInfoId, info: &types::TypeInfo, fields: &[types::Field<'c>], args: Vec<types::Type>,
    ) -> Type {
        let bindings = typechecker::type_application_bindings(info, &args, &self.cache);

        let t = Type::Tuple(vec![]);
        self.types.insert((id, args.clone()), t);

        let fields = fmap(fields, |field| {
            let field_type = typechecker::bind_typevars(&field.field_type, &bindings, &self.cache);
            self.convert_type(&field_type)
        });

        let t = Type::Tuple(fields);
        self.types.insert((id, args), t.clone());
        t
    }

    /// Given a list of TypeConstructors representing each variant of a sum type,
    /// find the largest variant in memory (with the given type bindings for any type variables)
    /// and return its field types.
    fn find_largest_union_variant(
        &mut self, variants: &[types::TypeConstructor<'c>], bindings: &TypeBindings,
    ) -> Option<Vec<types::Type>> {
        let variants: Vec<Vec<types::Type>> =
            fmap(variants, |variant| fmap(&variant.args, |arg| typechecker::bind_typevars(arg, bindings, &self.cache)));

        variants.into_iter().max_by_key(|variant| variant.iter().map(|arg| self.size_of_type(arg)).sum::<usize>())
    }

    /// Returns the type of a tag in an unoptimized tagged union
    pub fn tag_type() -> Type {
        Type::Primitive(hir::types::PrimitiveType::Integer(IntegerKind::U8))
    }

    fn convert_union_type(
        &mut self, id: TypeInfoId, info: &types::TypeInfo, variants: &[types::TypeConstructor<'c>],
        args: Vec<types::Type>,
    ) -> Type {
        let bindings = typechecker::type_application_bindings(info, &args, &self.cache);

        let mut t = Type::Tuple(vec![]);

        if let Some(variant) = self.find_largest_union_variant(variants, &bindings) {
            self.types.insert((id, args.clone()), t);

            let mut fields = vec![Self::tag_type()];
            for typ in variant {
                fields.push(self.convert_type(&typ));
            }

            t = Type::Tuple(fields);
        }

        self.types.insert((id, args), t.clone());
        t
    }

    fn convert_user_defined_type(&mut self, id: TypeInfoId, args: Vec<types::Type>) -> Type {
        let info = &self.cache[id];
        assert!(info.args.len() == args.len(), "Kind error during monomorphisation");

        if let Some(typ) = self.types.get(&(id, args.clone())) {
            return typ.clone();
        }

        use types::TypeInfoBody::*;
        let typ = match &info.body {
            // TODO: Need to split out self.types and self.cache parameters to be able to remove this
            Union(variants) => trustme::make_mut_ref(self).convert_union_type(id, info, variants, args),
            Struct(fields) => trustme::make_mut_ref(self).convert_struct_type(id, info, fields, args),

            // Aliases should be desugared prior to codegen
            Alias(_) => unreachable!(),
            Unknown => unreachable!(),
        };

        typ
    }

    fn empty_closure_environment(&self, environment: &types::Type) -> bool {
        self.follow_bindings_shallow(environment).map_or(false, |env| env.is_unit(&self.cache))
    }

    /// Monomorphise a types::Type into a hir::Type with no generics.
    pub fn convert_type(&mut self, typ: &types::Type) -> Type {
        self.convert_type_inner(typ, RECURSION_LIMIT)
    }

    fn is_type_variable(&self, typ: &types::Type) -> bool {
        self.follow_bindings_shallow(typ).is_err()
    }

    pub fn convert_type_inner(&mut self, typ: &types::Type, fuel: u32) -> Type {
        use types::PrimitiveType;
        use types::Type::*;

        if fuel == 0 {
            panic!("Recursion limit reached in convert_type");
        }

        let fuel = fuel - 1;
        match typ {
            Primitive(primitive) => self.convert_primitive_type(primitive),

            Function(function) => {
                let mut parameters = fmap(&function.parameters, |typ| self.convert_type_inner(typ, fuel));

                let return_type = Box::new(self.convert_type_inner(&function.return_type, fuel));

                let environment = (!self.empty_closure_environment(&function.environment)).then(|| {
                    let environment_parameter = self.convert_type_inner(&function.environment, fuel);
                    parameters.push(environment_parameter.clone());
                    environment_parameter
                });

                let function = Type::Function(hir::types::FunctionType {
                    parameters,
                    return_type,
                    is_varargs: function.is_varargs,
                });

                match environment {
                    None => function,
                    Some(environment) => Type::Tuple(vec![function, environment]),
                }
            },

            TypeVariable(id) => match self.find_binding(*id, fuel) {
                Ok(binding) => {
                    let binding = binding.clone();
                    self.convert_type_inner(&binding, fuel)
                },
                Err(_) => self.convert_type_inner(&UNBOUND_TYPE, fuel),
            },

            UserDefined(id) => self.convert_user_defined_type(*id, vec![]),

            TypeApplication(typ, args) => {
                let args = fmap(args, |arg| self.follow_all_bindings(arg));
                let typ = self.follow_bindings_shallow(typ);

                match typ {
                    Ok(Primitive(PrimitiveType::Ptr) | Ref(..)) => Type::Primitive(hir::PrimitiveType::Pointer),
                    Ok(Primitive(PrimitiveType::IntegerType)) => {
                        if self.is_type_variable(&args[0]) {
                            // Default to i32
                            DEFAULT_INTEGER
                        } else {
                            self.convert_type_inner(&args[0], fuel)
                        }
                    },
                    Ok(Primitive(PrimitiveType::FloatType)) => {
                        if self.is_type_variable(&args[0]) {
                            // Default to f64
                            DEFAULT_FLOAT
                        } else {
                            self.convert_type_inner(&args[0], fuel)
                        }
                    },
                    Ok(UserDefined(id)) => {
                        let id = *id;
                        self.convert_user_defined_type(id, args)
                    },
                    Ok(other) => {
                        unreachable!(
                            "Type {} requires 0 type args but was applied to {:?}",
                            other.display(&self.cache),
                            args
                        );
                    },
                    Err(var) => {
                        unreachable!("Tried to apply an unbound type variable (id {}), args: {:?}", var.0, args);
                    },
                }
            },

            Ref(..) => {
                unreachable!(
                    "Kind error during monomorphisation. Attempted to translate a `ref` without a type argument"
                )
            },
            Struct(fields, rest) => {
                if let Ok(binding) = self.find_binding(*rest, fuel) {
                    let binding = binding.clone();
                    return self.convert_type_inner(&binding, fuel);
                }

                Type::Tuple(fmap(fields, |(_, field)| self.convert_type_inner(field, fuel)))
            },
            Effects(_) => unreachable!(),
        }
    }

    fn convert_integer_kind(&self, kind: crate::lexer::token::IntegerKind) -> IntegerKind {
        use crate::lexer::token::IntegerKind;
        match kind {
            IntegerKind::I8 => hir::IntegerKind::I8,
            IntegerKind::I16 => hir::IntegerKind::I16,
            IntegerKind::I32 => hir::IntegerKind::I32,
            IntegerKind::I64 => hir::IntegerKind::I64,
            IntegerKind::Isz => hir::IntegerKind::Isz,
            IntegerKind::U8 => hir::IntegerKind::U8,
            IntegerKind::U16 => hir::IntegerKind::U16,
            IntegerKind::U32 => hir::IntegerKind::U32,
            IntegerKind::U64 => hir::IntegerKind::U64,
            IntegerKind::Usz => hir::IntegerKind::Usz,
        }
    }

    fn monomorphise_literal(&mut self, literal: &ast::Literal, typ: &types::Type) -> hir::Ast {
        use hir::Ast::*;
        use hir::Literal::*;

        match &literal.kind {
            ast::LiteralKind::Integer(n, _) => {
                let kind = match self.convert_type(typ) {
                    Type::Primitive(hir::PrimitiveType::Integer(kind)) => kind,
                    other => unreachable!("monomorphise_literal: expected integer type, found {}", other),
                };
                Literal(Integer(*n, kind))
            },
            ast::LiteralKind::Float(f, _) => {
                let kind = match self.convert_type(typ) {
                    Type::Primitive(hir::PrimitiveType::Float(kind)) => kind,
                    other => unreachable!("monomorphise_literal: expected float type, found {}", other),
                };
                Literal(Float(*f, kind))
            },
            ast::LiteralKind::String(s) => {
                let len = Literal(Integer(s.len() as u64, IntegerKind::Usz));
                let c_string = Literal(CString(s.clone()));

                Tuple(hir::Tuple { fields: vec![c_string, len] })
            },
            ast::LiteralKind::Char(c) => Literal(Char(*c)),
            ast::LiteralKind::Bool(b) => Literal(Bool(*b)),
            ast::LiteralKind::Unit => unit_literal(),
        }
    }

    fn add_required_impls(&mut self, required_impls: &[RequiredImpl]) {
        let new_impls = self.impl_mappings.last_mut().unwrap();

        for required_impl in required_impls {
            match &required_impl.callsite {
                Callsite::Direct(callsite) => {
                    let binding = self.cache.find_method_in_impl(*callsite, required_impl.binding);
                    new_impls.entry(*callsite).or_default().direct_binding = Some(binding);
                },
                Callsite::Indirect(callsite, ids) => {
                    let mut ids = ids.clone();
                    let top = ids.remove(0);

                    new_impls
                        .entry(*callsite)
                        .or_default()
                        .indirect
                        .entry(top)
                        .or_default()
                        .push((ids, required_impl.binding));
                },
            }
        }
    }

    /// Get the DefinitionInfoId this variable should point to. This is usually
    /// given by variable.definition but in the case of static trait dispatch,
    /// self.impl_mappings may be set to bind a given variable id to another
    /// definition. This is currently only done for trait functions/values to
    /// point them to impls that actually have definitions.
    fn get_definition_id(&self, variable: &ast::Variable<'c>) -> DefinitionInfoId {
        self.impl_mappings
            .last()
            .unwrap()
            .get(&variable.id.unwrap())
            .and_then(|impl_| impl_.direct_binding)
            .unwrap_or_else(|| variable.definition.unwrap())
    }

    fn monomorphise_variable(&mut self, variable: &ast::Variable<'c>) -> hir::Ast {
        let id = variable.id.unwrap();
        let required_impls = self.cache[id].required_impls.clone();

        self.add_required_impls(&required_impls);

        // The definition to compile is either the corresponding impl definition if this
        // variable refers to a trait function, or otherwise it is the regular definition of this variable.
        let definition_id = self.get_definition_id(variable);

        let typ = variable.typ.as_ref().unwrap();
        let definition = self.monomorphise_definition_id(definition_id, id, typ, &variable.instantiation_mapping);

        definition.reference()
    }

    pub fn lookup_definition(&self, id: DefinitionInfoId, typ: &types::Type) -> Option<Definition> {
        let typ = self.follow_all_bindings(typ);
        self.definitions.get(id, typ).cloned()
    }

    fn push_monomorphisation_bindings(
        &mut self, instantiation_mapping: &Rc<TypeBindings>, typ: &types::Type,
        definition: &crate::cache::DefinitionInfo<'c>,
    ) {
        if !instantiation_mapping.is_empty() {
            self.monomorphisation_bindings.push(instantiation_mapping.clone());
        }

        if definition.trait_impl.is_some() {
            let definition_type = definition.typ.as_ref().unwrap().remove_forall();
            let bindings = typechecker::try_unify(
                definition_type,
                typ,
                definition.location,
                &mut self.cache,
                TE::MonomorphizationError,
            )
            .unwrap_or_else(|error| panic!("ICE: {}", error.display(&self.cache)));

            self.monomorphisation_bindings.push(Rc::new(bindings.bindings));
        }
    }

    fn pop_monomorphisation_bindings(
        &mut self, instantiation_mapping: &Rc<TypeBindings>, definition: &crate::cache::DefinitionInfo,
    ) {
        if !instantiation_mapping.is_empty() {
            self.monomorphisation_bindings.pop();
        }

        if definition.trait_impl.is_some() {
            self.monomorphisation_bindings.pop();
        }
    }

    fn add_required_traits(&mut self, definition: &crate::cache::DefinitionInfo, variable_id: VariableId) {
        let mut new_impls = Impls::new();

        for required_trait in &definition.required_traits {
            // If the impl has 0 definitions we can't attach it to any variables
            if self.cache[required_trait.signature.trait_id].definitions.is_empty() {
                continue;
            }

            let impls = self.impl_mappings.last().unwrap().get(&variable_id);

            let impls = match impls {
                Some(binding) => binding,
                None => {
                    let trait_ = required_trait.display(&self.cache);
                    panic!(
                        "Monomorphisation: no entry found for impl key {:?} ({}) for trait {} in definition {}.\nimpl_mappings.last() = {:?}",
                        variable_id, self.cache[variable_id].name, trait_, definition.name, self.impl_mappings.last().unwrap(),
                    )
                },
            };

            let mut bindings = impls.find_indirect(required_trait.signature.id).to_vec();

            match &required_trait.callsite {
                Callsite::Direct(callsite) => {
                    for (mut path, impl_) in bindings.iter().cloned() {
                        if !path.is_empty() {
                            let top = path.remove(0);
                            new_impls
                                .entry(*callsite)
                                .or_default()
                                .indirect
                                .entry(top)
                                .or_default()
                                .push((path, impl_));
                        } else {
                            let binding = self.cache.find_method_in_impl(*callsite, impl_);
                            new_impls.entry(*callsite).or_default().direct_binding = Some(binding);
                        }
                    }
                },
                Callsite::Indirect(callsite, ids) => {
                    if ids.len() == 1 {
                        let top = *ids.last().unwrap();
                        new_impls.entry(*callsite).or_default().indirect.entry(top).or_default().append(&mut bindings);
                    } else {
                        let mut ids = ids.clone();
                        let top = ids.remove(0);

                        for (path, _) in &mut bindings {
                            path.append(&mut ids.clone());
                        }

                        new_impls
                            .entry(*callsite)
                            .or_default()
                            .indirect
                            .entry(top)
                            .or_default()
                            .append(&mut bindings.to_vec());
                    }
                },
            }
        }

        self.impl_mappings.push(new_impls);
    }

    fn monomorphise_definition_id(
        &mut self, id: DefinitionInfoId, variable_id: VariableId, typ: &types::Type,
        instantiation_mapping: &Rc<TypeBindings>,
    ) -> Definition {
        if let Some(value) = self.lookup_definition(id, typ) {
            return value;
        }

        let typ = self.follow_all_bindings(typ);

        let info = trustme::extend_lifetime(&mut self.cache[id]);
        self.push_monomorphisation_bindings(instantiation_mapping, &typ, info);
        self.add_required_traits(info, variable_id);

        // Compile the definition with the bindings in scope. Each definition is expected to
        // add itself to Generator.definitions
        let value = match &info.definition {
            Some(DefinitionKind::Definition(definition)) => {
                // Any recursive calls to this variable will refer to this binding
                let definition_id = self.next_unique_id();
                let name = info.name.clone();
                let monomorphized_type = Rc::new(self.convert_type(&typ));
                let info = hir::DefinitionInfo {
                    definition: None,
                    definition_id,
                    name: Some(name.clone()),
                    typ: monomorphized_type,
                };

                self.definitions.insert(id, typ.clone(), Definition::Normal(info));

                let def = self.monomorphise_nonlocal_definition(definition, definition_id, name);

                self.definitions.insert(id, typ, def.clone());
                def
            },
            Some(DefinitionKind::Extern(_)) => self.make_extern(id, &typ),
            Some(DefinitionKind::TypeConstructor { tag, name: _ }) => {
                let definition = self.monomorphise_type_constructor(tag, &typ);
                self.define_type_constructor(definition, id, typ)
            },
            Some(DefinitionKind::TraitDefinition(_)) => {
                unreachable!(
                    "Cannot monomorphise from a TraitDefinition.\nNo cached impl for {} {}: {}",
                    info.name,
                    id.0,
                    typ.debug(&self.cache)
                )
            },
            Some(DefinitionKind::EffectDefinition(_)) => {
                unreachable!(
                    "Cannot monomorphise from a EffectDefinition.\nNo cached handle for {} {}: {}",
                    info.name,
                    id.0,
                    typ.debug(&self.cache)
                )
            },
            Some(DefinitionKind::Parameter) => {
                unreachable!(
                    "Parameters should already be defined.\nEncountered while compiling {} {}: {}, {:?}",
                    info.name,
                    id.0,
                    typ.debug(&self.cache),
                    typ
                )
            },
            Some(DefinitionKind::MatchPattern) => {
                unreachable!(
                    "MatchPatterns should already be defined.\n Encountered while compiling {} {}: {}",
                    info.name,
                    id.0,
                    typ.debug(&self.cache)
                )
            },
            None => unreachable!("No definition for {} {}", info.name, id.0),
        };

        self.impl_mappings.pop();
        self.pop_monomorphisation_bindings(instantiation_mapping, info);
        value
    }

    /// This function is 'make_extern' rathern than 'monomorphise_extern' since extern declarations
    /// shouldn't be monomorphised across multiple types.
    fn make_extern(&mut self, id: DefinitionInfoId, typ: &types::Type) -> Definition {
        // extern definitions should only be declared once - never duplicated & monomorphised.
        // For this reason their value is always stored with the Unit type in the definitions map.
        if let Some(value) = self.lookup_definition(id, &UNBOUND_TYPE) {
            self.definitions.insert(id, typ.clone(), value.clone());
            return value;
        }

        let name = self.cache[id].name.clone();
        let monomorphized_type = self.convert_type(typ);
        let extern_ = hir::Ast::Extern(hir::Extern { name: name.clone(), typ: monomorphized_type.clone() });

        let definition = self.make_definition(extern_, Some(name), Rc::new(monomorphized_type));

        // Insert the global for both the current type and the unit type
        let definition = Definition::Normal(definition);

        // TODO: Perhaps we should have a DefinitionKind (not cache::DefinitionKind)
        // to differentiate over the required keys for a given Definition.
        // - Id for extern
        // - (Id, Type) for globals
        // - (Id, Type, ParentId) for locals, to get rid of "all" and "local" fields within
        //   self.definitions and the duplication it requires.
        self.definitions.insert(id, typ.clone(), definition.clone());
        self.definitions.insert(id, UNBOUND_TYPE.clone(), definition.clone());
        definition
    }

    /// Wrap the given Ast in a new DefinitionInfo and store it
    fn define_type_constructor(
        &mut self, definition_rhs: hir::Ast, original_id: DefinitionInfoId, typ: types::Type,
    ) -> Definition {
        let def = if matches!(&definition_rhs, hir::Ast::Lambda(_)) {
            let variable = self.next_unique_id();
            let expr = Box::new(definition_rhs);

            let name = Some(self.cache[original_id].name.clone());
            let definition = hir::Definition { variable, expr, name };
            let typ = Rc::new(self.convert_type(&typ));
            Definition::Normal(hir::Variable::with_definition(definition, typ))
        } else {
            Definition::Macro(definition_rhs)
        };

        self.definitions.insert(original_id, typ, def.clone());
        def
    }

    pub fn fresh_variable(&mut self, typ: Type) -> hir::Variable {
        hir::Variable { definition: None, definition_id: self.next_unique_id(), name: None, typ: Rc::new(typ) }
    }

    pub fn fresh_definition(
        &mut self, definition_rhs: hir::Ast, name: Option<String>,
    ) -> (hir::Ast, hir::DefinitionId) {
        let variable = self.next_unique_id();
        let expr = Box::new(definition_rhs);
        let definition = hir::Ast::Definition(hir::Definition { variable, expr, name });
        (definition, variable)
    }

    fn make_definition(
        &mut self, definition_rhs: hir::Ast, name: Option<String>, typ: Rc<Type>,
    ) -> hir::DefinitionInfo {
        let (definition, definition_id) = self.fresh_definition(definition_rhs, name.clone());
        hir::DefinitionInfo { definition_id, definition: Some(Rc::new(definition)), name, typ }
    }

    /// Monomorphise a definition defined elsewhere
    ///
    /// TODO: This may be a clone of monomorphise_definition now
    fn monomorphise_nonlocal_definition(
        &mut self, definition: &ast::Definition<'c>, definition_id: hir::DefinitionId, name: String,
    ) -> Definition {
        let value = self.monomorphise(&definition.expr);
        let value = self.fix_recursive_closure_calls(value, definition, definition_id);

        let mut expr = Box::new(value);

        if definition.mutable {
            expr = Box::new(hir::Ast::Builtin(hir::Builtin::StackAlloc(expr)));
        }

        let variable = definition_id;
        let new_definition = hir::Ast::Definition(hir::Definition { variable, expr, name: Some(name.clone()) });

        let mut nested_definitions = vec![new_definition];
        let typ = self.follow_all_bindings(definition.pattern.get_type().unwrap());

        self.desugar_pattern(&definition.pattern, definition_id, typ.clone(), &mut nested_definitions);

        let definition = if nested_definitions.len() == 1 {
            nested_definitions.remove(0)
        } else {
            hir::Ast::Sequence(hir::Sequence { statements: nested_definitions })
        };

        let definition = Some(Rc::new(definition));
        let typ = Rc::new(self.convert_type(&typ));
        Definition::Normal(hir::Variable { definition_id, definition, name: Some(name), typ })
    }

    /// Simplifies a pattern and expression like `(a, b) = foo ()`
    /// into multiple successive bindings:
    /// ```ante
    /// new_var = foo ()
    /// a = extract 0 new_var
    /// b = extract 1 new_var
    /// ```
    /// This function will not add the new variables into self.definitions
    /// as they should not be able to be referenced externally - unlike `a` and `b` above.
    ///
    /// PRE-REQUISITE: `typ` must equal `self.follow_all_bindings(typ)`
    fn desugar_pattern(
        &mut self, pattern: &ast::Ast<'c>, definition_id: hir::DefinitionId, typ: types::Type,
        definitions: &mut Vec<hir::Ast>,
    ) {
        use {
            ast::Ast::{FunctionCall, Literal, TypeAnnotation, Variable},
            ast::LiteralKind,
        };

        match pattern {
            Literal(literal) => assert_eq!(literal.kind, LiteralKind::Unit),
            Variable(variable_pattern) => {
                let id = variable_pattern.definition.unwrap();

                let name = Some(variable_pattern.to_string());
                let monomorphized_type = Rc::new(self.convert_type(&typ));
                let variable = hir::Variable { definition_id, definition: None, name, typ: monomorphized_type };
                let definition = Definition::Normal(variable);

                self.definitions.insert(id, typ, definition);
            },
            TypeAnnotation(annotation) => {
                self.desugar_pattern(annotation.lhs.as_ref(), definition_id, typ, definitions)
            },
            // Match a struct pattern
            FunctionCall(call) if call.is_pair_constructor() => {
                let monomorphized_type = Rc::new(self.convert_type(&typ));
                let variable = hir::Variable { definition_id, definition: None, name: None, typ: monomorphized_type };

                for (i, arg_pattern) in call.args.iter().enumerate() {
                    let arg_type = self.follow_all_bindings(arg_pattern.get_type().unwrap());

                    let extract_result_type = self.convert_type(&arg_type);
                    let extract = Self::extract(variable.clone().into(), i as u32, extract_result_type);

                    let (definition, id) = self.fresh_definition(extract, None);
                    definitions.push(definition);

                    self.desugar_pattern(arg_pattern, id, arg_type, definitions)
                }
            },
            _ => {
                unreachable!();
            },
        }
    }

    fn monomorphise_type_constructor(&mut self, tag: &Option<u8>, typ: &types::Type) -> hir::Ast {
        use hir::types::Type::*;
        let typ = self.convert_type(typ);
        match typ {
            Function(function_type) => {
                let args = fmap(&function_type.parameters, |param| self.fresh_variable(param.clone()));

                let mut tuple_args = Vec::with_capacity(args.len() + 1);
                let mut tuple_size = function_type.parameters.iter().map(Self::size_of_monomorphised_type).sum();

                if let Some(tag) = tag {
                    tuple_args.push(tag_value(*tag));
                    tuple_size += Self::size_of_monomorphised_type(&Self::tag_type());
                }

                tuple_args.extend(args.iter().cloned().map(Into::into));

                let tuple = hir::Ast::Tuple(hir::Tuple { fields: tuple_args });

                let body = match tag {
                    None => tuple,
                    Some(_) => {
                        let target_type = function_type.return_type.as_ref().clone();
                        self.make_reinterpret_cast(tuple, tuple_size, target_type)
                    },
                };

                hir::Ast::Lambda(hir::Lambda { args, body: Box::new(body), typ: function_type })
            },
            // Since this is not a function type, we know it has no bundled data and we can
            // thus ignore the additional type arguments, extract the tag value, and
            // reinterpret_cast to the appropriate type.
            Tuple(..) => match tag {
                None => unit_literal(),
                Some(tag) => {
                    let value = tag_value(*tag);
                    let size = Self::size_of_monomorphised_type(&Self::tag_type());
                    self.make_reinterpret_cast(value, size, typ)
                },
            },
            Primitive(_) => {
                unreachable!("Type constructor must be a Function or Tuple type: {}", typ)
            },
        }
    }

    /// Create a reinterpret_cast instruction for the given Ast value.
    /// arg_type_size is the size of the value represented by the given ast, in bytes.
    fn make_reinterpret_cast(&mut self, ast: hir::Ast, mut arg_type_size: u32, target_type: Type) -> hir::Ast {
        let target_size = Self::size_of_monomorphised_type(&target_type);
        assert!(arg_type_size <= target_size);

        if arg_type_size == target_size {
            return hir::Ast::ReinterpretCast(hir::ReinterpretCast { lhs: Box::new(ast), target_type });
        }

        let mut padded = vec![ast];
        let type_tower = [(IntegerKind::U64, 8), (IntegerKind::U32, 4), (IntegerKind::U16, 2), (IntegerKind::U8, 1)];

        for (int_kind, size) in type_tower {
            while arg_type_size + size <= target_size {
                padded.push(int_literal(0, int_kind));
                arg_type_size += size;
            }
        }

        hir::Ast::ReinterpretCast(hir::ReinterpretCast { lhs: Box::new(tuple(padded)), target_type })
    }

    fn size_of_monomorphised_type(typ: &Type) -> u32 {
        use hir::types::PrimitiveType;
        match typ {
            Type::Primitive(p) => {
                match p {
                    PrimitiveType::Integer(kind) => {
                        use IntegerKind::*;
                        match kind {
                            I8 | U8 => 1,
                            I16 | U16 => 2,
                            I32 | U32 => 4,
                            I64 | U64 => 8,
                            Isz | Usz => Self::ptr_size() as u32,
                        }
                    },
                    PrimitiveType::Float(FloatKind::F32) => 4,
                    PrimitiveType::Float(FloatKind::F64) => 8,
                    PrimitiveType::Char => 1,
                    PrimitiveType::Boolean => 1,
                    PrimitiveType::Unit => 1, // TODO: this can depend on the backend
                    PrimitiveType::Pointer => Self::ptr_size() as u32,
                }
            },
            Type::Function(_) => Self::ptr_size() as u32, // Closures would be represented as tuples
            Type::Tuple(fields) => fields.iter().map(Self::size_of_monomorphised_type).sum(),
        }
    }

    fn get_function_type(&mut self, typ: &types::Type) -> hir::FunctionType {
        match self.convert_type(typ) {
            Type::Function(f) => f,
            Type::Tuple(mut values) => {
                // Closure
                assert!(!values.is_empty());
                match values.swap_remove(0) {
                    Type::Function(f) => f,
                    other => unreachable!("Lambda has a non-function type: {}", other),
                }
            },
            other => unreachable!("Lambda has a non-function type: {}", other),
        }
    }

    fn monomorphise_lambda(&mut self, lambda: &ast::Lambda<'c>) -> hir::Ast {
        self.definitions.push_local_scope();

        let t = lambda.typ.as_ref().unwrap();
        let t = self.follow_all_bindings(t);
        let typ = self.get_function_type(&t);
        let mut body_prelude = vec![];

        // Bind each parameter node to the nth parameter of `function`
        // This will also desugar any patterns in the parameter, prepending extra
        // statements to the function body to extract the relevant fields.
        let mut args = fmap(&lambda.args, |arg| {
            let typ = self.follow_all_bindings(arg.get_type().unwrap());
            let monomorphized_type = self.convert_type(&typ);
            let param = self.fresh_variable(monomorphized_type);
            self.desugar_pattern(arg, param.definition_id, typ, &mut body_prelude);
            param
        });

        if !lambda.closure_environment.is_empty() {
            let env = self.unpack_environment(&lambda.closure_environment, &mut body_prelude);
            args.push(env);
        }

        let body = self.monomorphise(&lambda.body);

        let body = Box::new(if body_prelude.is_empty() {
            body
        } else {
            body_prelude.push(body);
            hir::Ast::Sequence(hir::Sequence { statements: body_prelude })
        });

        let mut function = hir::Ast::Lambda(hir::Lambda { args, body, typ });

        if !lambda.closure_environment.is_empty() {
            function = self.pack_closure_environment(function, &lambda.closure_environment);
        };

        self.definitions.pop_local_scope();
        function
    }

    /// A closure's environment is structured as (a, (b, (c, ...))) while each variable {a, b, c, ...}
    /// are used directly in the function body. This function is needed at the beginning of a
    /// function to extract each variable from the environment tuple and bind it to the correct value.
    fn unpack_environment(
        &mut self, closure_environment: &ClosureEnvironment, definitions: &mut Vec<hir::Ast>,
    ) -> hir::DefinitionInfo {
        assert!(!closure_environment.is_empty());

        let mut env_type = self.make_env_type(closure_environment);
        let mut env = self.fresh_variable(env_type.clone());
        env.name = Some("env".to_string()); // Not named in source code, but this is only for debugging
        let first_env = env.clone();

        for (i, (_, (_, inner_var, _))) in closure_environment.iter().enumerate() {
            let info = &self.cache[*inner_var];
            let name = info.name.clone();
            let typ = info.typ.as_ref().unwrap().as_monotype();
            let typ = self.follow_all_bindings(typ);

            let value = if i == closure_environment.len() - 1 {
                env.name = Some(name);
                env.clone()
            } else {
                let param_ast: hir::Ast = env.clone().into();
                env_type = Self::extract_second_type(env_type);

                let extract_var_in_env = Self::extract(param_ast.clone(), 0, self.convert_type(&typ));
                let extract_rest_of_env = Self::extract(param_ast, 1, env_type.clone());

                let (definition, definition_id) = self.fresh_definition(extract_var_in_env, Some(name.clone()));
                let (rest_env_def, rest_env_var) = self.fresh_definition(extract_rest_of_env, None);

                definitions.push(definition);
                definitions.push(rest_env_def);

                env = hir::Variable {
                    definition_id: rest_env_var,
                    definition: None,
                    name: None,
                    typ: Rc::new(env_type.clone()),
                };

                let monomorphized_type = Rc::new(self.convert_type(&typ));
                hir::Variable { definition_id, definition: None, name: None, typ: monomorphized_type }
            };

            self.definitions.insert(*inner_var, typ, Definition::Normal(value));
        }

        first_env
    }

    fn pack_closure_environment(&mut self, function: hir::Ast, env: &ClosureEnvironment) -> hir::Ast {
        let env = env
            .iter()
            .map(|(outer_var, (var_id, inner_var, bindings))| {
                // use the type from the inner variable. The outer one may be generalized
                let typ = self.cache[*inner_var].typ.as_ref().unwrap().clone().into_monotype();
                let definition = self.monomorphise_definition_id(*outer_var, *var_id, &typ, bindings);
                definition.reference()
            })
            .collect();

        let env = Self::make_closure_environment(env);
        tuple(vec![function, env])
    }

    fn make_env_type(&mut self, env: &ClosureEnvironment) -> Type {
        if env.is_empty() {
            Type::Primitive(hir::PrimitiveType::Unit)
        } else if env.len() == 1 {
            let inner_var = env.first_key_value().unwrap().1 .1;
            let info = &self.cache[inner_var];
            let typ = info.typ.as_ref().unwrap().as_monotype().clone();
            self.convert_type(&typ)
        } else {
            let mut ids = fmap(env, |(_, (_, id, _))| *id);

            let info = &self.cache[ids.pop().unwrap()];
            let typ = info.typ.as_ref().unwrap().as_monotype().clone();
            let mut typ = self.convert_type(&typ);

            for id in ids.into_iter().rev() {
                let info = &self.cache[id];
                let mtyp = info.typ.as_ref().unwrap().as_monotype().clone();
                typ = Type::Tuple(vec![self.convert_type(&mtyp), typ]);
            }

            typ
        }
    }

    fn extract_second_type(typ: Type) -> Type {
        match typ {
            Type::Tuple(mut elements) => elements.swap_remove(1),
            other => panic!("extract_second_type: expected tuple, found {}", other),
        }
    }

    // This needs to match the packing done in typechecker::infer_closure_environment
    fn make_closure_environment(mut env: VecDeque<hir::Ast>) -> hir::Ast {
        if env.is_empty() {
            unit_literal()
        } else if env.len() == 1 {
            env.pop_back().unwrap()
        } else {
            let first = env.pop_front().unwrap();
            let rest = Self::make_closure_environment(env);
            tuple(vec![first, rest])
        }
    }

    fn convert_type_arg0(&mut self, ptr_type: &types::Type) -> hir::Type {
        match self.follow_all_bindings(ptr_type) {
            types::Type::TypeApplication(_, arg_types) => {
                assert_eq!(arg_types.len(), 1);
                self.convert_type(&arg_types[0])
            },
            _ => unreachable!(),
        }
    }

    fn size_of_type_arg0(&mut self, ptr_type: &types::Type) -> u32 {
        match self.follow_all_bindings(ptr_type) {
            types::Type::TypeApplication(_, arg_types) => {
                assert_eq!(arg_types.len(), 1);
                self.size_of_type(&arg_types[0]) as u32
            },
            _ => unreachable!(),
        }
    }

    fn convert_builtin(&mut self, args: &[ast::Ast<'c>], result_type: &types::Type) -> hir::Ast {
        use hir::Builtin::*;
        let arg = match &args[0] {
            ast::Ast::Literal(ast::Literal { kind: ast::LiteralKind::String(string), .. }) => string,
            _ => unreachable!(),
        };

        let binary = |this: &mut Self, f: fn(Box<hir::Ast>, Box<hir::Ast>) -> hir::Builtin| {
            f(Box::new(this.monomorphise(&args[1])), Box::new(this.monomorphise(&args[2])))
        };

        let cast = |this: &mut Self, f: fn(Box<hir::Ast>, Type) -> hir::Builtin| {
            f(Box::new(this.monomorphise(&args[1])), this.convert_type(result_type))
        };

        hir::Ast::Builtin(match arg.as_ref() {
            "AddInt" => binary(self, AddInt),
            "AddFloat" => binary(self, AddFloat),

            "SubInt" => binary(self, SubInt),
            "SubFloat" => binary(self, SubFloat),

            "MulInt" => binary(self, MulInt),
            "MulFloat" => binary(self, MulFloat),

            "DivSigned" => binary(self, DivSigned),
            "DivUnsigned" => binary(self, DivUnsigned),
            "DivFloat" => binary(self, DivFloat),

            "ModSigned" => binary(self, ModSigned),
            "ModUnsigned" => binary(self, ModUnsigned),
            "ModFloat" => binary(self, ModFloat),

            "LessSigned" => binary(self, LessSigned),
            "LessUnsigned" => binary(self, LessUnsigned),
            "LessFloat" => binary(self, LessFloat),

            "EqInt" => binary(self, EqInt),
            "EqFloat" => binary(self, EqFloat),
            "EqChar" => binary(self, EqChar),
            "EqBool" => binary(self, EqBool),

            "SignExtend" => cast(self, SignExtend),
            "ZeroExtend" => cast(self, ZeroExtend),

            "SignedToFloat" => cast(self, SignedToFloat),
            "UnsignedToFloat" => cast(self, UnsignedToFloat),
            "FloatToSigned" => cast(self, FloatToSigned),
            "FloatToUnsigned" => cast(self, FloatToUnsigned),
            "FloatPromote" => FloatPromote(Box::new(self.monomorphise(&args[1]))),
            "FloatDemote" => FloatDemote(Box::new(self.monomorphise(&args[1]))),

            "BitwiseAnd" => binary(self, BitwiseAnd),
            "BitwiseOr" => binary(self, BitwiseOr),
            "BitwiseXor" => binary(self, BitwiseXor),
            "BitwiseNot" => BitwiseNot(Box::new(self.monomorphise(&args[1]))),

            "Truncate" => cast(self, Truncate),

            "Deref" => cast(self, Deref),
            "Offset" => Offset(
                Box::new(self.monomorphise(&args[1])),
                Box::new(self.monomorphise(&args[2])),
                self.convert_type_arg0(result_type),
            ),
            "Transmute" => cast(self, Transmute),

            // We know the result of SizeOf now, so replace it with a constant
            "SizeOf" => {
                // We expect (size_of : Type t -> usz), so get the size of t
                let size = self.size_of_type_arg0(args[1].get_type().unwrap());
                return int_literal(size as u64, IntegerKind::Usz);
            },

            _ => unreachable!("Unknown builtin '{}'", arg),
        })
    }

    fn monomorphise_call(&mut self, call: &ast::FunctionCall<'c>) -> hir::Ast {
        match call.function.as_ref() {
            ast::Ast::Variable(variable) if variable.definition == Some(BUILTIN_ID) => {
                self.convert_builtin(&call.args, call.typ.as_ref().unwrap())
            },
            _ => {
                // TODO: Code smell: args currently must be monomorphised before the function in case
                // they contain polymorphic integer literals which still need to be defaulted
                // to i32. This can happen if a top-level definition like `a = Some 2` is
                // generalized.
                // TODO: Review this restriction. `a = Some 2` is no longer generalized due to the
                // value restriction.
                let mut args = fmap(&call.args, |arg| self.monomorphise(arg));
                let function = self.monomorphise(&call.function);

                // We could use a new convert_type_shallow here in the future since all we need
                // is to check if it is a tuple type or not
                let function_type = self.convert_type(call.function.get_type().unwrap());

                match function_type {
                    Type::Tuple(mut params) => {
                        // Expect (function, env)
                        assert_eq!(params.len(), 2);

                        let env_type = params.pop().unwrap();

                        let function_type = match params.swap_remove(0) {
                            Type::Function(f) => f,
                            _ => unreachable!(),
                        };

                        // Extract the function from the closure
                        let (function_definition, id) = self.fresh_definition(function, None);
                        let typ = Type::Function(function_type.clone());
                        let function_variable = hir::Ast::Variable(hir::Variable::new(id, Rc::new(typ.clone())));

                        let function = Box::new(Self::extract(function_variable.clone(), 0, typ));
                        let environment = Self::extract(function_variable, 1, env_type);
                        args.push(environment);

                        hir::Ast::Sequence(hir::Sequence {
                            statements: vec![
                                function_definition,
                                hir::Ast::FunctionCall(hir::FunctionCall { function, args, function_type }),
                            ],
                        })
                    },
                    Type::Function(function_type) => {
                        let function = Box::new(function);
                        hir::Ast::FunctionCall(hir::FunctionCall { function, args, function_type })
                    },
                    _ => unreachable!(),
                }
            },
        }
    }

    fn try_get_pattern_name(pattern: &ast::Ast) -> Option<String> {
        match pattern {
            ast::Ast::Variable(var) => Some(var.to_string()),
            ast::Ast::TypeAnnotation(annotation) => Self::try_get_pattern_name(&annotation.lhs),
            _ => None,
        }
    }

    fn monomorphise_definition(&mut self, definition: &ast::Definition<'c>) -> hir::Ast {
        match definition.expr.as_ref() {
            // If the value is a function we can skip it and come back later to only
            // monomorphise it when we know what types it should be instantiated with.
            // TODO: Do we need a check for Variables as well since they can also be generalized?
            ast::Ast::Lambda(_) => unit_literal(),
            _ => {
                let mut expr = self.monomorphise(&definition.expr);
                if definition.mutable {
                    expr = hir::Ast::Builtin(hir::Builtin::StackAlloc(Box::new(expr)));
                }

                let name = Self::try_get_pattern_name(definition.pattern.as_ref());
                let (new_definition, id) = self.fresh_definition(expr, name);

                let mut nested_definitions = vec![new_definition];
                let typ = self.follow_all_bindings(definition.pattern.get_type().unwrap());

                // Used to desugar definitions like `(a, (b, c)) = ...` into
                // id = ...
                // a = extract 0 id
                // fresh = extract 1 id
                // b = extract 0 fresh
                // c = extract 1 fresh
                self.desugar_pattern(&definition.pattern, id, typ, &mut nested_definitions);

                if nested_definitions.len() == 1 {
                    nested_definitions.remove(0)
                } else {
                    hir::Ast::Sequence(hir::Sequence { statements: nested_definitions })
                }
            },
        }
    }

    fn monomorphise_if(&mut self, if_: &ast::If<'c>) -> hir::Ast {
        let condition = Box::new(self.monomorphise(&if_.condition));
        let then = Box::new(self.monomorphise(&if_.then));
        let otherwise = Box::new(self.monomorphise(&if_.otherwise));
        let result_type = self.convert_type(if_.typ.as_ref().unwrap());

        hir::Ast::If(hir::If { condition, then, otherwise, result_type })
    }

    fn monomorphise_return(&mut self, return_: &ast::Return<'c>) -> hir::Ast {
        hir::Ast::Return(hir::Return { expression: Box::new(self.monomorphise(&return_.expression)) })
    }

    fn monomorphise_sequence(&mut self, sequence: &ast::Sequence<'c>) -> hir::Ast {
        let statements = fmap(&sequence.statements, |statement| self.monomorphise(statement));
        hir::Ast::Sequence(hir::Sequence { statements })
    }

    fn get_field_index(&self, field_name: &str, typ: &types::Type) -> u32 {
        use types::Type::*;

        let typ = self.follow_all_bindings(typ);
        match &typ {
            UserDefined(id) => self.cache[*id].find_field(field_name).unwrap().0,
            TypeApplication(typ, args) => {
                match typ.as_ref() {
                    // Pass through ref types transparently
                    types::Type::Ref(..) => self.get_field_index(field_name, &args[0]),
                    // These last 2 cases are the same. They're duplicated to avoid another follow_bindings_shallow call.
                    typ => self.get_field_index(field_name, typ),
                }
            },
            // This case should only happen when a bottom type is unified with an anonymous field
            // type. Default to alphabetically ordered fields, but it should never actually be
            // accessed anyway.
            Struct(fields, _binding) => fields.keys().position(|name| name == field_name).unwrap_or_else(|| {
                panic!("Expected type {} to have a field named '{}'", typ.display(&self.cache), field_name)
            }) as u32,
            _ => unreachable!(
                "get_field_index called with type {} that doesn't have a '{}' field",
                typ.display(&self.cache),
                field_name
            ),
        }
    }

    fn get_field_offset(collection_type: &hir::Type, field_index: u32) -> u32 {
        match collection_type {
            Type::Primitive(_) | Type::Function(_) => unreachable!(),
            Type::Tuple(fields) => fields.iter().map(Self::size_of_monomorphised_type).take(field_index as usize).sum(),
        }
    }

    fn monomorphise_member_access(&mut self, member_access: &ast::MemberAccess<'c>) -> hir::Ast {
        let lhs = self.monomorphise(&member_access.lhs);
        let lhs_type = self.follow_all_bindings(member_access.lhs.get_type().unwrap());
        let index = self.get_field_index(&member_access.field, &lhs_type);

        let ref_type = match lhs_type {
            types::Type::TypeApplication(constructor, args) => match self.follow_bindings_shallow(constructor.as_ref())
            {
                Ok(types::Type::Ref(..)) => Some(self.convert_type(&args[0])),
                _ => None,
            },
            _ => None,
        };

        let result_type = self.convert_type(member_access.typ.as_ref().unwrap());

        // If our collection type is a ref we do a ptr offset instead of a direct access
        match (ref_type, member_access.is_offset) {
            (Some(elem_type), true) => {
                let offset = Self::get_field_offset(&elem_type, index);
                offset_ptr(lhs, offset as u64)
            },
            (Some(elem_type), false) => {
                let lhs = hir::Ast::Builtin(hir::Builtin::Deref(Box::new(lhs), elem_type));
                Self::extract(lhs, index, result_type)
            },
            _ => {
                assert!(!member_access.is_offset);
                Self::extract(lhs, index, result_type)
            },
        }
    }

    fn monomorphise_assignment(&mut self, assignment: &ast::Assignment<'c>) -> hir::Ast {
        let lhs = match self.monomorphise(&assignment.lhs) {
            hir::Ast::Builtin(hir::Builtin::Deref(value, _)) => *value,
            // TODO: Refactor mutability semantics to make this more resiliant
            other => other,
        };

        hir::Ast::Assignment(hir::Assignment { lhs: Box::new(lhs), rhs: Box::new(self.monomorphise(&assignment.rhs)) })
    }

    fn monomorphise_named_constructor(&mut self, constructor: &ast::NamedConstructor<'c>) -> hir::Ast {
        match constructor.sequence.as_ref() {
            ast::Ast::Sequence(sequence) => self.monomorphise_sequence(sequence),
            _ => unreachable!(),
        }
    }

    pub fn extract(ast: hir::Ast, member_index: u32, result_type: Type) -> hir::Ast {
        use hir::{
            Ast,
            Builtin::{Deref, Offset},
        };
        match ast {
            // Try to delay load as long as possible to make valid l-values easier to detect
            Ast::Builtin(Deref(addr, typ)) => {
                let mut elems = match typ {
                    Type::Tuple(elems) => elems,
                    other => unreachable!("Tried to extract from non-tuple type: {}", other),
                };

                let field_type = elems.swap_remove(member_index as usize);

                // The element order was changed by swap_remove above, but we only
                // take the elements that are strictly less than that index
                let offset: u32 = elems
                    .into_iter()
                    .take(member_index as usize)
                    .map(|typ| Self::size_of_monomorphised_type(&typ))
                    .sum();

                if offset == 0 {
                    Ast::Builtin(Deref(addr, field_type))
                } else {
                    let offset_int = Box::new(int_literal(offset as u64, IntegerKind::Usz));
                    let u8 = Type::Primitive(hir::PrimitiveType::Integer(IntegerKind::U8));
                    let offset_ast = Ast::Builtin(Offset(addr, offset_int, u8));
                    Ast::Builtin(Deref(Box::new(offset_ast), field_type))
                }
            },
            other => {
                let lhs = Box::new(other);
                Ast::MemberAccess(hir::MemberAccess { lhs, member_index, typ: result_type })
            },
        }
    }
}

fn tuple(fields: Vec<hir::Ast>) -> hir::Ast {
    hir::Ast::Tuple(hir::Tuple { fields })
}

fn unit_literal() -> hir::Ast {
    hir::Ast::Literal(hir::Literal::Unit)
}

fn int_literal(value: u64, kind: IntegerKind) -> hir::Ast {
    hir::Ast::Literal(hir::Literal::Integer(value, kind))
}

fn tag_value(tag: u8) -> hir::Ast {
    int_literal(tag as u64, IntegerKind::U8)
}

pub fn offset_ptr(addr: hir::Ast, offset: u64) -> hir::Ast {
    let addr = Box::new(addr);
    let offset = int_literal(offset, IntegerKind::Usz);
    let u8 = Type::Primitive(hir::PrimitiveType::Integer(IntegerKind::U8));
    hir::Ast::Builtin(hir::Builtin::Offset(addr, Box::new(offset), u8))
}
