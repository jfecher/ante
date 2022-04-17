use std::collections::HashMap;

use crate::parser::ast;
use crate::cache::{ModuleCache, VariableId, DefinitionInfoId};
use crate::hir as hir;
use crate::types::typechecker::TypeBindings;
use crate::types::typed::Typed;
use crate::types::{ self, TypeVariableId, TypeInfoId };
use crate::util::fmap;

use super::types::{IntegerKind, Type, TupleId};

const DEFAULT_INTEGER_KIND: IntegerKind = IntegerKind::I32;

/// The type to bind most typevars to if they are still unbound when we codegen them.
const UNBOUND_TYPE: types::Type = types::Type::Primitive(types::PrimitiveType::UnitType);

/// Monomorphise this ast, simplifying it by removing all generics, traits,
/// and unneeded ast constructs.
pub fn monomorphise<'c>(ast: ast::Ast<'c>, cache: ModuleCache<'c>) -> hir::Ast {
    let mut context = Context::new(cache);
    context.monomorphise(ast)
}

struct Context<'c> {
    monomorphisation_bindings: Vec<TypeBindings>,
    cache: ModuleCache<'c>,

    definitions: HashMap<(DefinitionInfoId, types::Type), hir::Ast>,

    types: HashMap<(types::TypeInfoId, Vec<types::Type>), Type>,

    /// Compile-time mapping of variable -> definition for impls that were resolved
    /// after type inference. This is needed for definitions that are polymorphic in
    /// the impls they may use within.
    impl_mappings: HashMap<VariableId, DefinitionInfoId>,
}

fn unit_literal() -> hir::Ast {
    hir::Ast::Literal(hir::Literal::Unit)
}

impl<'c> Context<'c> {
    fn new(cache: ModuleCache) -> Context {
        Context {
            monomorphisation_bindings: vec![],
            definitions: HashMap::new(),
            types: HashMap::new(),
            impl_mappings: HashMap::new(),
            cache,
        }
    }

    fn monomorphise(&mut self, ast: ast::Ast<'c>) -> hir::Ast {
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
            TypeAnnotation(annotation)  => self.monomorphise(*annotation.lhs),
            Import(_)                   => unit_literal(),
            TraitDefinition(_)          => unit_literal(),
            TraitImpl(_)                => unit_literal(),
            Return(return_)             => self.monomorphise_return(return_),
            Sequence(sequence)          => self.monomorphise_sequence(sequence),
            Extern(extern_)             => self.monomorphise_extern(extern_),
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
    fn follow_all_bindings<'a>(&'a self, typ: &'a types::Type) -> types::Type {
        use types::Type::*;

        match typ {
            TypeVariable(id) => self.find_binding(*id).unwrap_or(typ).clone(),
            Primitive(p) => typ.clone(),
            Function(f) => {
                let f = types::FunctionType {
                    parameters: fmap(&f.parameters, |param| self.follow_all_bindings(param)),
                    return_type: Box::new(self.follow_all_bindings(&f.return_type)),
                    environment: Box::new(self.follow_all_bindings(&f.environment)),
                    is_varargs: f.is_varargs,
                };
                Function(f)
            },
            UserDefined(id) => typ.clone(),
            TypeApplication(con, args) => {
                let con = self.follow_all_bindings(con);
                let args = fmap(args, |arg| self.follow_all_bindings(arg));
                TypeApplication(Box::new(con), args)
            },
            Ref(r) => typ.clone(),
            ForAll(_, t) => self.follow_all_bindings(t),
        }
    }

    fn size_of_struct_type(&mut self, info: &types::TypeInfo, fields: &[types::Field], args: &[types::Type]) -> usize {
        let bindings = types::typechecker::type_application_bindings(info, args);

        fields
            .iter()
            .map(|field| {
                let field_type = types::typechecker::bind_typevars(&field.field_type, &bindings, &self.cache);
                self.size_of_type(&field_type)
            })
            .sum()
    }

