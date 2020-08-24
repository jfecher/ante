use crate::parser::ast::{ self, Ast };
use std::fmt::{ self, Display, Formatter };
use crate::util::{ fmap, join_with };

impl<'a> Display for Ast<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        dispatch_on_expr!(self, Display::fmt, f)
    }
}

impl<'a> Display for ast::Literal<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ast::LiteralKind::*;
        match &self.kind {
            Integer(x) => write!(f, "{}", x),
            Float(x) => write!(f, "{}", x),
            String(s) => write!(f, "\"{}\"", s),
            Char(c) => write!(f, "'{}'", c),
            Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Unit => write!(f, "()"),
        }
    }
}

impl<'a> Display for ast::Variable<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ast::VariableKind::*;
        match &self.kind {
            Identifier(name) => write!(f, "{}", name),
            Operator(token) => write!(f, "{}", token),
            TypeConstructor(name) => write!(f, "{}", name),
        }
    }
}

impl<'a> Display for ast::Lambda<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "(\\")?;
        for arg in self.args.iter() {
            write!(f, " {}", arg)?;
        }
        if let Some(typ) = &self.return_type {
            write!(f, " -> {}", typ)?;
        }
        write!(f, " . {})", self.body)
    }
}

impl<'a> Display for ast::FunctionCall<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({} {})", self.function, join_with(&self.args, " "))
    }
}

impl<'a> Display for ast::Definition<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({} = {})", self.pattern, self.expr)
    }
}

impl<'a> Display for ast::If<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(ref otherwise) = self.otherwise {
            write!(f, "(if {} {} {})", self.condition, self.then, otherwise)
        } else {
            write!(f, "(if {} {})", self.condition, self.then)
        }
    }
}

impl<'a> Display for ast::Match<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "(match {}", self.expression)?;
        for (pattern, branch) in self.branches.iter() {
            write!(f, " ({} {})", pattern, branch)?;
        }
        write!(f, ")")
    }
}

impl<'a> Display for ast::Type<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ast::Type::*;
        match self {
            IntegerType(_) => write!(f, "int"),
            FloatType(_) => write!(f, "float"),
            CharType(_) => write!(f, "char"),
            StringType(_) => write!(f, "string"),
            BooleanType(_) => write!(f, "bool"),
            UnitType(_) => write!(f, "unit"),
            ReferenceType(_) => write!(f, "ref"),
            TypeVariable(name, _) => write!(f, "{}", name),
            UserDefinedType(name, _) => write!(f, "{}", name),
            FunctionType(params, return_type, _) => {
                write!(f, "({} -> {})", join_with(params, " "), return_type)
            },
            TypeApplication(constructor, args, _) => {
                write!(f, "({} {})", constructor, join_with(args, " "))
            },
        }
    }
}

impl<'a> Display for ast::TypeDefinitionBody<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ast::TypeDefinitionBody::*;
        match self {
            UnionOf(types) => {
                for (name, variant_fields, _) in types {
                    let s = join_with(variant_fields, " ");
                    write!(f, "| {} {}", name, s)?;
                }
                Ok(())
            },
            StructOf(types) => {
                let types = fmap(&types, |(name, ty, _)| format!("{}: {}", name, ty));
                write!(f, "{}", types.join(", "))
            },
            AliasOf(alias) => write!(f, "{}", alias),
        }
    }
}

impl<'a> Display for ast::TypeDefinition<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let args = join_with(&self.args, "");
        write!(f, "(type {} {} = {})", self.name, args, self.definition)
    }
}

impl<'a> Display for ast::TypeAnnotation<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "(: {} {})", self.lhs, self.rhs)
    }
}

impl<'a> Display for ast::Import<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "(import {})", join_with(&self.path, "."))
    }
}

impl<'a> Display for ast::TraitDefinition<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "(trait {} {} ", self.name, join_with(&self.args, " "))?;
        if !self.fundeps.is_empty() {
            write!(f, "-> {} ", join_with(&self.fundeps, " "))?;
        }
        write!(f, "=\n    {}\n)", join_with(&self.declarations, "\n    "))
    }
}

impl<'a> Display for ast::TraitImpl<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let args = join_with(&self.trait_args, " ");
        let definitions = join_with(&self.definitions, "\n    ");
        write!(f, "(impl {} {}\n    {}\n)", self.trait_name, args, definitions)
    }
}

impl<'a> Display for ast::Return<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "(return {})", self.expression)
    }
}

impl<'a> Display for ast::Sequence<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", join_with(&self.statements, ";\n"))
    }
}

impl<'a> Display for ast::Extern<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "(extern\n    {})", join_with(&self.declarations, "\n    "))
    }
}

impl<'a> Display for ast::MemberAccess<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({}.{})", self.lhs, self.field)
    }
}

impl<'a> Display for ast::Tuple<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        for element in self.elements.iter() {
            write!(f, "{}, ", element)?;
        }
        write!(f, ")")
    }
}
