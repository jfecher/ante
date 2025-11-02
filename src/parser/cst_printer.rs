use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
    sync::Arc,
};

use crate::{
    incremental::{Db, GetItem, Resolve, TypeCheck},
    name_resolution::{namespace::SourceFileId, Origin},
    parser::{
        cst::{Constructor, TopLevelItemKind},
        ids::{NameId, PathId},
    },
    type_inference::type_id::TypeId,
};

use super::{
    cst::{
        Call, Comptime, Cst, Declaration, Definition, EffectDefinition, EffectType, Expr, Extern, FunctionType, Handle,
        HandlePattern, If, Import, Index, Lambda, Literal, Match, MemberAccess, Mutability, OwnershipMode, Parameter,
        Path, Pattern, Quoted, Reference, SequenceItem, Sharedness, TopLevelItem, TraitDefinition, TraitImpl, Type,
        TypeAnnotation, TypeDefinition, TypeDefinitionBody,
    },
    ids::{ExprId, PatternId, TopLevelId},
    TopLevelContext,
};

pub struct CstDisplayContext<'a> {
    cst: &'a Cst,
    context: &'a BTreeMap<TopLevelId, Arc<TopLevelContext>>,
    config: CstDisplayConfig<'a>,
}

#[derive(Copy, Clone, Default)]
pub struct CstDisplayConfig<'db> {
    pub show_comments: bool,

    /// This field is required if `show_resolved` or `show_types` are set
    pub db: Option<&'db Db>,

    /// Show resolved definitions for each name. Requires `db` to bet set.
    pub show_resolved: bool,

    /// Show types for each name. Requires `db` to bet set.
    pub show_types: bool,
}

impl Cst {
    pub fn display<'a>(&'a self, context: &'a BTreeMap<TopLevelId, Arc<TopLevelContext>>) -> CstDisplayContext<'a> {
        CstDisplayContext { cst: self, context, config: CstDisplayConfig::default() }
    }

    /// Display this Cst, annotating each name with a number pointing to its
    /// resolved definition
    pub fn display_resolved<'a>(
        &'a self, context: &'a BTreeMap<TopLevelId, Arc<TopLevelContext>>, compiler: &'a Db,
    ) -> CstDisplayContext<'a> {
        let config = CstDisplayConfig { show_resolved: true, db: Some(compiler), ..Default::default() };
        CstDisplayContext { cst: self, context, config }
    }

    /// Display this Cst, annotating each name with its type
    pub fn display_typed<'a>(
        &'a self, context: &'a BTreeMap<TopLevelId, Arc<TopLevelContext>>, compiler: &'a Db,
    ) -> CstDisplayContext<'a> {
        let config = CstDisplayConfig { show_types: true, db: Some(compiler), ..Default::default() };
        CstDisplayContext { cst: self, context, config }
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut path = self.components.iter();
        write!(f, "{}", path.next().unwrap().0)?;
        for (item, _) in path {
            write!(f, ".{item}")?;
        }
        Ok(())
    }
}

pub struct PatternDisplayContext<'a> {
    pattern: PatternId,
    context: &'a TopLevelContext,
    config: CstDisplayConfig<'a>,
}

impl PatternId {
    pub fn display_cst(self, context: &TopLevelContext) -> PatternDisplayContext {
        PatternDisplayContext { pattern: self, context, config: Default::default() }
    }
}

impl<'a> Display for PatternDisplayContext<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        CstDisplay::new(&BTreeMap::new(), self.config).fmt_pattern(self.pattern, self.context, f)
    }
}

pub struct ExprDisplayContext<'a> {
    expr: ExprId,
    context: &'a TopLevelContext,
    config: CstDisplayConfig<'a>,
}

impl ExprId {
    pub fn display_cst(self, context: &TopLevelContext) -> ExprDisplayContext {
        ExprDisplayContext { expr: self, context, config: Default::default() }
    }
}

impl<'a> Display for ExprDisplayContext<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        CstDisplay::new(&BTreeMap::new(), self.config).fmt_expr(self.expr, self.context, f)
    }
}

pub struct TypeDisplayContext<'a> {
    typ: &'a Type,
    context: &'a TopLevelContext,
    config: CstDisplayConfig<'a>,
}

