use std::collections::HashMap;
use std::rc::Rc;

use crate::nameresolution::builtin::BUILTIN_ID;
use crate::parser::ast;
use crate::cache::{ModuleCache, VariableId, DefinitionInfoId, DefinitionKind};
use crate::hir as hir;
use crate::types::traits::RequiredImpl;
use crate::types::typechecker::{TypeBindings, self};
use crate::types::typed::Typed;
use crate::types::{ self, TypeVariableId, TypeInfoId };
use crate::util::{fmap, trustme};

use super::types::{IntegerKind, Type, TupleId};

const DEFAULT_INTEGER_KIND: IntegerKind = IntegerKind::I32;

/// The type to bind most typevars to if they are still unbound when we codegen them.
const UNBOUND_TYPE: types::Type = types::Type::Primitive(types::PrimitiveType::UnitType);

/// Monomorphise this ast, simplifying it by removing all generics, traits,
/// and unneeded ast constructs.
pub fn monomorphise<'c>(ast: &ast::Ast<'c>, cache: ModuleCache<'c>) -> hir::Ast {
    let mut context = Context::new(cache);
    context.monomorphise(ast)
}

pub struct Context<'c> {
    monomorphisation_bindings: Vec<TypeBindings>,
    pub cache: ModuleCache<'c>,

    /// Monomorphisation can result in what was 1 DefinitionInfoId being split into
    /// many different monomorphised variants, each represented by a unique hir::DefinitionId.
    pub definitions: HashMap<(DefinitionInfoId, types::Type), hir::DefinitionInfo>,

    types: HashMap<(types::TypeInfoId, Vec<types::Type>), Type>,

    /// Compile-time mapping of variable -> definition for impls that were resolved
    /// after type inference. This is needed for definitions that are polymorphic in
    /// the impls they may use within.
    impl_mappings: HashMap<VariableId, DefinitionInfoId>,

    next_id: usize,
}

impl<'c> Context<'c> {
    fn new(cache: ModuleCache) -> Context {
        Context {
            monomorphisation_bindings: vec![],
            definitions: HashMap::new(),
            types: HashMap::new(),
            impl_mappings: HashMap::new(),
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
            Literal(literal)            => self.monomorphise_literal(literal),
            Variable(variable)          => self.monomorphise_variable(variable),
            Lambda(lambda)              => self.monomorphise_lambda(lambda),
            FunctionCall(call)          => self.monomorphise_call(call),
            Definition(definition)      => self.monomorphise_definition(definition),
            If(if_)                     => self.monomorphise_if(if_),
            Match(match_)               => self.monomorphise_match(match_),
            TypeDefinition(_)           => unit_literal(),
            TypeAnnotation(annotation)  => self.monomorphise(&annotation.lhs),
            Import(_)                   => unit_literal(),
            TraitDefinition(_)          => unit_literal(),
            TraitImpl(_)                => unit_literal(),
            Return(return_)             => self.monomorphise_return(return_),
            Sequence(sequence)          => self.monomorphise_sequence(sequence),
            Extern(_)                   => unit_literal(),
            MemberAccess(member_access) => self.monomorphise_member_access(member_access),
            Assignment(assignment)      => self.monomorphise_assignment(assignment),
        }
    }

    fn find_binding(&self, id: TypeVariableId) -> Option<&types::Type> {
        use types::Type::*;
        use types::TypeBinding::*;

        match &self.cache.type_bindings[id.0] {
            Bound(TypeVariable(id) | Ref(id)) => self.find_binding(*id),
            Bound(binding) => Some(binding),
            Unbound(..) => {
                for bindings in self.monomorphisation_bindings.iter().rev() {
                    if let Some(binding) = bindings.get(&id) {
                        return Some(binding);
                    }
                }
                None
            },
        }
    }

    /// If this type is a type variable, follow what it is bound to
    /// until we find the first type that isn't also a type variable.
    fn follow_bindings_shallow<'a>(&'a self, typ: &'a types::Type) -> &'a types::Type {
        use types::Type::*;

        match typ {
            TypeVariable(id) => self.find_binding(*id).unwrap_or(typ),
            _ => typ
        }
    }

    /// Recursively follow all type variables in this type such that all Bound
    /// type variables are replaced with whatever they are bound to.
    pub fn follow_all_bindings<'a>(&'a self, typ: &'a types::Type) -> types::Type {
        use types::Type::*;

