//! typed.rs - Defines a simple trait for getting and setting
//! the type of something. Currently this is only implemented for
//! AST nodes.
use crate::parser::ast::*;
use crate::types::Type;

pub trait Typed {
    fn get_type(&self) -> Option<&Type>;
    fn set_type(&mut self, typ: Type);
}

impl<'a> Typed for Ast<'a> {
    fn get_type(&self) -> Option<&Type> {
        dispatch_on_expr!(self, Typed::get_type)
    }

    fn set_type(&mut self, typ: Type) {
        dispatch_on_expr!(self, Typed::set_type, typ)
    }
}

macro_rules! impl_typed_for {
    ( $name:tt ) => {
        impl<'a> Typed for $name<'a> {
            fn get_type(&self) -> Option<&Type> {
                self.typ.as_ref()
            }

            fn set_type(&mut self, typ: Type) {
                self.typ = Some(typ);
            }
        }
    };
}

impl_typed_for!(Literal);
impl_typed_for!(Variable);
impl_typed_for!(Lambda);
impl_typed_for!(FunctionCall);
impl_typed_for!(Definition);
impl_typed_for!(If);
impl_typed_for!(Match);
impl_typed_for!(TypeDefinition);
impl_typed_for!(TypeAnnotation);
impl_typed_for!(Import);
impl_typed_for!(TraitDefinition);
impl_typed_for!(TraitImpl);
impl_typed_for!(Return);
impl_typed_for!(Sequence);
impl_typed_for!(Extern);
impl_typed_for!(MemberAccess);
impl_typed_for!(Assignment);
impl_typed_for!(EffectDefinition);
impl_typed_for!(Handle);
impl_typed_for!(NamedConstructor);
