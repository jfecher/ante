use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
    sync::Arc,
};

use crate::{
    incremental::{Db, Resolve, TypeCheck},
    name_resolution::{namespace::SourceFileId, Origin},
    parser::ids::{NameId, PathId}, type_inference::type_id::TypeId,
};

use super::{
    cst::{
        Call, Comptime, Cst, Declaration, Definition, EffectDefinition, EffectType, Expr, Extern, FunctionType, Handle,
        HandlePattern, If, Import, Index, Lambda, Literal, Match, MemberAccess, Mutability, OwnershipMode, Parameter,
        Path, Pattern, Quoted, Reference, SequenceItem, Sharedness, TopLevelItem, TopLevelItemKind, TraitDefinition,
        TraitImpl, Type, TypeAnnotation, TypeDefinition, TypeDefinitionBody,
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
        CstDisplay::new(&BTreeMap::new(), Some(self.context), self.config)
            .fmt_pattern(self.pattern, f)
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
        CstDisplay::new(&BTreeMap::new(), Some(self.context), self.config)
            .fmt_expr(self.expr, f)
    }
}

/// This needs to be separate from `CstDisplayContext` since fmt requires `self` to be immutable
/// but we need to mutate indent_level.
struct CstDisplay<'a> {
    indent_level: u32,
    current_item: Option<&'a TopLevelContext>,
    current_item_id: Option<TopLevelId>,
    context: &'a BTreeMap<TopLevelId, Arc<TopLevelContext>>,
    config: CstDisplayConfig<'a>,
}

impl<'a> Display for CstDisplayContext<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        CstDisplay::new(self.context, None, self.config)
            .fmt_cst(self.cst, f)
    }
}

impl<'a> CstDisplay<'a> {
    fn new(context: &'a BTreeMap<TopLevelId, Arc<TopLevelContext>>, current_item: Option<&'a TopLevelContext>, config: CstDisplayConfig<'a>) -> Self {
        Self { context, current_item, current_item_id: None, indent_level: 0, config }
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
            self.fmt_top_level_item(item, f)?;
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

    fn context(&self) -> &'a TopLevelContext {
        self.current_item.unwrap()
    }