        match typ {
            TypeVariable(id) => {
                match self.find_binding(*id) {
                    Some(binding) => self.follow_all_bindings(binding),
                    None => typ.clone(),
                }
            }
            Primitive(_) => typ.clone(),
            Function(f) => {
                let f = types::FunctionType {
                    parameters: fmap(&f.parameters, |param| self.follow_all_bindings(param)),
                    return_type: Box::new(self.follow_all_bindings(&f.return_type)),
                    environment: Box::new(self.follow_all_bindings(&f.environment)),
                    is_varargs: f.is_varargs,
                };
                Function(f)
            },
            UserDefined(_) => typ.clone(),
            TypeApplication(con, args) => {
                let con = self.follow_all_bindings(con);
                let args = fmap(args, |arg| self.follow_all_bindings(arg));
                TypeApplication(Box::new(con), args)
            },
            Ref(_) => typ.clone(),
            ForAll(_, t) => self.follow_all_bindings(t),
        }
    }

    fn size_of_struct_type(&mut self, info: &types::TypeInfo, fields: &[types::Field], args: &[types::Type]) -> usize {
        let bindings = typechecker::type_application_bindings(info, args);

        fields
            .iter()
            .map(|field| {
                let field_type = typechecker::bind_typevars(&field.field_type, &bindings, &self.cache);
                self.size_of_type(&field_type)
            })
            .sum()
    }

    fn size_of_union_type(&mut self, info: &types::TypeInfo, variants: &[types::TypeConstructor<'c>], args: &[types::Type]) -> usize {
        let bindings = typechecker::type_application_bindings(info, args);

        match self.find_largest_union_variant(variants, &bindings) {
            None => 0, // Void type
            Some(variant) => {
                // The size of a union is the size of its largest field, plus 1 byte for the tag
                variant
                    .iter()
                    .map(|field| self.size_of_type(field))
                    .sum::<usize>()
                    + 1
            },
        }
    }

    fn size_of_user_defined_type(&mut self, id: TypeInfoId, args: &[types::Type]) -> usize {
        let info = &self.cache[id];
        assert!(
            info.args.len() == args.len(),
            "Kind error during llvm code generation"
        );

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
            Primitive(IntegerType(kind)) => self.integer_bit_count(*kind) as usize / 8,
            Primitive(FloatType) => 8,
            Primitive(CharType) => 1,
            Primitive(BooleanType) => 1,
            Primitive(UnitType) => 1,
            Primitive(Ptr) => Self::ptr_size(),

            Function(..) => Self::ptr_size(),

            TypeVariable(id) => {
                let binding = self.find_binding(*id).unwrap_or(&UNBOUND_TYPE).clone();
                self.size_of_type(&binding)
            },

            UserDefined(id) => self.size_of_user_defined_type(*id, &[]),

            TypeApplication(typ, args) => match typ.as_ref() {
                UserDefined(id) => self.size_of_user_defined_type(*id, args),
                _ => unreachable!("Kind error inside size_of_type"),
            },

            Ref(_) => Self::ptr_size(),

            ForAll(_, typ) => self.size_of_type(typ),
        }
    }

    fn convert_primitive_type(&mut self, typ: &types::PrimitiveType) -> Type {
        use types::PrimitiveType::*;
        Type::Primitive(match typ {
            IntegerType(kind) => {
                let kind = self.convert_integer_kind(*kind);
                hir::types::PrimitiveType::IntegerType(kind)
            },
            FloatType => hir::types::PrimitiveType::FloatType,
            CharType => hir::types::PrimitiveType::CharType,
            BooleanType => hir::types::PrimitiveType::BooleanType,
            UnitType => hir::types::PrimitiveType::UnitType,
            Ptr => unreachable!("Kind error during monomorphisation"),
        })
    }

    fn convert_struct_type(&mut self, id: TypeInfoId, info: &types::TypeInfo, fields: &[types::Field<'c>], args: Vec<types::Type>) -> Type {
        let bindings = typechecker::type_application_bindings(info, &args);

        let tuple_id = Some(TupleId(self.types.len()));
        let t = Type::Tuple(tuple_id, vec![]);
        self.types.insert((id, args.clone()), t);

        let fields = fmap(fields, |field| {
            let field_type = typechecker::bind_typevars(&field.field_type, &bindings, &self.cache);
            self.convert_type(&field_type)
        });

        let t = Type::Tuple(tuple_id, fields);
        self.types.insert((id, args), t.clone());
        t
    }

    /// Given a list of TypeConstructors representing each variant of a sum type,
    /// find the largest variant in memory (with the given type bindings for any type variables)
    /// and return its field types.
    fn find_largest_union_variant(&mut self, variants: &[types::TypeConstructor<'c>], bindings: &TypeBindings) -> Option<Vec<types::Type>> {
        let variants: Vec<Vec<types::Type>> = fmap(variants, |variant| {
            fmap(&variant.args, |arg| {
                typechecker::bind_typevars(arg, bindings, &self.cache)
            })
        });

        variants.into_iter().max_by_key(|variant| {
            variant
                .iter()
                .map(|arg| self.size_of_type(arg))
                .sum::<usize>()
        })
    }

    /// Returns the type of a tag in an unoptimized tagged union
    pub fn tag_type() -> Type {
        Type::Primitive(hir::types::PrimitiveType::IntegerType(IntegerKind::U8))
    }

    fn convert_union_type(
        &mut self, id: TypeInfoId, info: &types::TypeInfo, variants: &[types::TypeConstructor<'c>],
        args: Vec<types::Type>,
    ) -> Type {
        let bindings = typechecker::type_application_bindings(info, &args);

        let tuple_id = Some(TupleId(self.types.len()));
        let mut t = Type::Tuple(tuple_id, vec![]);

        if let Some(variant) = self.find_largest_union_variant(variants, &bindings) {
            self.types.insert((id, args.clone()), t);

            let mut fields = vec![Self::tag_type()];
            for typ in variant {
                fields.push(self.convert_type(&typ));
            }

            t = Type::Tuple(tuple_id, fields);
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
        self.follow_bindings_shallow(environment).is_unit(&self.cache)
    }

    /// Monomorphise a types::Type into a hir::Type with no generics.
    pub fn convert_type(&mut self, typ: &types::Type) -> Type {
        use types::PrimitiveType::Ptr;
        use types::Type::*;

        match typ {
            Primitive(primitive) => self.convert_primitive_type(primitive),

            Function(function) => {
                let mut parameters = fmap(&function.parameters, |typ| {
                    self.convert_type(typ).into()
                });

                let return_type = Box::new(self.convert_type(&function.return_type));

                let environment = (!self.empty_closure_environment(&function.environment)).then(|| {
                    let environment_parameter = self.convert_type(&function.environment);
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
                    Some(environment) => Type::Tuple(None, vec![function, environment]),
                }
            },

            TypeVariable(id) => {
                self.convert_type(&self.find_binding(*id).unwrap_or(&UNBOUND_TYPE).clone())
            },

            UserDefined(id) => self.convert_user_defined_type(*id, vec![]),

            TypeApplication(typ, args) => {
                let args = fmap(args, |arg| self.follow_all_bindings(arg));
                let typ = self.follow_bindings_shallow(typ);

                match typ {
                    Primitive(Ptr) | Ref(_) => {
                        assert_eq!(args.len(), 1);
                        let elem = self.convert_type(&args[0]);
                        Type::Pointer(Box::new(elem))
                    },
                    UserDefined(id) => {
                        let id = *id;
                        self.convert_user_defined_type(id, args)
                    }
                    _ => {
                        unreachable!(
                            "Type {} requires 0 type args but was applied to {:?}",
                            typ.display(&self.cache),
                            args
                        );
                    },
                }
            },

            Ref(_) => {
                unreachable!("Kind error during monomorphisation. Attempted to translate a `ref` without a type argument")
            },

            ForAll(_, typ) => self.convert_type(typ),
        }
    }

    fn convert_integer_kind(&self, kind: crate::lexer::token::IntegerKind) -> IntegerKind {
        use crate::lexer::token::IntegerKind;
        match kind {
            IntegerKind::Unknown => DEFAULT_INTEGER_KIND,
            IntegerKind::Inferred(id) => {
                use types::Type::*;
                use types::PrimitiveType;

                match self.find_binding(id) {
                    Some(Primitive(PrimitiveType::IntegerType(kind))) => self.convert_integer_kind(*kind),
                    None => DEFAULT_INTEGER_KIND,
                    Some(other) => unreachable!("convert_integer_kind called with non-integer type {}", other.display(&self.cache)),
                }
            },
            IntegerKind::I8 =>  hir::IntegerKind::I8,
            IntegerKind::I16 => hir::IntegerKind::I16,
            IntegerKind::I32 => hir::IntegerKind::I32,
            IntegerKind::I64 => hir::IntegerKind::I64,
            IntegerKind::Isz => hir::IntegerKind::Isz,
            IntegerKind::U8 =>  hir::IntegerKind::U8,
            IntegerKind::U16 => hir::IntegerKind::U16,
            IntegerKind::U32 => hir::IntegerKind::U32,
            IntegerKind::U64 => hir::IntegerKind::U64,
            IntegerKind::Usz => hir::IntegerKind::Usz,
        }
    }

    fn monomorphise_literal(&mut self, literal: &ast::Literal) -> hir::Ast {
        use hir::Ast::*;
        use hir::Literal::*;

        match &literal.kind {
            ast::LiteralKind::Integer(n, kind) => {
                let kind = self.convert_integer_kind(*kind);
                Literal(Integer(*n, kind))
            },
            ast::LiteralKind::Float(f) => Literal(Float(*f)),
            ast::LiteralKind::String(s) => {
                let len = Literal(Integer(s.len() as u64, IntegerKind::Usz));
                let c_string = Literal(CString(s.clone()));

                Tuple(hir::Tuple {
                    fields: vec![c_string, len],
                })
            },
            ast::LiteralKind::Char(c) => Literal(Char(*c)),
            ast::LiteralKind::Bool(b) => Literal(Bool(*b)),
            ast::LiteralKind::Unit => unit_literal(),
        }
    }

    fn add_required_impls(&mut self, required_impls: &[RequiredImpl]) {
        for required_impl in required_impls {
            // TODO: This assert is failing in builtin_int for some reason.
            // It may be the case that this assert was wrong to begin with and
            // there _should_ be multiple bindings for a given origin.
            // assert!(!self.impl_mappings.contains_key(&required_impl.origin), "impl_mappings already had a mapping for {:?}", required_impl.origin);
            self.impl_mappings
                .insert(required_impl.origin, required_impl.binding);
        }
    }

    fn remove_required_impls(&mut self, required_impls: &[RequiredImpl]) {
        for required_impl in required_impls {
            self.impl_mappings.remove(&required_impl.origin);
        }
    }

    /// Get the DefinitionInfoId this variable should point to. This is usually
    /// given by variable.definition but in the case of static trait dispatch,
    /// self.impl_mappings may be set to bind a given variable id to another
    /// definition. This is currently only done for trait functions/values to
    /// point them to impls that actually have definitions.
    fn get_definition_id(&self, variable: &ast::Variable<'c>) -> DefinitionInfoId {
        self.impl_mappings
            .get(&variable.id.unwrap())
            .copied()
            .unwrap_or_else(|| variable.definition.unwrap())
    }

    fn monomorphise_variable(&mut self, variable: &ast::Variable<'c>) -> hir::Ast {
        let required_impls = self.cache[variable.trait_binding.unwrap()]
            .required_impls
            .clone();

        self.add_required_impls(&required_impls);

        // The definition to compile is either the corresponding impl definition if this
        // variable refers to a trait function, or otherwise it is the regular definition of this variable.
        let id = self.get_definition_id(&variable);

        let value = self.monomorphise_definition_id(id, variable.typ.as_ref().unwrap());

        self.remove_required_impls(&required_impls);
        hir::Ast::Variable(value)
    }

    pub fn lookup_definition(&self, id: DefinitionInfoId, typ: &types::Type) -> Option<hir::DefinitionInfo> {
        let typ = self.follow_all_bindings(typ);
        self.definitions.get(&(id, typ)).cloned()
    }

    fn monomorphise_definition_id(&mut self, id: DefinitionInfoId, typ: &types::Type) -> hir::DefinitionInfo {
        if let Some(value) = self.lookup_definition(id, &typ) {
            return value;
        }

        let definition = trustme::extend_lifetime(&mut self.cache[id]);
        let definition_type = remove_forall(definition.typ.as_ref().unwrap());

        let typ = self.follow_all_bindings(typ);

        let bindings = typechecker::try_unify(&typ, definition_type, definition.location, &mut self.cache)
            .map_err(|error| eprintln!("{}", error))
            .expect("Unification error during monomorphisation");

        self.monomorphisation_bindings.push(bindings);

        // Compile the definition with the bindings in scope. Each definition is expected to
        // add itself to Generator.definitions
        let value = match &definition.definition {
            Some(DefinitionKind::Definition(definition)) => {
                // Any recursive calls to this variable will refer to this binding
                let definition_id = self.next_unique_id();
                let info = hir::DefinitionInfo { definition: None, definition_id };
                self.definitions.insert((id, typ), info);

                self.monomorphise_nonlocal_definition(definition, definition_id)
            },
            Some(DefinitionKind::Extern(_)) => self.make_extern(id, &typ),
            Some(DefinitionKind::TypeConstructor { tag, name: _ }) => {
                let definition = self.monomorphise_type_constructor(tag, &typ);
                self.define(definition, id, typ)
            },
            Some(DefinitionKind::TraitDefinition(_)) => {
                unreachable!("Cannot monomorphise from a TraitDefinition.\nNo cached impl for {} {}: {}", definition.name, id.0, typ.display(&self.cache))
            },
            Some(DefinitionKind::Parameter) => {
                unreachable!("Parameters should already be defined.\nEncountered while compiling {} {}: {}", definition.name, id.0, typ.display(&self.cache))
            },
            Some(DefinitionKind::MatchPattern) => {
                unreachable!("MatchPatterns should already be defined.\n Encountered while compiling {} {}: {}", definition.name, id.0, typ.display(&self.cache))
            },
            None => unreachable!("No definition for {} {}", definition.name, id.0),
        };

        self.monomorphisation_bindings.pop();
        value
    }

    /// This function is 'make_extern' rathern than 'monomorphise_extern' since extern declarations
    /// shouldn't be monomorphised across multiple types.
    fn make_extern(&mut self, id: DefinitionInfoId, typ: &types::Type) -> hir::DefinitionInfo {
        // extern definitions should only be declared once - never duplicated & monomorphised.
        // For this reason their value is always stored with the Unit type in the definitions map.
        if let Some(value) = self.lookup_definition(id, &UNBOUND_TYPE).clone() {
            self.definitions.insert((id, typ.clone()), value.clone());
            return value;
        }

        let extern_ = hir::Ast::Extern(hir::Extern {
            name: self.cache[id].name.clone(),
            typ: self.convert_type(typ),
        });

        let mutable = self.cache[id].mutable;
        let definition = self.make_definition(extern_, mutable);

        // Insert the global for both the current type and the unit type
        self.definitions.insert((id, typ.clone()), definition.clone());
        self.definitions.insert((id, UNBOUND_TYPE.clone()), definition.clone());
        definition
    }

    /// Wrap the given Ast in a new DefinitionInfo and store it
    fn define(&mut self, definition_rhs: hir::Ast, original_id: DefinitionInfoId, typ: types::Type) -> hir::DefinitionInfo {
        let definition = hir::Definition {
            variable: self.next_unique_id(),
            expr: Box::new(definition_rhs),
            mutable: false,
        };

        let info = hir::DefinitionInfo::from(definition);
        self.definitions.insert((original_id, typ), info.clone());
        info
    }

    fn fresh_variable(&mut self) -> hir::Variable {
        hir::Variable {
            definition: None,
            definition_id: self.next_unique_id(),
        }
    }

    pub fn fresh_definition(&mut self, definition_rhs: hir::Ast, mutable: bool) -> (hir::Ast, hir::DefinitionId) {
        let variable = self.next_unique_id();
        let definition = hir::Ast::Definition(hir::Definition {
            variable,
            expr: Box::new(definition_rhs),
            mutable,
        });
        (definition, variable)
    }

    fn make_definition(&mut self, definition_rhs: hir::Ast, mutable: bool) -> hir::DefinitionInfo {
        let (definition, definition_id) = self.fresh_definition(definition_rhs, mutable);
        hir::DefinitionInfo {
            definition_id,
            definition: Some(Rc::new(definition))
        }
    }

    /// Monomorphise a definition defined elsewhere
    ///
    /// TODO: This may be a clone of monomorphise_definition now
    fn monomorphise_nonlocal_definition(&mut self, definition: &ast::Definition<'c>,
        definition_id: hir::DefinitionId) -> hir::DefinitionInfo
    {
        let value = self.monomorphise(&*definition.expr);
        let new_definition = hir::Ast::Definition(hir::Definition {
            variable: definition_id,
            expr: Box::new(value),
            mutable: definition.mutable,
        });

        let mut nested_definitions = vec![new_definition];
        let typ = self.follow_all_bindings(definition.pattern.get_type().unwrap());
        self.desugar_pattern(&definition.pattern, definition_id, typ, &mut nested_definitions);

        let definition = if nested_definitions.len() == 1 {
            nested_definitions.remove(0)
        } else {
            hir::Ast::Sequence(hir::Sequence { statements: nested_definitions })
        };

        hir::Variable { definition_id, definition: Some(Rc::new(definition)) }
    }

    /// Simplifies a pattern and expression like `(a, b) = foo ()`
    /// into multiple successive bindings:
    /// ```
    /// new_var = foo ()
    /// a = extract 0 new_var
    /// b = extract 1 new_var
    /// ```
    /// This function will not add the new variables into self.definitions
    /// as they should not be able to be referenced externally - unlike `a` and `b` above.
    fn desugar_pattern(&mut self, pattern: &ast::Ast<'c>, definition_id: hir::DefinitionId,
        typ: types::Type, definitions: &mut Vec<hir::Ast>)
    {
        use {ast::LiteralKind, ast::Ast::*};

        // Sanity check the expected type exactly matches that of the actual pattern
        let pattern_type = pattern.get_type().unwrap();
        let pattern_type = self.follow_all_bindings(pattern_type);
        assert_eq!(pattern_type, typ);

        match pattern {
            Literal(literal) => assert_eq!(literal.kind, LiteralKind::Unit),
            Variable(variable_pattern) => {
                let id = variable_pattern.definition.unwrap();

                let variable = hir::Variable { definition_id, definition: None };
                self.definitions.insert((id, typ), variable);
            },
            TypeAnnotation(annotation) => {
                self.desugar_pattern(annotation.lhs.as_ref(), definition_id, typ, definitions)
            },
            // Match a pair pattern
            FunctionCall(call) if call.is_pair_constructor() => {
                let variable = hir::Variable { definition_id, definition: None };

                for (i, arg_pattern) in call.args.iter().enumerate() {
                    let extract = extract(variable.clone().into(), i as u32);
                    let (definition, id) = self.fresh_definition(extract, false);
                    definitions.push(definition);

                    // Sanity check the expected type exactly matches that of the actual variable
                    let arg_type = self.follow_all_bindings(arg_pattern.get_type().unwrap());
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
                let args = fmap(&function_type.parameters, |_| self.fresh_variable());

                let mut tuple_args = Vec::with_capacity(args.len() + 1);
                let mut tuple_size = function_type.parameters.iter()
                    .map(|parameter| self.size_of_monomorphised_type(parameter))
                    .sum();

                if let Some(tag) = tag {
                    tuple_args.push(tag_value(*tag));
                    tuple_size += self.size_of_monomorphised_type(&Self::tag_type());
                }

                tuple_args.extend(args.iter().map(|arg| arg.clone().into()));

                let tuple = hir::Ast::Tuple(hir::Tuple { fields: tuple_args });

                let body = match tag {
                    None => tuple,
                    Some(_) => {
                        let target_type = function_type.return_type.as_ref().clone();
                        self.make_reinterpret_cast(tuple, tuple_size, target_type)
                    },
                };

                hir::Ast::Lambda(hir::Lambda {
                    args,
                    body: Box::new(body),
                    typ: function_type,
                })
            },
            // Since this is not a function type, we know it has no bundled data and we can
            // thus ignore the additional type arguments, extract the tag value, and
            // reinterpret_cast to the appropriate type.
            Tuple(..) => {
                match tag {
                    None => unit_literal(),
                    Some(tag) => {
                        let value = tag_value(*tag);
                        let size = self.size_of_monomorphised_type(&Self::tag_type());
                        self.make_reinterpret_cast(value, size, typ)
                    }
                }
            },
            Primitive(_) | Pointer(_) => unreachable!("Type constructor must be a Function or Tuple type: {}", typ),
        }
    }

    /// Create a reinterpret_cast instruction for the given Ast value.
    /// arg_type_size is the size of the value represented by the given ast, in bytes.
    fn make_reinterpret_cast(&mut self, ast: hir::Ast, mut arg_type_size: u32, target_type: Type) -> hir::Ast {
        let target_size = self.size_of_monomorphised_type(&target_type);
        assert!(arg_type_size <= target_size);

        if arg_type_size == target_size {
            return hir::Ast::ReinterpretCast(hir::ReinterpretCast {
                lhs: Box::new(ast),
                target_type,
            });
        }

        let mut padded = vec![ast];
        let type_tower = [
            (IntegerKind::U64, 8),
            (IntegerKind::U32, 4),
            (IntegerKind::U16, 2),
            (IntegerKind::U8, 1),
        ];

        for (int_kind, size) in type_tower {
            while arg_type_size + size <= target_size {
                padded.push(hir::Ast::Literal(hir::Literal::Integer(0, int_kind)));
                arg_type_size += size;
            }
        }

        hir::Ast::ReinterpretCast(hir::ReinterpretCast {
            lhs: Box::new(self.tuple(padded)),
            target_type,
        })
    }

    fn size_of_monomorphised_type(&self, typ: &Type) -> u32 {
        match typ {
            Type::Primitive(p) => {
                match p {
                    hir::types::PrimitiveType::IntegerType(kind) => {
                        use IntegerKind::*;
                        match kind {
                            I8 | U8 => 1,
                            I16 | U16 => 2,
                            I32 | U32 => 4,
                            I64 | U64 => 8,
                            Isz | Usz => Self::ptr_size() as u32,
                        }
                    },
                    hir::types::PrimitiveType::FloatType => 8,
                    hir::types::PrimitiveType::CharType => 1,
                    hir::types::PrimitiveType::BooleanType => 1,
                    hir::types::PrimitiveType::UnitType => 1, // TODO: this can depend on the backend
                }
            },
            Type::Function(_) => Self::ptr_size() as u32, // Closures would be represented as tuples
            Type::Pointer(_) => Self::ptr_size() as u32,
            Type::Tuple(_, fields) => {
                fields.iter().map(|f| self.size_of_monomorphised_type(f)).sum()
            },
        }
    }

    fn monomorphise_lambda(&mut self, lambda: &ast::Lambda<'c>) -> hir::Ast {
        let typ = match self.convert_type(lambda.typ.as_ref().unwrap()) {
            Type::Function(f) => f,
            other => unreachable!("Lambda has a non-function type: {}", other),
        };

        let mut body_prelude = vec![];

        // Bind each parameter node to the nth parameter of `function`
        // This will also desugar any patterns in the parameter, prepending extra
        // statements to the function body to extract the relevant fields.
        let mut args = fmap(&lambda.args, |arg| {
            let typ = self.follow_all_bindings(arg.get_type().unwrap());
            let param = self.fresh_variable();
            self.desugar_pattern(arg, param.definition_id, typ, &mut body_prelude);
            param
        });

        args.extend(lambda.closure_environment.values().map(|value| {
            let param = self.fresh_variable();
            let typ = self.cache[*value].typ.as_ref().unwrap();
            let typ = self.follow_all_bindings(typ);
            self.definitions.insert((*value, typ), param.clone());

            param.into()
        }));

        let body = self.monomorphise(&lambda.body);

        let body = Box::new(if body_prelude.is_empty() { body } else {
            body_prelude.push(body);
            hir::Ast::Sequence(hir::Sequence { statements: body_prelude })
        });

        let function = hir::Ast::Lambda(hir::Lambda { args, body, typ });

        if lambda.closure_environment.is_empty() {
            function
        } else {
            let mut values = Vec::with_capacity(lambda.closure_environment.len() + 1);
            values.push(function);

            for key in lambda.closure_environment.keys() {
                let typ = self.cache[*key].typ.as_ref().unwrap().clone();
                let definition = self.monomorphise_definition_id(*key, &typ);
                values.push(hir::Ast::Variable(definition));
            }

            self.tuple(values)
        }
    }

    fn tuple(&self, fields: Vec<hir::Ast>) -> hir::Ast {
        hir::Ast::Tuple(hir::Tuple { fields })
    }

    fn convert_builtin(&mut self, args: &[ast::Ast<'c>]) -> hir::Ast {
        assert!(args.len() == 1);
    
        let arg = match &args[0] {
            ast::Ast::Literal(ast::Literal { kind: ast::LiteralKind::String(string), .. }) => string,
            _ => unreachable!(),
        };

        use hir::Builtin::*;
        hir::Ast::Builtin(match arg.as_ref() {
            "AddInt" => AddInt,
            "AddFloat" => AddFloat,
    
            "SubInt" => SubInt,
            "SubFloat" => SubFloat,
    
            "MulInt" => MulInt,
            "MulFloat" => MulFloat,
    
            "DivInt" => DivInt,
            "DivFloat" => DivFloat,
    
            "ModInt" => ModInt,
            "ModFloat" => ModFloat,
    
            "LessInt" => LessInt,
            "LessFloat" => LessFloat,
    
            "GreaterInt" => GreaterInt,
            "GreaterFloat" => GreaterFloat,
    
            "EqInt" => EqInt,
            "EqFloat" => EqFloat,
            "EqChar" => EqChar,
            "EqBool" => EqBool,
    
            "sign_extend" => SignExtend,
            "zero_extend" => ZeroExtend,
    
            "truncate" => Truncate,
    
            "deref" => Deref,
            "offset" => Offset,
            "transmute" => Transmute,
    
            _ => unreachable!("Unknown builtin '{}'", arg),
        })
    }

    fn monomorphise_call(&mut self, call: &ast::FunctionCall<'c>) -> hir::Ast {
        match call.function.as_ref() {
            ast::Ast::Variable(variable) if variable.definition == Some(BUILTIN_ID) => {
                self.convert_builtin(&call.args)
            },
            _ => {
                // TODO: Code smell: args currently must be monomorphised before the function in case
                // they contain polymorphic integer literals which still need to be defaulted
                // to i32. This can happen if a top-level definition like `a = Some 2` is
                // generalized.
                let args = fmap(&call.args, |arg| self.monomorphise(arg));
                let function = self.monomorphise(&call.function);

                // We could use a new convert_type_shallow here in the future since all we need
                // is to check if it is a tuple type or not
                let function_type = self.convert_type(call.function.get_type().unwrap());

                let function = if matches!(function_type, Type::Tuple(..)) {
                    // Extract the function from the closure
                    hir::Ast::MemberAccess(hir::MemberAccess {
                        lhs: Box::new(function),
                        member_index: 0,
                    })
                } else {
                    function
                };

                let function = Box::new(function);
                hir::Ast::FunctionCall(hir::FunctionCall { function, args })
            },
        }
    }

    fn monomorphise_definition(&mut self, definition: &ast::Definition<'c>) -> hir::Ast {
        match definition.expr.as_ref() {
            // If the value is a function we can skip it and come back later to only
            // monomorphise it when we know what types it should be instantiated with.
            ast::Ast::Lambda(_) => unit_literal(),
            _ => {
                let expr = self.monomorphise(&definition.expr);
                let (new_definition, id) = self.fresh_definition(expr, definition.mutable);

                // Used to desugar definitions like `(a, (b, c)) = ...` into
                // id = ...
                // a = extract 0 id
                // fresh = extract 1 id
                // b = extract 0 fresh
                // c = extract 1 fresh
                let mut nested_definitions = vec![new_definition];
                let typ = self.follow_all_bindings(definition.pattern.get_type().unwrap());
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
        let otherwise = if_.otherwise.as_ref().map(|e| Box::new(self.monomorphise(&e)));

        hir::Ast::If(hir::If {
            condition,
            then,
            otherwise,
        })
    }

    fn monomorphise_return(&mut self, return_: &ast::Return<'c>) -> hir::Ast {
        hir::Ast::Return(hir::Return { 
            expression: Box::new(self.monomorphise(&return_.expression)),
        })
    }

    fn monomorphise_sequence(&mut self, sequence: &ast::Sequence<'c>) -> hir::Ast {
        let statements = fmap(&sequence.statements, |statement| self.monomorphise(statement));
        hir::Ast::Sequence(hir::Sequence { statements })
    }

    fn get_field_index(&self, field_name: &str, typ: &types::Type) -> u32 {
        use types::Type::*;

        match self.follow_bindings_shallow(typ) {
            UserDefined(id) => {
                self.cache[*id].find_field(field_name).unwrap().0
            }
            _ => unreachable!("get_field_index called with type {} that doesn't have a '{}' field", typ.display(&self.cache), field_name),
        }
    }

    fn monomorphise_member_access(&mut self, member_access: &ast::MemberAccess<'c>) -> hir::Ast {
        let index = self.get_field_index(&member_access.field, member_access.lhs.get_type().unwrap());
        let lhs = self.monomorphise(&member_access.lhs);
        extract(lhs, index)
    }

    fn monomorphise_assignment(&mut self, assignment: &ast::Assignment<'c>) -> hir::Ast {
        hir::Ast::Assignment(hir::Assignment {
            lhs: Box::new(self.monomorphise(&assignment.lhs)),
            rhs: Box::new(self.monomorphise(&assignment.rhs)),
        })
    }
}

fn unit_literal() -> hir::Ast {
    hir::Ast::Literal(hir::Literal::Unit)
}

fn remove_forall(typ: &types::Type) -> &types::Type {
    match typ {
        types::Type::ForAll(_, t) => t,
        _ => typ,
    }
}

fn tag_value(tag: u8) -> hir::Ast {
    let kind = IntegerKind::U8;
    hir::Ast::Literal(hir::Literal::Integer(tag as u64, kind))
}

pub fn extract(ast: hir::Ast, index: u32) -> hir::Ast {
    hir::Ast::MemberAccess(hir::MemberAccess {
        lhs: Box::new(ast),
        member_index: index,
    })
}