impl Type {
    pub fn display<'a>(&'a self, context: &'a TopLevelContext) -> TypeDisplayContext {
        TypeDisplayContext { typ: self, context, config: Default::default() }
    }
}

impl<'a> Display for TypeDisplayContext<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        CstDisplay::new(&BTreeMap::new(), self.config).fmt_type(self.typ, self.context, f)
    }
}

/// This needs to be separate from `CstDisplayContext` since fmt requires `self` to be immutable
/// but we need to mutate indent_level.
struct CstDisplay<'a> {
    indent_level: u32,
    current_item_id: Option<TopLevelId>,
    context: &'a BTreeMap<TopLevelId, Arc<TopLevelContext>>,
    config: CstDisplayConfig<'a>,
}

impl<'a> Display for CstDisplayContext<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        CstDisplay::new(self.context, self.config).fmt_cst(self.cst, f)
    }
}

impl<'a> CstDisplay<'a> {
    fn new(context: &'a BTreeMap<TopLevelId, Arc<TopLevelContext>>, config: CstDisplayConfig<'a>) -> Self {
        Self { context, current_item_id: None, indent_level: 0, config }
    }

    fn fmt_cst(&mut self, cst: &Cst, f: &mut Formatter) -> std::fmt::Result {
        for import in &cst.imports {
            self.fmt_import(import, f)?;
            writeln!(f)?;
        }

        if !cst.imports.is_empty() {
            writeln!(f)?;
        }

        for item in &cst.top_level_items {
            if let Some(db) = self.db_resolve() {
                let (item, context) = GetItem(item.id).get(db);
                self.fmt_top_level_item(&item, &context, f)?;
            } else {
                let context = &self.context[&item.id];
                self.fmt_top_level_item(item, context, f)?;
            }
            writeln!(f)?;
        }

        self.fmt_comments(&cst.ending_comments, f)
    }