    fn fmt_top_level_item(&mut self, item: &TopLevelItem, f: &mut Formatter) -> std::fmt::Result {
        self.current_item = self.context.get(&item.id).map(|x| x.as_ref());
        self.current_item_id = Some(item.id);

        self.fmt_comments(&item.comments, f)?;

        if self.config.show_resolved {
            writeln!(f, "// id = {}", item.id)?;
            self.indent(f)?;
        }

        match &item.kind {
            TopLevelItemKind::TypeDefinition(type_definition) => self.fmt_type_definition(type_definition, f),
            TopLevelItemKind::Definition(definition) => self.fmt_definition(definition, f),
            TopLevelItemKind::TraitDefinition(trait_definition) => self.fmt_trait_definition(trait_definition, f),
            TopLevelItemKind::TraitImpl(trait_impl) => self.fmt_trait_impl(trait_impl, f),
            TopLevelItemKind::EffectDefinition(effect_definition) => self.fmt_effect_definition(effect_definition, f),
            TopLevelItemKind::Extern(extern_) => self.fmt_extern(extern_, f),
            TopLevelItemKind::Comptime(comptime) => self.fmt_comptime(comptime, f),
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

    fn fmt_definition(&mut self, definition: &Definition, f: &mut Formatter) -> std::fmt::Result {
        if let Expr::Lambda(lambda) = &self.context().exprs[definition.rhs] {
            return self.fmt_function(definition, lambda, f);
        }

        if definition.mutable {
            write!(f, "mut ")?;
        }

        self.fmt_pattern(definition.pattern, f)?;

        write!(f, " =")?;
        if !matches!(self.context().exprs[definition.rhs], Expr::Sequence(_)) {
            write!(f, " ")?;
        }

        self.fmt_expr(definition.rhs, f)
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

    fn fmt_name(&self, name: NameId, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_name_helper(name, f, true)
    }

    fn fmt_type_name(&self, name: NameId, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_name_helper(name, f, false)
    }

    fn fmt_name_helper(&self, name: NameId, f: &mut Formatter, show_type: bool) -> std::fmt::Result {
        if self.config.show_types && show_type {
            write!(f, "(")?;
        }

        write!(f, "{}", &self.context().names[name])?;

        if let Some(db) = self.db_resolve() {
            let resolved = Resolve(self.current_item_id.unwrap()).get(db);
            let origin = resolved.name_origins.get(&name);
            let id = origin.map(ToString::to_string).unwrap_or_else(|| "?".into());
            write!(f, "_{id}")?;
        }

        if let Some(db) = self.db_type_check() {
            if show_type {
                let check = TypeCheck(self.current_item_id.unwrap()).get(db);
                let typ = check.result.name_types.get(&name).copied().unwrap_or(TypeId::ERROR);
                write!(f, ": {})", typ.to_string(&check.types, &check.bindings, &self.context().names, db))?
            }
        }

        Ok(())
    }

    fn fmt_path(&self, path: PathId, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_path_helper(path, f, true)
    }

    fn fmt_type_path(&self, path: PathId, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_path_helper(path, f, false)
    }

    fn fmt_path_helper(&self, path: PathId, f: &mut Formatter, show_type: bool) -> std::fmt::Result {
        if self.config.show_types && show_type {
            write!(f, "(")?;
        }

        write!(f, "{}", &self.context().paths[path])?;

        if let Some(db) = self.db_resolve() {
            let resolved = Resolve(self.current_item_id.unwrap()).get(db);
            let origin = resolved.path_origins.get(&path);
            let id = origin.map(ToString::to_string).unwrap_or_else(|| "?".into());
            write!(f, "_{id}")?;
        }

        if show_type {
            if let Some(db) = self.db_type_check() {
                let check = TypeCheck(self.current_item_id.unwrap()).get(db);
                let typ = check.result.path_types.get(&path).copied().unwrap_or(TypeId::ERROR);
                write!(f, ": {})", typ.to_string(&check.types, &check.bindings, &self.context().names, db))?
            }
        }

        Ok(())
    }

    fn fmt_function(&mut self, definition: &Definition, lambda: &Lambda, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_pattern(definition.pattern, f)?;
        self.fmt_lambda_inner(lambda, f, false)
    }

    /// Format each part of a lambda except the leading `fn`
    ///
    /// If `write_arrow` is true, `->` will be used as the body separator. Otherwise `=` is used.
    fn fmt_lambda_inner(&mut self, lambda: &Lambda, f: &mut Formatter, write_arrow: bool) -> std::fmt::Result {
        self.fmt_parameters(&lambda.parameters, f)?;

        if let Some(typ) = &lambda.return_type {
            write!(f, " : ")?;
            self.fmt_type(typ, f)?;
            self.fmt_effect_clause(&lambda.effects, f)?;
        }

        write!(f, " {}", if write_arrow { "->" } else { "=" })?;
        if !matches!(self.context().exprs[lambda.body], Expr::Sequence(_)) {
            write!(f, " ")?;
        }
        self.fmt_expr(lambda.body, f)
    }

    /// Formats an effect clause with a leading space
    fn fmt_effect_clause(&self, effects: &Option<Vec<EffectType>>, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(effects) = effects {
            if effects.is_empty() {
                write!(f, " pure")?;
            } else {
                write!(f, " can ")?;
                for (i, effect) in effects.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    self.fmt_effect_type(effect, f)?;
                }
            }
        }
        Ok(())
    }

    fn fmt_effect_type(&self, effect: &EffectType, f: &mut Formatter) -> std::fmt::Result {
        match effect {
            EffectType::Known(path_id, args) => {
                self.fmt_path(*path_id, f)?;
                self.fmt_type_args(args, f)
            },
            EffectType::Variable(name_id) => self.fmt_type_name(*name_id, f),
        }
    }

    /// Formats type arguments with a leading space in front of each (including the first)
    fn fmt_type_args(&self, args: &[Type], f: &mut Formatter) -> std::fmt::Result {
        let requires_parens = |typ: &Type| matches!(typ, Type::Function(_) | Type::Application(..));

        for arg in args {
            if requires_parens(arg) {
                write!(f, " (")?;
                self.fmt_type(arg, f)?;
                write!(f, ")")?;
            } else {
                write!(f, " ")?;
                self.fmt_type(arg, f)?;
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

    fn fmt_type_definition(&mut self, type_definition: &TypeDefinition, f: &mut Formatter) -> std::fmt::Result {
        if type_definition.shared {
            write!(f, "shared ")?;
        }

        write!(f, "type ")?;
        self.fmt_type_name(type_definition.name, f)?;
        write!(f, " =")?;

        match &type_definition.body {
            TypeDefinitionBody::Error => {
                write!(f, " (error)")?;
            },
            TypeDefinitionBody::Struct(fields) => {
                self.indent_level += 1;
                for (name, typ) in fields {
                    self.newline(f)?;
                    self.fmt_type_name(*name, f)?;
                    write!(f, ": ")?;
                    self.fmt_type(typ, f)?;
                }
                self.indent_level -= 1;
            },
            TypeDefinitionBody::Enum(variants) => {
                self.indent_level += 1;
                for (name, params) in variants {
                    self.newline(f)?;
                    write!(f, "| ")?;
                    self.fmt_type_name(*name, f)?;
                    self.fmt_type_args(params, f)?;
                }
                self.indent_level -= 1;
            },
            TypeDefinitionBody::Alias(typ) => {
                write!(f, " ")?;
                self.fmt_type(typ, f)?;
            },
        }
        Ok(())
    }

    fn fmt_type(&self, typ: &Type, f: &mut Formatter) -> std::fmt::Result {
        match typ {
            Type::Error => write!(f, "(error)"),
            Type::Named(path) => self.fmt_type_path(*path, f),
            Type::Variable(name) => self.fmt_type_name(*name, f),
            Type::Unit => write!(f, "Unit"),
            Type::Integer(kind) => write!(f, "{kind}"),
            Type::Float(kind) => write!(f, "{kind}"),
            Type::Function(function_type) => self.fmt_function_type(function_type, f),
            Type::Application(constructor, args) => self.fmt_type_application(constructor, args, f),
            Type::String => write!(f, "String"),
            Type::Char => write!(f, "Char"),
            Type::Reference(mutable, shared) => self.fmt_reference_type(*mutable, *shared, f),
        }
    }

    fn fmt_reference_type(&self, mutable: Mutability, shared: Sharedness, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{mutable}{shared}")
    }

    fn fmt_type_application(&self, constructor: &Type, args: &[Type], f: &mut Formatter) -> std::fmt::Result {
        let requires_parens = |typ: &Type| matches!(typ, Type::Function(_) | Type::Application(..));

        if requires_parens(constructor) {
            write!(f, "(")?;
            self.fmt_type(constructor, f)?;
            write!(f, ")")?;
        } else {
            self.fmt_type(constructor, f)?;
        }

        self.fmt_type_args(args, f)
    }

    fn fmt_function_type(&self, function_type: &FunctionType, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "fn")?;
        self.fmt_type_args(&function_type.parameters, f)?;
        write!(f, " -> ")?;
        self.fmt_type(&function_type.return_type, f)?;
        self.fmt_effect_clause(&function_type.effects, f)
    }

    fn fmt_expr(&mut self, expr: ExprId, f: &mut Formatter) -> std::fmt::Result {
        match &self.context().exprs[expr] {
            Expr::Error => write!(f, "(error)"),
            Expr::Literal(literal) => self.fmt_literal(literal, f),
            Expr::Variable(path) => self.fmt_path(*path, f),
            Expr::Sequence(seq) => self.fmt_sequence(seq, f),
            Expr::Definition(definition) => self.fmt_definition(definition, f),
            Expr::Call(call) => self.fmt_call(call, f),
            Expr::MemberAccess(access) => self.fmt_member_access(access, f),
            Expr::Index(index) => self.fmt_index(index, f),
            Expr::Lambda(lambda) => self.fmt_lambda(lambda, f),
            Expr::If(if_) => self.fmt_if(if_, f),
            Expr::Match(match_) => self.fmt_match(match_, f),
            Expr::Handle(handle_) => self.fmt_handle(handle_, f),
            Expr::Reference(reference) => self.fmt_reference(reference, f),
            Expr::TypeAnnotation(type_annotation) => self.fmt_type_annotation(type_annotation, f),
            Expr::Quoted(quoted) => self.fmt_quoted(quoted, f),
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

    fn fmt_sequence(&mut self, seq: &[SequenceItem], f: &mut Formatter) -> std::fmt::Result {
        self.indent_level += 1;
        for item in seq {
            self.newline(f)?;
            self.fmt_comments(&item.comments, f)?;
            self.fmt_expr(item.expr, f)?;
        }
        self.indent_level -= 1;
        Ok(())
    }

    fn fmt_call(&mut self, call: &Call, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_expr(call.function, f)?;

        for arg in call.arguments.iter().copied() {
            if self.context().exprs[arg].is_atom() {
                write!(f, " ")?;
                self.fmt_expr(arg, f)?;
            } else {
                write!(f, " (")?;
                self.fmt_expr(arg, f)?;
                write!(f, ")")?;
            }
        }

        Ok(())
    }

    fn fmt_member_access(&mut self, access: &MemberAccess, f: &mut Formatter) -> std::fmt::Result {
        if self.context().exprs[access.object].is_atom() {
            self.fmt_expr(access.object, f)?;
        } else {
            write!(f, "(")?;
            self.fmt_expr(access.object, f)?;
            write!(f, ")")?;
        }

        match access.ownership {
            OwnershipMode::Owned => write!(f, ".{}", access.member),
            OwnershipMode::Borrow => write!(f, ".&{}", access.member),
            OwnershipMode::BorrowMut => write!(f, ".!{}", access.member),
        }
    }

    fn fmt_index(&mut self, index: &Index, f: &mut Formatter) -> std::fmt::Result {
        if self.context().exprs[index.object].is_atom() {
            self.fmt_expr(index.object, f)?;
        } else {
            write!(f, "(")?;
            self.fmt_expr(index.object, f)?;
            write!(f, ")")?;
        }

        match index.ownership {
            OwnershipMode::Owned => write!(f, ".[")?,
            OwnershipMode::Borrow => write!(f, ".&[")?,
            OwnershipMode::BorrowMut => write!(f, ".![")?,
        }

        self.fmt_expr(index.index, f)?;
        write!(f, "]")
    }

    fn fmt_declaration(&self, declaration: &Declaration, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_name(declaration.name, f)?;
        write!(f, ": ")?;
        self.fmt_type(&declaration.typ, f)
    }

    fn fmt_trait_definition(&mut self, trait_definition: &TraitDefinition, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "trait ")?;
        self.fmt_type_name(trait_definition.name, f)?;

        for generic in &trait_definition.generics {
            write!(f, " ")?;
            self.fmt_type_name(*generic, f)?;
        }

        if !trait_definition.functional_dependencies.is_empty() {
            write!(f, " ->")?;
            for generic in &trait_definition.functional_dependencies {
                write!(f, " ")?;
                self.fmt_type_name(*generic, f)?;
            }
        }

        write!(f, " with")?;
        self.indent_level += 1;
        for declaration in &trait_definition.body {
            self.newline(f)?;
            self.fmt_declaration(declaration, f)?;
        }
        self.indent_level -= 1;
        Ok(())
    }

    fn fmt_trait_impl(&mut self, trait_impl: &TraitImpl, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "impl ")?;
        self.fmt_type_name(trait_impl.name, f)?;
        self.fmt_parameters(&trait_impl.parameters, f)?;

        write!(f, ": ")?;
        self.fmt_path(trait_impl.trait_path, f)?;
        self.fmt_type_args(&trait_impl.trait_arguments, f)?;

        write!(f, " with")?;
        self.indent_level += 1;
        for definition in &trait_impl.body {
            self.newline(f)?;
            self.fmt_definition(definition, f)?;
        }
        self.indent_level -= 1;
        Ok(())
    }

    fn fmt_effect_definition(&mut self, effect_definition: &EffectDefinition, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "effect ")?;
        self.fmt_type_name(effect_definition.name, f)?;

        for generic in &effect_definition.generics {
            write!(f, " ")?;
            self.fmt_type_name(*generic, f)?;
        }

        write!(f, " with")?;
        self.indent_level += 1;
        for declaration in &effect_definition.body {
            self.newline(f)?;
            self.fmt_declaration(declaration, f)?;
        }
        self.indent_level -= 1;
        Ok(())
    }

    fn fmt_extern(&mut self, extern_: &Extern, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "extern ")?;
        self.fmt_declaration(&extern_.declaration, f)
    }

    fn fmt_lambda(&mut self, lambda: &Lambda, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "fn")?;
        self.fmt_lambda_inner(lambda, f, true)
    }

    fn fmt_if(&mut self, if_: &If, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "if ")?;
        self.fmt_expr(if_.condition, f)?;
        write!(f, " then ")?;
        self.fmt_expr(if_.then, f)?;

        if let Some(else_) = if_.else_ {
            write!(f, " else ")?;
            self.fmt_expr(else_, f)?;
        }
        Ok(())
    }