    fn size_of_union_type(&mut self, info: &types::TypeInfo, variants: &[types::TypeConstructor<'c>], args: &[types::Type]) -> usize {
        let bindings = types::typechecker::type_application_bindings(info, args);

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
        let info = &self.cache.type_infos[id.0];
        assert!(
            info.args.len() == args.len(),
            "Kind error during llvm code generation"
        );

        use types::TypeInfoBody::*;
        match &info.body {
            Union(variants) => self.size_of_union_type(info, variants, args),
            Struct(fields) => self.size_of_struct_type(info, fields, args),

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
        let bindings = types::typechecker::type_application_bindings(info, &args);

        let tuple_id = TupleId(self.types.len());
        let t = Type::Tuple(tuple_id, vec![]);
        self.types.insert((id, args.clone()), t);

        let fields = fmap(fields, |field| {
            let field_type = types::typechecker::bind_typevars(&field.field_type, &bindings, &self.cache);
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
                types::typechecker::bind_typevars(arg, bindings, &self.cache)
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
    fn tag_type(&self) -> Type {
        Type::Primitive(hir::types::PrimitiveType::IntegerType(IntegerKind::U8))
    }

    fn convert_union_type(
        &mut self, id: TypeInfoId, info: &types::TypeInfo, variants: &[types::TypeConstructor<'c>],
        args: Vec<types::Type>,
    ) -> Type {
        let bindings = types::typechecker::type_application_bindings(info, &args);

        let tuple_id = TupleId(self.types.len());
        let t = Type::Tuple(tuple_id, vec![]);
        self.types.insert((id, args), t);

        if let Some(variant) = self.find_largest_union_variant(variants, &bindings) {
            let mut fields = vec![self.tag_type()];
            for typ in variant {
                fields.push(self.convert_type(&typ));
            }

            let t = Type::Tuple(tuple_id, fields);
            self.types.insert((id, args), t);
        }

        t
    }

    fn convert_user_defined_type(&mut self, id: TypeInfoId, args: Vec<types::Type>) -> Type {
        let info = &self.cache.type_infos[id.0];
        assert!(info.args.len() == args.len(), "Kind error during monomorphisation");

        if let Some(typ) = self.types.get(&(id, args.clone())) {
            return *typ;
        }

        use types::TypeInfoBody::*;
        let typ = match &info.body {
            Union(variants) => self.convert_union_type(id, info, variants, args),
            Struct(fields) => self.convert_struct_type(id, info, fields, args),

            // Aliases should be desugared prior to codegen
            Alias(_) => unreachable!(),
            Unknown => unreachable!(),
        };

        typ
    }

    /// Monomorphise a types::Type into a hir::Type with no generics.
    fn convert_type(&mut self, typ: &types::Type) -> Type {
        use types::PrimitiveType::Ptr;
        use types::Type::*;

        match typ {
            Primitive(primitive) => self.convert_primitive_type(primitive),

            Function(function) => {
                let mut parameters = fmap(&function.parameters, |typ| {
                    self.convert_type(typ).into()
                });

                let return_type = self.convert_type(&function.return_type);
                let mut environment = None;

                if !self.empty_closure_environment(&function.environment) {
                    let environment_parameter = self.convert_type(&function.environment);
                    parameters.push(environment_parameter.into());
                    environment = Some(environment_parameter);
                }

                let function_pointer = return_type
                    .fn_type(&parameters, function.is_varargs)
                    .ptr_type(AddressSpace::Generic)
                    .into();

                match environment {
                    None => function_pointer,
                    Some(environment) => self
                        .context
                        .struct_type(&[function_pointer, environment], false)
                        .into(),
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
                    UserDefined(id) => self.convert_user_defined_type(*id, args),
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

    fn monomorphise_literal(&mut self, literal: ast::Literal) -> hir::Ast {
        use hir::Ast::*;
        use hir::Literal::*;

        match literal.kind {
            ast::LiteralKind::Integer(n, kind) => {
                let kind = self.convert_integer_kind(kind);
                Literal(Integer(n, kind))
            },
            ast::LiteralKind::Float(f) => Literal(Float(f)),
            ast::LiteralKind::String(s) => {
                let len = Literal(Integer(s.len() as u64, IntegerKind::Usz));
                let c_string = Literal(CString(s));

                Tuple(hir::Tuple {
                    fields: vec![len, c_string],
                })
            },
            ast::LiteralKind::Char(c) => Literal(Char(c)),
            ast::LiteralKind::Bool(b) => Literal(Bool(b)),
            ast::LiteralKind::Unit => unit_literal(),
        }
    }

    fn monomorphise_variable(&mut self, variable: ast::Variable) -> hir::Ast {
        todo!()
    }

    fn monomorphise_lambda(&mut self, lambda: ast::Lambda<'c>) -> hir::Ast {
        todo!()
    }

    fn monomorphise_call(&mut self, call: ast::FunctionCall<'c>) -> hir::Ast {
        todo!()
    }

    fn monomorphise_definition(&mut self, definition: ast::Definition) -> hir::Ast {
        todo!()
    }

    fn monomorphise_if(&mut self, if_: ast::If<'c>) -> hir::Ast {
        let condition = Box::new(self.monomorphise(*if_.condition));
        let then = Box::new(self.monomorphise(*if_.then));
        let otherwise = if_.otherwise.map(|e| Box::new(self.monomorphise(*e)));

        hir::Ast::If(hir::If {
            condition,
            then,
            otherwise,
        })
    }

    fn monomorphise_match(&mut self, match_: ast::Match) -> hir::Ast {
        todo!()
    }

    fn monomorphise_return(&mut self, return_: ast::Return<'c>) -> hir::Ast {
        hir::Ast::Return(hir::Return { 
            expression: Box::new(self.monomorphise(*return_.expression)),
        })
    }

    fn monomorphise_sequence(&mut self, sequence: ast::Sequence<'c>) -> hir::Ast {
        let statements = fmap(sequence.statements, |statement| self.monomorphise(statement));
        hir::Ast::Sequence(hir::Sequence { statements })
    }

    fn monomorphise_extern(&mut self, extern_: ast::Extern) -> hir::Ast {
        let declarations = fmap(extern_.declarations, |decl| {
            let pattern = self.monomorphise(*decl.lhs);
            let typ = self.convert_type(decl.rhs);
            (pattern, typ)
        });

        hir::Ast::Extern(hir::Extern { declarations })
    }

    fn get_field_index(&self, field_name: &str, typ: &types::Type) -> u32 {
        use types::Type::*;

        match self.follow_bindings_shallow(typ) {
            UserDefined(id) => {
                self.cache.type_infos[id.0].find_field(field_name).unwrap().0
            }
            _ => {
                unreachable!("get_field_index called with a type that clearly doesn't have a {} field: {}",
                    field_name,
                    typ.display(&self.cache)
                );
            },
        }
    }

    fn monomorphise_member_access(&mut self, member_access: ast::MemberAccess<'c>) -> hir::Ast {
        let index = self.get_field_index(&member_access.field, member_access.lhs.get_type().unwrap());

        hir::Ast::MemberAccess(hir::MemberAccess {
            lhs: Box::new(self.monomorphise(*member_access.lhs)),
            member_index: index,
        })
    }

    fn monomorphise_assignment(&mut self, assignment: ast::Assignment<'c>) -> hir::Ast {
        hir::Ast::Assignment(hir::Assignment {
            lhs: Box::new(self.monomorphise(*assignment.lhs)),
            rhs: Box::new(self.monomorphise(*assignment.rhs)),
        })
    }
}