    fn fmt_import(&mut self, import: &Import, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_comments(&import.comments, f)?;

        write!(f, "import {}.", import.crate_name)?;

        let path = import.module_path.to_string_lossy().replace("/", ".");
        if !path.is_empty() {
            write!(f, "{path}.")?;
        }

        for (i, (item, _location)) in import.items.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{item}")?;
        }
        Ok(())
    }

    fn fmt_top_level_item(
        &mut self, item: &TopLevelItem, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        self.current_item_id = Some(item.id);

        self.fmt_comments(&item.comments, f)?;

        if self.config.show_resolved {
            writeln!(f, "// id = {}", item.id)?;
            self.indent(f)?;
        }

        match &item.kind {
            TopLevelItemKind::TypeDefinition(type_definition) => self.fmt_type_definition(type_definition, context, f),
            TopLevelItemKind::Definition(definition) => self.fmt_definition(definition, context, f),
            TopLevelItemKind::TraitDefinition(trait_definition) => {
                self.fmt_trait_definition(trait_definition, context, f)
            },
            TopLevelItemKind::TraitImpl(trait_impl) => self.fmt_trait_impl(trait_impl, context, f),
            TopLevelItemKind::EffectDefinition(effect_definition) => {
                self.fmt_effect_definition(effect_definition, context, f)
            },
            TopLevelItemKind::Extern(extern_) => self.fmt_extern(extern_, context, f),
            TopLevelItemKind::Comptime(comptime) => self.fmt_comptime(comptime, context, f),
        }?;
        writeln!(f)
    }

    fn fmt_comments(&self, comments: &[String], f: &mut Formatter) -> std::fmt::Result {
        if self.config.show_comments {
            for comment in comments {
                writeln!(f, "{comment}")?;
                self.indent(f)?;
            }
        }
        Ok(())
    }

    fn fmt_definition(
        &mut self, definition: &Definition, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        if definition.implicit {
            write!(f, "implicit ")?;
        }

        if definition.mutable {
            write!(f, "mut ")?;
        }

        if let Expr::Lambda(lambda) = &context.exprs[definition.rhs] {
            return self.fmt_function(definition, lambda, context, f);
        }

        self.fmt_pattern(definition.pattern, context, f)?;

        write!(f, " =")?;
        if !self.is_block(definition.rhs, context) {
            write!(f, " ")?;
        }

        self.fmt_expr(definition.rhs, context, f)
    }

    fn db_resolve(&self) -> Option<&'a Db> {
        self.config
            .show_resolved
            .then(|| self.config.db.expect("Expected `CstDisplayConfig::db` to be set when `show_resolved` is set"))
    }

    fn db_type_check(&self) -> Option<&'a Db> {
        self.config
            .show_types
            .then(|| self.config.db.expect("Expected `CstDisplayConfig::db` to be set when `show_types` is set"))
    }

    fn fmt_name(&self, name: NameId, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_name_helper(name, context, f, true)
    }

    fn fmt_type_name(&self, name: NameId, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_name_helper(name, context, f, false)
    }

    fn fmt_name_helper(
        &self, name: NameId, context: &TopLevelContext, f: &mut Formatter, show_type: bool,
    ) -> std::fmt::Result {
        if self.config.show_types && show_type {
            write!(f, "(")?;
        }

        write!(f, "{}", &context.names[name])?;

        if let Some(db) = self.db_resolve() {
            let resolved = Resolve(self.current_item_id.unwrap()).get(db);
            let origin = resolved.name_origins.get(&name);
            let id = origin.map(ToString::to_string).unwrap_or_else(|| "?".into());
            write!(f, "_{id}")?;
        }

        if let Some(db) = self.db_type_check() {
            if show_type {
                let check = TypeCheck(self.current_item_id.unwrap()).get(db);
                let typ = check.result.maps.name_types.get(&name).copied().unwrap_or(TypeId::ERROR);
                write!(f, ": {})", typ.to_string(&check.types, &check.bindings, &context.names, db))?
            }
        }

        Ok(())
    }

    fn fmt_path(&self, path: PathId, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_path_helper(path, context, f, true)
    }

    fn fmt_type_path(&self, path: PathId, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_path_helper(path, context, f, false)
    }

    fn fmt_path_helper(
        &self, path: PathId, context: &TopLevelContext, f: &mut Formatter, show_type: bool,
    ) -> std::fmt::Result {
        if self.config.show_types && show_type {
            write!(f, "(")?;
        }

        write!(f, "{}", &context.paths[path])?;

        if let Some(db) = self.db_resolve() {
            let resolved = Resolve(self.current_item_id.unwrap()).get(db);
            let origin = resolved.path_origins.get(&path);
            let id = origin.map(ToString::to_string).unwrap_or_else(|| "?".into());
            write!(f, "_{id}")?;
        }

        if show_type {
            if let Some(db) = self.db_type_check() {
                let check = TypeCheck(self.current_item_id.unwrap()).get(db);
                let typ = check.result.maps.path_types.get(&path).copied().unwrap_or(TypeId::ERROR);
                write!(f, ": {})", typ.to_string(&check.types, &check.bindings, &context.names, db))?
            }
        }

        Ok(())
    }

    fn fmt_function(
        &mut self, definition: &Definition, lambda: &Lambda, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        self.fmt_pattern(definition.pattern, context, f)?;
        self.fmt_lambda_inner(lambda, context, f, false)
    }

    /// Format each part of a lambda except the leading `fn`
    ///
    /// If `write_arrow` is true, `->` will be used as the body separator. Otherwise `=` is used.
    fn fmt_lambda_inner(
        &mut self, lambda: &Lambda, context: &TopLevelContext, f: &mut Formatter, write_arrow: bool,
    ) -> std::fmt::Result {
        self.fmt_parameters(&lambda.parameters, context, f)?;

        if let Some(typ) = &lambda.return_type {
            write!(f, " : ")?;
            self.fmt_type(typ, context, f)?;
            self.fmt_effect_clause(&lambda.effects, context, f)?;
        }

        write!(f, " {}", if write_arrow { "->" } else { "=" })?;
        if !self.is_block(lambda.body, context) {
            write!(f, " ")?;
        }
        self.fmt_expr(lambda.body, context, f)
    }

    /// Formats an effect clause with a leading space
    fn fmt_effect_clause(
        &self, effects: &Option<Vec<EffectType>>, context: &TopLevelContext, f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        if let Some(effects) = effects {
            if effects.is_empty() {
                write!(f, " pure")?;
            } else {
                write!(f, " can ")?;
                for (i, effect) in effects.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    self.fmt_effect_type(effect, context, f)?;
                }
            }
        }
        Ok(())
    }

    fn fmt_effect_type(&self, effect: &EffectType, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        match effect {
            EffectType::Known(path_id, args) => {
                self.fmt_path(*path_id, context, f)?;
                self.fmt_type_args(args, context, f)
            },
            EffectType::Variable(name_id) => self.fmt_type_name(*name_id, context, f),
        }
    }

    /// Formats type arguments with a leading space in front of each (including the first)
    fn fmt_type_args(&self, args: &[Type], context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        let requires_parens = |typ: &Type| matches!(typ, Type::Function(_) | Type::Application(..));

        for arg in args {
            if requires_parens(arg) {
                write!(f, " (")?;
                self.fmt_type(arg, context, f)?;
                write!(f, ")")?;
            } else {
                write!(f, " ")?;
                self.fmt_type(arg, context, f)?;
            }
        }
        Ok(())
    }

    fn indent(&self, f: &mut Formatter) -> std::fmt::Result {
        for _ in 0..self.indent_level {
            write!(f, "    ")?;
        }
        Ok(())
    }

    fn newline(&self, f: &mut Formatter) -> std::fmt::Result {
        writeln!(f)?;
        self.indent(f)
    }

    fn fmt_type_definition(
        &mut self, type_definition: &TypeDefinition, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        if type_definition.shared {
            write!(f, "shared ")?;
        }

        write!(f, "type ")?;
        self.fmt_type_name(type_definition.name, context, f)?;
        write!(f, " =")?;

        match &type_definition.body {
            TypeDefinitionBody::Error => {
                write!(f, " (error)")?;
            },
            TypeDefinitionBody::Struct(fields) => {
                self.indent_level += 1;
                for (name, typ) in fields {
                    self.newline(f)?;
                    self.fmt_type_name(*name, context, f)?;
                    write!(f, ": ")?;
                    self.fmt_type(typ, context, f)?;
                }
                self.indent_level -= 1;
            },
            TypeDefinitionBody::Enum(variants) => {
                self.indent_level += 1;
                for (name, params) in variants {
                    self.newline(f)?;
                    write!(f, "| ")?;
                    self.fmt_type_name(*name, context, f)?;
                    self.fmt_type_args(params, context, f)?;
                }
                self.indent_level -= 1;
            },
            TypeDefinitionBody::Alias(typ) => {
                write!(f, " ")?;
                self.fmt_type(typ, context, f)?;
            },
        }
        Ok(())
    }

    fn fmt_type(&self, typ: &Type, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        match typ {
            Type::Error => write!(f, "(error)"),
            Type::Named(path) => self.fmt_type_path(*path, context, f),
            Type::Variable(name) => self.fmt_type_name(*name, context, f),
            Type::Unit => write!(f, "Unit"),
            Type::Integer(kind) => write!(f, "{kind}"),
            Type::Float(kind) => write!(f, "{kind}"),
            Type::Function(function_type) => self.fmt_function_type(function_type, context, f),
            Type::Application(constructor, args) => self.fmt_type_application(constructor, args, context, f),
            Type::String => write!(f, "String"),
            Type::Char => write!(f, "Char"),
            Type::Pair => write!(f, ","),
            Type::Reference(mutable, shared) => self.fmt_reference_type(*mutable, *shared, f),
        }
    }

    fn fmt_reference_type(&self, mutable: Mutability, shared: Sharedness, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{mutable}{shared}")
    }

    fn fmt_type_application(
        &self, constructor: &Type, args: &[Type], context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        if *constructor == Type::Pair {
            return self.fmt_pair_type(args, context, f);
        }

        let requires_parens = |typ: &Type| matches!(typ, Type::Function(_) | Type::Application(..));
        if requires_parens(constructor) {
            write!(f, "(")?;
            self.fmt_type(constructor, context, f)?;
            write!(f, ")")?;
        } else {
            self.fmt_type(constructor, context, f)?;
        }

        self.fmt_type_args(args, context, f)
    }

    fn fmt_pair_type(&self, args: &[Type], context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        assert_eq!(args.len(), 2);

        let lhs_requires_parens = |typ: &Type| match typ {
            Type::Function(_) => true,
            Type::Application(function, _) => matches!(function.as_ref(), Type::Pair),
            _ => false,
        };

        if lhs_requires_parens(&args[0]) {
            write!(f, "(")?;
            self.fmt_type(&args[0], context, f)?;
            write!(f, ")")?;
        } else {
            self.fmt_type(&args[0], context, f)?;
        }

        write!(f, ", ")?;
        self.fmt_type(&args[1], context, f)
    }

    fn fmt_function_type(
        &self, function_type: &FunctionType, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        write!(f, "fn")?;
        self.fmt_type_args(&function_type.parameters, context, f)?;
        write!(f, " -> ")?;
        self.fmt_type(&function_type.return_type, context, f)?;
        self.fmt_effect_clause(&function_type.effects, context, f)
    }

    fn fmt_expr(&mut self, expr: ExprId, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        match &context.exprs[expr] {
            Expr::Error => write!(f, "(error)"),
            Expr::Literal(literal) => self.fmt_literal(literal, f),
            Expr::Variable(path) => self.fmt_path(*path, context, f),
            Expr::Sequence(seq) => self.fmt_sequence(seq, context, f),
            Expr::Definition(definition) => self.fmt_definition(definition, context, f),
            Expr::Call(call) => self.fmt_call(call, context, f),
            Expr::MemberAccess(access) => self.fmt_member_access(access, context, f),
            Expr::Index(index) => self.fmt_index(index, context, f),
            Expr::Lambda(lambda) => self.fmt_lambda(lambda, context, f),
            Expr::If(if_) => self.fmt_if(if_, context, f),
            Expr::Match(match_) => self.fmt_match(match_, context, f),
            Expr::Handle(handle_) => self.fmt_handle(handle_, context, f),
            Expr::Reference(reference) => self.fmt_reference(reference, context, f),
            Expr::TypeAnnotation(type_annotation) => self.fmt_type_annotation(type_annotation, context, f),
            Expr::Quoted(quoted) => self.fmt_quoted(quoted, f),
            Expr::Constructor(constructor) => self.fmt_constructor(constructor, context, f),
        }
    }

    fn fmt_literal(&mut self, literal: &Literal, f: &mut Formatter) -> std::fmt::Result {
        match literal {
            Literal::Unit => write!(f, "()"),
            Literal::Bool(value) => write!(f, "{value}"),
            Literal::String(s) => write!(f, "\"{s}\""),
            Literal::Char(c) => write!(f, "c\"{c}\""),
            Literal::Integer(x, Some(kind)) => write!(f, "{x}_{kind}"),
            Literal::Integer(x, None) => write!(f, "{x}"),
            Literal::Float(x, Some(kind)) => write!(f, "{x}_{kind}"),
            Literal::Float(x, None) => write!(f, "{x}"),
        }
    }

    fn fmt_sequence(&mut self, seq: &[SequenceItem], context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        self.indent_level += 1;
        for item in seq {
            self.newline(f)?;
            self.fmt_comments(&item.comments, f)?;
            self.fmt_expr(item.expr, context, f)?;
        }
        self.indent_level -= 1;
        Ok(())
    }

    fn is_block(&self, expr: ExprId, context: &TopLevelContext) -> bool {
        matches!(&context.exprs[expr], Expr::Sequence(_))
    }

    fn fmt_call(&mut self, call: &Call, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        if call.arguments.len() == 2 && self.is_operator(call.function, context) {
            return self.fmt_infix_operator(call, context, f);
        }

        self.fmt_expr(call.function, context, f)?;

        for arg in call.arguments.iter().copied() {
            let parenthesize = !context.exprs[arg].is_atom();
            write!(f, " ")?;
            self.parenthesize(arg, parenthesize, context, f)?;
        }

        Ok(())
    }

    fn fmt_infix_operator(&mut self, call: &Call, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        assert_eq!(call.arguments.len(), 2);
        let lhs = call.arguments[0];
        let rhs = call.arguments[1];

        let parenthesize = |this: &Self, expr| match &context.exprs[expr] {
            Expr::Call(call) => this.is_operator(call.function, context),
            other => !other.is_atom(),
        };

        self.parenthesize(lhs, parenthesize(self, lhs), context, f)?;
        write!(f, " ")?;
        self.fmt_expr(call.function, context, f)?;
        write!(f, " ")?;
        self.parenthesize(rhs, parenthesize(self, rhs), context, f)
    }

    /// If `should_parenthesize` is true, format the given expression surrounded by parenthesis.
    /// Otherwise, format it normally.
    fn parenthesize(
        &mut self, expr: ExprId, should_parenthesize: bool, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        if should_parenthesize {
            write!(f, "(")?;
            self.fmt_expr(expr, context, f)?;
            write!(f, ")")
        } else {
            self.fmt_expr(expr, context, f)
        }
    }

    fn is_operator(&self, function: ExprId, context: &TopLevelContext) -> bool {
        if let Expr::Variable(path) = context.exprs[function] {
            let path = &context.paths[path];
            if path.components.len() == 1 {
                let name = &path.components[0].0;
                !name.chars().next().unwrap().is_alphanumeric()
            } else {
                false
            }
        } else {
            false
        }
    }

    fn fmt_member_access(
        &mut self, access: &MemberAccess, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        if context.exprs[access.object].is_atom() {
            self.fmt_expr(access.object, context, f)?;
        } else {
            write!(f, "(")?;
            self.fmt_expr(access.object, context, f)?;
            write!(f, ")")?;
        }

        match access.ownership {
            OwnershipMode::Owned => write!(f, ".{}", access.member),
            OwnershipMode::Borrow => write!(f, ".&{}", access.member),
            OwnershipMode::BorrowMut => write!(f, ".!{}", access.member),
        }
    }

    fn fmt_index(&mut self, index: &Index, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        if context.exprs[index.object].is_atom() {
            self.fmt_expr(index.object, context, f)?;
        } else {
            write!(f, "(")?;
            self.fmt_expr(index.object, context, f)?;
            write!(f, ")")?;
        }

        match index.ownership {
            OwnershipMode::Owned => write!(f, ".[")?,
            OwnershipMode::Borrow => write!(f, ".&[")?,
            OwnershipMode::BorrowMut => write!(f, ".![")?,
        }

        self.fmt_expr(index.index, context, f)?;
        write!(f, "]")
    }

    fn fmt_declaration(
        &self, declaration: &Declaration, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        self.fmt_name(declaration.name, context, f)?;
        write!(f, ": ")?;
        self.fmt_type(&declaration.typ, context, f)
    }

    fn fmt_trait_definition(
        &mut self, trait_definition: &TraitDefinition, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        write!(f, "trait ")?;
        self.fmt_type_name(trait_definition.name, context, f)?;

        for generic in &trait_definition.generics {
            write!(f, " ")?;
            self.fmt_type_name(*generic, context, f)?;
        }

        if !trait_definition.functional_dependencies.is_empty() {
            write!(f, " ->")?;
            for generic in &trait_definition.functional_dependencies {
                write!(f, " ")?;
                self.fmt_type_name(*generic, context, f)?;
            }
        }

        write!(f, " with")?;
        self.indent_level += 1;
        for declaration in &trait_definition.body {
            self.newline(f)?;
            self.fmt_declaration(declaration, context, f)?;
        }
        self.indent_level -= 1;
        Ok(())
    }

    fn fmt_trait_impl(
        &mut self, trait_impl: &TraitImpl, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        write!(f, "impl ")?;
        self.fmt_type_name(trait_impl.name, context, f)?;
        self.fmt_parameters(&trait_impl.parameters, context, f)?;

        write!(f, ": ")?;
        self.fmt_path(trait_impl.trait_path, context, f)?;
        self.fmt_type_args(&trait_impl.trait_arguments, context, f)?;

        write!(f, " with")?;
        self.indent_level += 1;
        for (name, expr) in &trait_impl.body {
            self.newline(f)?;
            self.fmt_name(*name, context, f)?;

            if let Expr::Lambda(lambda) = &context.exprs[*expr] {
                self.fmt_lambda_inner(lambda, context, f, false)?;
            } else {
                write!(f, " =")?;
                if !self.is_block(*expr, context) {
                    write!(f, " ")?;
                }
                self.fmt_expr(*expr, context, f)?;
            }
        }
        self.indent_level -= 1;
        Ok(())
    }

    fn fmt_effect_definition(
        &mut self, effect_definition: &EffectDefinition, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        write!(f, "effect ")?;
        self.fmt_type_name(effect_definition.name, context, f)?;

        for generic in &effect_definition.generics {
            write!(f, " ")?;
            self.fmt_type_name(*generic, context, f)?;
        }

        write!(f, " with")?;
        self.indent_level += 1;
        for declaration in &effect_definition.body {
            self.newline(f)?;
            self.fmt_declaration(declaration, context, f)?;
        }
        self.indent_level -= 1;
        Ok(())
    }

    fn fmt_extern(&mut self, extern_: &Extern, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "extern ")?;
        self.fmt_declaration(&extern_.declaration, context, f)
    }

    fn fmt_lambda(&mut self, lambda: &Lambda, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "fn")?;
        self.fmt_lambda_inner(lambda, context, f, true)
    }

    fn fmt_if(&mut self, if_: &If, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "if ")?;
        self.fmt_expr(if_.condition, context, f)?;

        if !self.is_block(if_.condition, context) {
            write!(f, " ")?;
        }
        write!(f, "then ")?;
        self.fmt_expr(if_.then, context, f)?;

        if let Some(else_) = if_.else_ {
            if self.is_block(if_.then, context) {
                self.newline(f)?;
            } else {
                write!(f, " ")?;
            }
            write!(f, "else ")?;
            self.fmt_expr(else_, context, f)?;
        }
        Ok(())
    }

    fn fmt_match(&mut self, match_: &Match, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "match ")?;
        self.fmt_expr(match_.expression, context, f)?;

        for (pattern, branch) in &match_.cases {
            self.newline(f)?;
            write!(f, "| ")?;
            self.fmt_pattern(*pattern, context, f)?;
            write!(f, " -> ")?;
            self.fmt_expr(*branch, context, f)?;
        }

        Ok(())
    }

    fn fmt_handle(&mut self, handle_: &Handle, context: &TopLevelContext, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "handle ")?;
        self.fmt_expr(handle_.expression, context, f)?;

        for (pattern, branch) in &handle_.cases {
            self.newline(f)?;
            write!(f, "| ")?;
            self.fmt_handle_pattern(pattern, context, f)?;
            write!(f, " -> ")?;
            self.fmt_expr(*branch, context, f)?;
        }

        Ok(())
    }

    fn fmt_handle_pattern(
        &mut self, pattern: &HandlePattern, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        self.fmt_name(pattern.function, context, f)?;
        for arg in pattern.args.iter() {
            if self.is_pattern_atom(*arg, context) {
                write!(f, " ")?;
                self.fmt_pattern(*arg, context, f)?;
            } else {
                write!(f, " (")?;
                self.fmt_pattern(*arg, context, f)?;
                write!(f, ")")?;
            }
        }
        Ok(())
    }

    fn fmt_pattern(&mut self, pattern: PatternId, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        match &context.patterns[pattern] {
            Pattern::Variable(name) => self.fmt_name(*name, context, f),
            Pattern::Literal(literal) => self.fmt_literal(literal, f),
            Pattern::Constructor(path, args) => {
                self.fmt_path(*path, context, f)?;
                for arg in args {
                    if self.is_pattern_atom(*arg, context) {
                        write!(f, " ")?;
                        self.fmt_pattern(*arg, context, f)?;
                    } else {
                        write!(f, " (")?;
                        self.fmt_pattern(*arg, context, f)?;
                        write!(f, ")")?;
                    }
                }
                Ok(())
            },
            Pattern::Error => write!(f, "(error)"),
            Pattern::TypeAnnotation(pattern, typ) => {
                self.fmt_pattern(*pattern, context, f)?;

                // If show types is set we don't want to print annotations twice
                if !(matches!(&context.patterns[*pattern], Pattern::Variable(_)) && self.config.show_types) {
                    write!(f, ": ")?;
                    self.fmt_type(typ, context, f)?;
                }
                Ok(())
            },
            Pattern::MethodName { type_name, item_name } => {
                self.fmt_type_name(*type_name, context, f)?;
                write!(f, ".")?;
                self.fmt_name(*item_name, context, f)
            },
        }
    }

    fn fmt_reference(
        &mut self, reference: &Reference, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        write!(f, "{}{}", reference.mutability, reference.sharedness)?;
        if reference.sharedness != Sharedness::Shared {
            write!(f, " ")?;
        }
        self.fmt_expr(reference.rhs, context, f)
    }

    fn fmt_type_annotation(
        &mut self, type_annotation: &TypeAnnotation, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        self.fmt_expr(type_annotation.lhs, context, f)?;

        // If show types is set we don't want to print annotations twice
        if !(matches!(&context.exprs[type_annotation.lhs], Expr::Variable(_)) && self.config.show_types) {
            write!(f, ": ")?;
            self.fmt_type(&type_annotation.rhs, context, f)?;
        }
        Ok(())
    }

    fn fmt_constructor(
        &mut self, constructor: &Constructor, context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        self.fmt_type(&constructor.typ, context, f)?;
        write!(f, " with")?;
        self.indent_level += 1;
        for (name, expr) in &constructor.fields {
            self.newline(f)?;
            self.fmt_name(*name, context, f)?;
            write!(f, " = ")?;
            self.fmt_expr(*expr, context, f)?;
        }
        self.indent_level -= 1;
        Ok(())
    }

    fn fmt_comptime(&mut self, comptime: &Comptime, context: &TopLevelContext, f: &mut Formatter) -> std::fmt::Result {
        match comptime {
            Comptime::Expr(expr_id) => {
                write!(f, "#")?;
                self.fmt_expr(*expr_id, context, f)
            },
            Comptime::Derive(paths) => {
                write!(f, "derive")?;
                for path in paths {
                    write!(f, " ")?;
                    self.fmt_path(*path, context, f)?;
                }
                Ok(())
            },
            Comptime::Definition(definition) => {
                write!(f, "#")?;
                self.fmt_definition(definition, context, f)
            },
        }
    }

    fn fmt_quoted(&self, quoted: &Quoted, f: &mut Formatter) -> std::fmt::Result {
        assert!(!quoted.tokens.is_empty());
        write!(f, "'{}", quoted.tokens.first().unwrap())?;

        for token in quoted.tokens.iter().skip(1) {
            write!(f, " {token}")?;
        }
        Ok(())
    }

    /// True if this pattern never requires parenthesis
    fn is_pattern_atom(&self, pattern: PatternId, context: &TopLevelContext) -> bool {
        use Pattern::*;
        match &context.patterns[pattern] {
            Error | Variable(_) | Literal(_) | MethodName { .. } => true,
            Constructor(_, args) => args.is_empty(),
            TypeAnnotation(_, _) => false,
        }
    }

    fn fmt_parameters(
        &mut self, parameters: &[Parameter], context: &TopLevelContext, f: &mut Formatter,
    ) -> std::fmt::Result {
        for parameter in parameters {
            write!(f, " ")?;
            if parameter.implicit {
                write!(f, "{{")?;
                self.fmt_pattern(parameter.pattern, context, f)?;
                write!(f, "}}")?;
            } else if self.is_pattern_atom(parameter.pattern, context) {
                self.fmt_pattern(parameter.pattern, context, f)?;
            } else {
                write!(f, "(")?;
                self.fmt_pattern(parameter.pattern, context, f)?;
                write!(f, ")")?;
            }
        }
        Ok(())
    }
}

impl Display for Mutability {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Mutability::Immutable => write!(f, "&"),
            Mutability::Mutable => write!(f, "!"),
        }
    }
}

impl Display for Sharedness {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Sharedness::Shared => Ok(()),
            Sharedness::Owned => write!(f, "own"),
        }
    }
}

impl Display for Origin {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Origin::TopLevelDefinition(top_level_id) => write!(f, "{top_level_id}"),
            Origin::Local(name_id) => write!(f, "{name_id}"),
            Origin::TypeResolution => write!(f, "td"), // type-directed
            Origin::Builtin(_) => write!(f, "b"),
        }
    }
}

impl Display for SourceFileId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Limit to 3 digits, otherwise it is too long and hurts the debug format
        write!(f, "c{}m{}", self.crate_id.0, self.local_module_id.0 % 1000)
    }
}