    fn fmt_match(&mut self, match_: &Match, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "match ")?;
        self.fmt_expr(match_.expression, f)?;

        for (pattern, branch) in &match_.cases {
            self.newline(f)?;
            write!(f, "| ")?;
            self.fmt_pattern(*pattern, f)?;
            write!(f, " -> ")?;
            self.fmt_expr(*branch, f)?;
        }

        Ok(())
    }

    fn fmt_handle(&mut self, handle_: &Handle, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "handle ")?;
        self.fmt_expr(handle_.expression, f)?;

        for (pattern, branch) in &handle_.cases {
            self.newline(f)?;
            write!(f, "| ")?;
            self.fmt_handle_pattern(pattern, f)?;
            write!(f, " -> ")?;
            self.fmt_expr(*branch, f)?;
        }

        Ok(())
    }

    fn fmt_handle_pattern(&mut self, pattern: &HandlePattern, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_name(pattern.function, f)?;
        for arg in pattern.args.iter() {
            if self.is_pattern_atom(*arg) {
                write!(f, " ")?;
                self.fmt_pattern(*arg, f)?;
            } else {
                write!(f, " (")?;
                self.fmt_pattern(*arg, f)?;
                write!(f, ")")?;
            }
        }
        Ok(())
    }

    fn fmt_pattern(&mut self, pattern: PatternId, f: &mut Formatter) -> std::fmt::Result {
        match &self.context().patterns[pattern] {
            Pattern::Variable(name) => self.fmt_name(*name, f),
            Pattern::Literal(literal) => self.fmt_literal(literal, f),
            Pattern::Constructor(path, args) => {
                self.fmt_path(*path, f)?;
                for arg in args {
                    if self.is_pattern_atom(*arg) {
                        write!(f, " ")?;
                        self.fmt_pattern(*arg, f)?;
                    } else {
                        write!(f, " (")?;
                        self.fmt_pattern(*arg, f)?;
                        write!(f, ")")?;
                    }
                }
                Ok(())
            },
            Pattern::Error => write!(f, "(error)"),
            Pattern::TypeAnnotation(pattern, typ) => {
                self.fmt_pattern(*pattern, f)?;

                // If show types is set we don't want to print annotations twice
                if !(matches!(&self.context().patterns[*pattern], Pattern::Variable(_)) && self.config.show_types) {
                    write!(f, ": ")?;
                    self.fmt_type(typ, f)?;
                }
                Ok(())
            },
            Pattern::MethodName { type_name, item_name } => {
                self.fmt_type_name(*type_name, f)?;
                write!(f, ".")?;
                self.fmt_name(*item_name, f)
            },
        }
    }

    fn fmt_reference(&mut self, reference: &Reference, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}{}", reference.mutability, reference.sharedness)?;
        if reference.sharedness != Sharedness::Shared {
            write!(f, " ")?;
        }
        self.fmt_expr(reference.rhs, f)
    }

    fn fmt_type_annotation(&mut self, type_annotation: &TypeAnnotation, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_expr(type_annotation.lhs, f)?;

        // If show types is set we don't want to print annotations twice
        if !(matches!(&self.context().exprs[type_annotation.lhs], Expr::Variable(_)) && self.config.show_types) {
            write!(f, ": ")?;
            self.fmt_type(&type_annotation.rhs, f)?;
        }
        Ok(())
    }

    fn fmt_comptime(&mut self, comptime: &Comptime, f: &mut Formatter) -> std::fmt::Result {
        match comptime {
            Comptime::Expr(expr_id) => {
                write!(f, "#")?;
                self.fmt_expr(*expr_id, f)
            },
            Comptime::Derive(paths) => {
                write!(f, "derive")?;
                for path in paths {
                    write!(f, " ")?;
                    self.fmt_path(*path, f)?;
                }
                Ok(())
            },
            Comptime::Definition(definition) => {
                write!(f, "#")?;
                self.fmt_definition(definition, f)
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
    fn is_pattern_atom(&self, pattern: PatternId) -> bool {
        use Pattern::*;
        match &self.context().patterns[pattern] {
            Error | Variable(_) | Literal(_) | MethodName { .. } => true,
            Constructor(_, args) => args.is_empty(),
            TypeAnnotation(_, _) => false,
        }
    }

    fn fmt_parameters(&mut self, parameters: &[Parameter], f: &mut Formatter) -> std::fmt::Result {
        for parameter in parameters {
            write!(f, " ")?;
            if parameter.implicit {
                write!(f, "{{")?;
                self.fmt_pattern(parameter.pattern, f)?;
                write!(f, "}}")?;
            } else if self.is_pattern_atom(parameter.pattern) {
                self.fmt_pattern(parameter.pattern, f)?;
            } else {
                write!(f, "(")?;
                self.fmt_pattern(parameter.pattern, f)?;
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
