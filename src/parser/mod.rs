use std::{collections::BTreeMap, sync::Arc};

use cst::{
    Comptime, Declaration, EffectType, Index, Lambda, MemberAccess, Mutability, Name, OwnershipMode, Parameter,
    Pattern, Sharedness,
};
use ids::{ExprId, NameId, PathId, PatternId, TopLevelId};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::{Diagnostic, ErrorDefault, Location, Span},
    incremental,
    iterator_extensions::vecmap,
    lexer::{Lexer, token::Token},
    name_resolution::namespace::SourceFileId,
    parser::{context::TopLevelContext, cst::{Constructor, HandlePattern}},
};

use self::cst::{
    Call, Cst, Definition, Expr, Import, Literal, Path, SequenceItem, TopLevelItem, TopLevelItemKind, Type,
    TypeDefinition, TypeDefinitionBody,
};

pub mod context;
pub mod cst;
pub mod cst_printer;
pub mod get_item;
pub mod ids;

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ParseResult {
    pub cst: Cst,
    pub top_level_data: BTreeMap<TopLevelId, Arc<TopLevelContext>>,
}

type Result<T> = std::result::Result<T, Diagnostic>;

struct Parser<'tokens> {
    file_id: SourceFileId,
    tokens: &'tokens [(Token, Span)],
    diagnostics: Vec<Diagnostic>,
    top_level_data: BTreeMap<TopLevelId, Arc<TopLevelContext>>,

    /// Keep track of any name collisions in the top level items
    top_level_item_hashes: FxHashSet<u64>,

    current_context: TopLevelContext,

    token_index: usize,
}

pub fn parse_impl(ctx: &incremental::Parse, db: &incremental::DbHandle) -> Arc<ParseResult> {
    let file = ctx.0.get(db);
    let tokens = Lexer::new(&file.contents).collect::<Vec<_>>();
    Arc::new(Parser::new(ctx.0, &tokens).parse(db))
}

impl<'tokens> Parser<'tokens> {
    fn new(file_id: SourceFileId, tokens: &'tokens [(Token, Span)]) -> Self {
        Self {
            file_id,
            tokens,
            diagnostics: Vec::new(),
            token_index: 0,
            top_level_data: Default::default(),
            top_level_item_hashes: Default::default(),
            current_context: TopLevelContext::new(file_id),
        }
    }

    fn parse(mut self, db: &incremental::DbHandle) -> ParseResult {
        let imports = self.parse_imports();
        let top_level_items = self.parse_top_level_items();
        self.accept(Token::Newline);
        let ending_comments = self.parse_comments();
        let cst = Cst { imports, top_level_items, ending_comments };
        for diagnostic in self.diagnostics {
            db.accumulate(diagnostic);
        }
        ParseResult { cst, top_level_data: self.top_level_data }
    }

    fn current_token(&self) -> &'tokens Token {
        &self.tokens[self.token_index].0
    }

    fn peek_next_token(&self) -> &'tokens Token {
        &self.tokens[self.token_index + 1].0
    }

    fn current_token_span(&self) -> Span {
        self.tokens[self.token_index].1
    }

    fn current_token_location(&self) -> Location {
        self.current_token_span().in_file(self.file_id)
    }

    /// True if we are at (or past) the end of input
    fn at_end_of_input(&self) -> bool {
        // The +1 accounts for the last token being `Token::EndOfInput`
        self.token_index + 1 >= self.tokens.len()
    }

    /// Returns the previous token, if it exists.
    /// Returns the current token otherwise.
    fn previous_token(&self) -> &'tokens Token {
        &self.tokens[self.token_index.saturating_sub(1)].0
    }

    /// Returns the previous token's span, if it exists.
    /// Returns the current token's span otherwise.
    fn previous_token_span(&self) -> Span {
        self.tokens[self.token_index.saturating_sub(1)].1
    }

    /// Returns the previous token's location, if it exists.
    /// Returns the current token's location otherwise.
    fn previous_token_location(&self) -> Location {
        self.previous_token_span().in_file(self.file_id)
    }

    fn current_token_and_span(&self) -> &'tokens (Token, Span) {
        &self.tokens[self.token_index]
    }

    fn advance(&mut self) {
        self.token_index += 1;
        assert!(self.token_index < self.tokens.len(), "Parser advanced pass the end of input!");
    }

    /// Advance the input if the current token matches the given token.
    /// Returns true if we advanced the input.
    fn accept(&mut self, token: Token) -> bool {
        if *self.current_token() == token {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Advance the input if the current token matches the given token, or error otherwise.
    fn expect(&mut self, token: Token, message: &'static str) -> Result<()> {
        if self.accept(token) {
            Ok(())
        } else {
            self.expected(message)
        }
    }

    /// Return a `ParserExpected` error.
    /// Uses the current token as the actual token for comparison and for the location for the error.
    fn expected<T>(&self, message: impl Into<String>) -> Result<T> {
        let message = message.into();
        let actual = self.current_token().clone();
        let location = self.current_token_location();
        Err(Diagnostic::ParserExpected { message, actual, location })
    }

    /// Reserve a space for an expression.
    /// This can be more cache efficient to reserve the spaces for parent expressions
    /// before their children.
    fn reserve_expr(&mut self) -> ExprId {
        let id = self.current_context.exprs.push(Expr::Error);
        let id2 = self.current_context.expr_locations.push(self.current_token_location());
        assert_eq!(id, id2);
        id
    }

    fn insert_expr(&mut self, id: ExprId, expr: Expr, location: Location) {
        self.current_context.exprs[id] = expr;
        self.current_context.expr_locations[id] = location;
    }

    fn push_expr(&mut self, expr: Expr, location: Location) -> ExprId {
        let id = self.current_context.exprs.push(expr);
        let id2 = self.current_context.expr_locations.push(location);
        assert_eq!(id, id2);
        id
    }

    fn reserve_pattern(&mut self) -> PatternId {
        let id = self.current_context.patterns.push(Pattern::Error);
        let id2 = self.current_context.pattern_locations.push(self.current_token_location());
        assert_eq!(id, id2);
        id
    }

    fn insert_pattern(&mut self, id: PatternId, pattern: Pattern, location: Location) {
        self.current_context.patterns[id] = pattern;
        self.current_context.pattern_locations[id] = location;
    }

    fn push_pattern(&mut self, pattern: Pattern, location: Location) -> PatternId {
        let id = self.current_context.patterns.push(pattern);
        let id2 = self.current_context.pattern_locations.push(location);
        assert_eq!(id, id2);
        id
    }

    fn push_path(&mut self, path: Path, location: Location) -> PathId {
        let id = self.current_context.paths.push(path);
        let id2 = self.current_context.path_locations.push(location);
        assert_eq!(id, id2);
        id
    }

    fn push_name(&mut self, name: Name, location: Location) -> NameId {
        let id = self.current_context.names.push(name);
        let id2 = self.current_context.name_locations.push(location);
        assert_eq!(id, id2);
        id
    }

    /// Return a hash of the given data guaranteed to be unique within the current module.
    fn hash_top_level_data(top_level_item_hashes: &mut FxHashSet<u64>, data: &impl std::hash::Hash) -> u64 {
        for collisions in 0.. {
            let hash = ids::hash((data, collisions));
            if top_level_item_hashes.insert(hash) {
                return hash;
            }
        }
        unreachable!()
    }

    /// Create a new TopLevelId from the name of a given top level item.
    /// In the case of definitions, this name will be only the last element in their path.
    fn new_top_level_id(&mut self, data: impl std::hash::Hash) -> TopLevelId {
        let hash = Self::hash_top_level_data(&mut self.top_level_item_hashes, &data);
        self.new_top_level_id_helper(hash)
    }

    /// Create a new TopLevelId from the name of a given top level item.
    /// This is a specialized version to avoid cloning the string given by the given NameId.
    fn new_top_level_id_from_name_id(&mut self, name: NameId) -> TopLevelId {
        let data = &self.current_context.names[name];
        let hash = Self::hash_top_level_data(&mut self.top_level_item_hashes, data);
        self.new_top_level_id_helper(hash)
    }

    /// Create a new TopLevelId from the name of a given top level item.
    fn new_top_level_id_from_pattern_id(&mut self, pattern: PatternId) -> TopLevelId {
        let hash = match &self.current_context.patterns[pattern] {
            Pattern::Variable(name) | Pattern::MethodName { type_name: _, item_name: name } => {
                let data = &self.current_context.names[*name];
                Self::hash_top_level_data(&mut self.top_level_item_hashes, data)
            },
            // Default to a nonsense hash and rely on collision detection to deduplicate it
            _ => Self::hash_top_level_data(&mut self.top_level_item_hashes, &()),
        };

        self.new_top_level_id_helper(hash)
    }

    /// Create a new TopLevelId from the path of a given top level item.
    /// This is a specialized version to avoid cloning the string given by the given PathId.
    fn new_top_level_id_from_path_id(&mut self, path: PathId) -> TopLevelId {
        let data = &self.current_context.paths[path];
        let hash = Self::hash_top_level_data(&mut self.top_level_item_hashes, data);
        self.new_top_level_id_helper(hash)
    }

    fn new_top_level_id_helper(&mut self, hash: u64) -> TopLevelId {
        let id = TopLevelId::new(self.file_id, hash);
        let empty_context = TopLevelContext::new(self.file_id);
        let old_context = std::mem::replace(&mut self.current_context, empty_context);
        self.top_level_data.insert(id, Arc::new(old_context));
        id
    }

    /// Return the location of an ExprId within the current context
    fn expr_location(&self, expr: ExprId) -> Location {
        self.current_context.expr_locations[expr].clone()
    }

    /// Skip all tokens up to the next newline (or unindent) token.
    /// If an Indent token is encountered we'll try to match indents and unindents
    /// so that any newlines in between are skipped.
    fn recover_to_next_newline(&mut self) {
        let mut indents = 0;

        loop {
            match self.current_token() {
                Token::Newline => {
                    if indents == 0 {
                        break;
                    }
                },
                Token::Indent => indents += 1,
                Token::Unindent => {
                    // Since we could recover from anywhere in the program, its possible
                    // we recover from, e.g. the middle of a function body and hit an
                    // unindent before we hit a newline.
                    if indents == 0 {
                        break;
                    } else {
                        indents -= 1;
                    }
                },
                Token::EndOfInput => return,
                _ => (),
            }
            self.advance();
        }

        while !self.at_end_of_input() && *self.current_token() != Token::Newline {
            self.advance();
        }
    }

    /// Try to recover to the target token (not consuming it), stopping
    /// early if any of the `too_far` tokens (or EOF) are found.
    /// Returns `true` if we successfully recovered, or `false` if any
    /// of the `too_far` tokens were encountered first.
    fn recover_to(&mut self, target: Token, too_far: &[Token]) -> bool {
        loop {
            let token = self.current_token();

            if *token == target {
                break true;
            } else if *token == Token::EndOfInput || too_far.contains(token) {
                break false;
            } else {
                self.advance();
            }
        }
    }

    /// Try to parse an item using the given parser. On failure,
    /// report the error and recover by skipping all tokens up to
    /// next newline or unindent token.
    ///
    /// Note that this will also attempt to match indents to unindents.
    /// So any newlines within indented blocks will be skipped until an
    /// unbalanced unindent or newline on the same indentation level is found.
    fn try_parse_or_recover_to_newline<T>(&mut self, parser: impl FnOnce(&mut Self) -> Result<T>) -> Option<T> {
        match parser(self) {
            Ok(item) => Some(item),
            Err(error) => {
                self.diagnostics.push(error);
                self.recover_to_next_newline();
                None
            },
        }
    }

    /// Run the given parse function and return its result on success.
    ///
    /// On error, try to recover to the given token, stopping short if any of
    /// the `too_far` tokens (or EOF) are found first. On a successful recovery,
    /// return the given default error value. Otherwise return the original error.
    fn parse_with_recovery<T>(
        &mut self, f: impl FnOnce(&mut Self) -> Result<T>, recover_to: Token, too_far: &[Token],
    ) -> Result<T>
    where
        T: ErrorDefault,
    {
        match f(self) {
            Ok(typ) => Ok(typ),
            Err(error) => {
                if self.recover_to(recover_to, too_far) {
                    self.diagnostics.push(error);
                    Ok(T::error_default())
                } else {
                    Err(error)
                }
            },
        }
    }

    /// Same as `parse_with_expr` but recovers with `Expr::Error` with an approximated location
    /// since `ExprId` does not implement `ErrorDefault`.
    fn parse_expr_with_recovery(
        &mut self, f: impl FnOnce(&mut Self) -> Result<ExprId>, recover_to: Token, too_far: &[Token],
    ) -> Result<ExprId> {
        match f(self) {
            Ok(typ) => Ok(typ),
            Err(error) => {
                let start = self.current_token_span();
                if self.recover_to(recover_to, too_far) {
                    self.diagnostics.push(error);
                    let end = self.current_token_span();
                    let location = start.to(&end).in_file(self.file_id);
                    let expr = self.push_expr(Expr::Error, location);
                    Ok(expr)
                } else {
                    Err(error)
                }
            },
        }
    }

    fn parse_imports(&mut self) -> Vec<Import> {
        let mut imports = Vec::new();
        self.accept(Token::Newline);

        loop {
            let position_before_comments = self.token_index;
            let comments = self.parse_comments();

            if !self.accept(Token::Import) {
                // The comments, if any, should be attached to the next top level item
                // since there is no import here.
                self.token_index = position_before_comments;
                break;
            }

            let start = self.current_token_span();
            if let Some(mut path) = self.try_parse_or_recover_to_newline(Self::parse_value_path) {
                // `import Crate.Module`
                // with just a single path component, nothing actually gets imported
                if path.components.len() < 2 {
                    // The path parser shouldn't parse empty paths
                    assert_eq!(path.components.len(), 1);
                    let location = path.components[0].1.clone();
                    self.diagnostics.push(Diagnostic::ExpectedPathForImport { location });
                    self.recover_to_next_newline();
                    continue;
                }

                let crate_name = path.components.remove(0).0;

                let mut items = Vec::with_capacity(1);
                if let Some(item) = path.components.pop() {
                    items.push(item);
                }

                // Parse any extra items `, b, c, d`
                while self.accept(Token::Comma) {
                    match self.parse_ident() {
                        Ok(name) => items.push((name, self.previous_token_location())),
                        Err(error) => self.diagnostics.push(error),
                    }
                }

                let end = self.previous_token_span();
                let location = start.to(&end).in_file(self.file_id);
                let path = path.into_file_path();
                imports.push(Import { comments, crate_name, module_path: path, items, location });
            }

            self.expect_newline_with_recovery("a newline after the import");
        }

        imports
    }

    fn expect_newline_with_recovery(&mut self, error_message: &'static str) {
        let expect_newline = |this: &mut Self| this.expect(Token::Newline, error_message);
        if self.try_parse_or_recover_to_newline(expect_newline).is_none() {
            // We should have recovered to a newline by this point, so we need to parse it again.
            // Don't error here, the only errors possible are if we recover to an Unindent or
            // the end of
            // the file.
            expect_newline(self).ok();
        }
    }

    // type_path: (typename '.')* typename
    fn parse_type_path(&mut self) -> Result<Path> {
        let location = self.current_token_location();
        let mut components = vec![(self.parse_type_name()?, location)];

        while self.accept(Token::MemberAccess) {
            let location = self.current_token_location();
            components.push((self.parse_type_name()?, location));
        }

        Ok(Path { components })
    }

    /// value_path: (typename '.')* (ident | typename)
    fn parse_value_path(&mut self) -> Result<Path> {
        let mut components = Vec::new();

        while let Ok(typename) = self.parse_type_name() {
            let location = self.previous_token_location();
            components.push((typename, location));

            if !self.accept(Token::MemberAccess) {
                return Ok(Path { components });
            }
        }

        // If we made it here we had a trailing `.` but the token after it was not a typename,
        // so it must be a variable name.
        let location = self.current_token_location();
        components.push((self.parse_ident()?, location));
        Ok(Path { components })
    }

    fn parse_top_level_items(&mut self) -> Vec<Arc<TopLevelItem>> {
        let mut items = Vec::new();

        while !self.at_end_of_input() {
            let position_before_comments = self.token_index;
            let comments = self.parse_comments();

            // We may have comments at the end of the file not attached to any top level item
            if self.at_end_of_input() {
                self.token_index = position_before_comments;
                return items;
            }

            if *self.current_token() == Token::Extern {
                self.try_parse_or_recover_to_newline(|this| this.parse_extern(comments, &mut items));
            } else if let Some(item) = self.try_parse_or_recover_to_newline(|this| this.parse_top_level_item(comments))
            {
                items.push(Arc::new(item));
            }

            // In case there is no newline at the end of the file
            if self.at_end_of_input() {
                break;
            }
            self.expect_newline_with_recovery("a newline after the top level item");
        }

        items
    }

    fn parse_top_level_item(&mut self, comments: Vec<String>) -> Result<TopLevelItem> {
        let id: TopLevelId;

        let kind = match self.current_token() {
            Token::Identifier(_) | Token::TypeName(_) | Token::ParenthesisLeft => {
                let definition = self.parse_definition()?;
                // parse_definition can eat the trailing newline
                if *self.previous_token() == Token::Newline {
                    self.token_index -= 1;
                }

                id = self.new_top_level_id_from_pattern_id(definition.pattern);
                TopLevelItemKind::Definition(definition)
            },
            Token::Shared | Token::Type => {
                let definition = self.parse_type_definition()?;
                id = self.new_top_level_id_from_name_id(definition.name);
                TopLevelItemKind::TypeDefinition(definition)
            },
            Token::Trait => {
                let trait_ = self.parse_trait_definition()?;
                id = self.new_top_level_id_from_name_id(trait_.name);
                TopLevelItemKind::TraitDefinition(trait_)
            },
            Token::Impl => {
                let impl_ = self.parse_trait_impl()?;
                id = self.new_top_level_id_from_path_id(impl_.trait_path);
                TopLevelItemKind::TraitImpl(impl_)
            },
            Token::Effect => {
                let effect = self.parse_effect_definition()?;
                id = self.new_top_level_id_from_name_id(effect.name);
                TopLevelItemKind::EffectDefinition(effect)
            },
            Token::Octothorpe => {
                let comptime = self.parse_comptime()?;
                // Hashing the whole comptime object here contains ExprIds which means this
                // top level id will not be stable if any of its contents change
                id = self.new_top_level_id(&comptime);
                TopLevelItemKind::Comptime(comptime)
            },
            _ => return self.expected("a top-level item"),
        };

        Ok(TopLevelItem { id, comments, kind })
    }

    fn parse_comments(&mut self) -> Vec<String> {
        let mut comments = Vec::new();

        while let Token::LineComment(comment) = self.current_token() {
            comments.push(comment.clone());
            self.advance();
            self.expect_newline_with_recovery("a newline after the comment");
        }

        comments
    }

    /// definition: non_function_definition | function_definition
    fn parse_definition(&mut self) -> Result<Definition> {
        match self.current_token() {
            Token::Implicit | Token::Var => self.parse_non_function_definition(),
            _ => {
                if let Ok(function) = self.try_(Self::parse_function_definition) {
                    Ok(function)
                } else {
                    self.parse_non_function_definition()
                }
            },
        }
    }

    /// non_function_definition: 'implicit'? 'mut'? pattern '=' expression
    fn parse_non_function_definition(&mut self) -> Result<Definition> {
        let implicit = self.accept(Token::Implicit);
        let mutable = self.accept(Token::Var);
        let pattern = self.parse_pattern()?;
        self.expect(Token::Equal, "`=` to begin the function body")?;

        let rhs = self
            .try_parse_or_recover_to_newline(|this| this.parse_block_or_expression())
            .unwrap_or_else(|| self.push_expr(Expr::Error, self.current_token_location()));

        Ok(Definition { implicit, mutable, pattern, rhs })
    }

    /// function_definition: function_name_pattern parameter+ (':' typ)? effects_clause '=' expression
    fn parse_function_definition(&mut self) -> Result<Definition> {
        let start_location = self.current_token_location();
        let name = self.parse_function_name_pattern()?;
        let parameters = self.parse_function_parameters()?;

        let return_type = if self.accept(Token::Colon) {
            self.parse_with_recovery(Self::parse_type, Token::Equal, &[Token::Newline, Token::Indent]).ok()
        } else {
            None
        };

        let effects = self.parse_effects_clause();
        self.expect(Token::Equal, "`=` to begin the function body")?;

        let lambda_id = self.reserve_expr();
        let body = self
            .try_parse_or_recover_to_newline(|this| this.parse_block_or_expression())
            .unwrap_or_else(|| self.push_expr(Expr::Error, self.current_token_location()));

        let lambda = Expr::Lambda(Lambda { parameters, return_type, effects, body });
        self.insert_expr(lambda_id, lambda, start_location);
        Ok(Definition { implicit: false, mutable: false, pattern: name, rhs: lambda_id })
    }

    fn parse_function_name_pattern(&mut self) -> Result<PatternId> {
        self.with_pattern_id_and_location(|this| match this.current_token() {
            Token::Identifier(_) | Token::ParenthesisLeft => this.parse_ident_id().map(Pattern::Variable),
            Token::TypeName(_) => {
                let type_name = this.parse_type_name_id()?;
                this.expect(Token::MemberAccess, "a `.` to separate this method's object type from its name")?;
                let item_name = this.parse_ident_id()?;
                Ok(Pattern::MethodName { type_name, item_name })
            },
            _ => this.expected("a definition name"),
        })
    }

    fn parse_type_definition(&mut self) -> Result<TypeDefinition> {
        let shared = self.accept(Token::Shared);
        self.expect(Token::Type, "`type`")?;
        let name = self.parse_type_name_id()?;
        let generics = self.parse_generics();
        self.expect(Token::Equal, "`=` to begin the type definition")?;
        let body = self.parse_type_body()?;
        Ok(TypeDefinition { shared, name, generics, body })
    }

    /// generics: ident*
    fn parse_generics(&mut self) -> Vec<NameId> {
        self.many0(Self::parse_ident_id)
    }

    fn parse_type_body(&mut self) -> Result<TypeDefinitionBody> {
        match self.current_token() {
            Token::Indent => self.parse_indented(Self::parse_indented_type_body),
            _ => self.parse_non_indented_type_body(),
        }
    }

    fn parse_indented_type_body(&mut self) -> Result<TypeDefinitionBody> {
        match self.current_token() {
            // struct - trailing commas are optional when we have newlines separating fields
            Token::Identifier(_) if *self.peek_next_token() == Token::Colon => {
                let fields = self.delimited(
                    |this| {
                        let field_name = this.parse_ident_id()?;
                        this.expect(Token::Colon, "a colon separating the field name from its type")?;
                        let field_type = this.parse_type()?;
                        this.accept(Token::Comma);
                        Ok((field_name, field_type))
                    },
                    Token::Newline,
                    false,
                );
                Ok(TypeDefinitionBody::Struct(fields))
            },
            // enum
            Token::Pipe => {
                let variants = self.delimited(
                    |this| {
                        this.expect(Token::Pipe, "`|`")?;
                        let variant_name = this.parse_type_name_id()?;
                        let parameters = this.many0(Self::parse_type_arg);
                        Ok((variant_name, parameters))
                    },
                    Token::Newline,
                    false,
                );
                Ok(TypeDefinitionBody::Enum(variants))
            },
            _ => match self.parse_type() {
                Ok(typ) => Ok(TypeDefinitionBody::Alias(typ)),
                Err(_) => self.expected("a field name or `|` to start this type body"),
            },
        }
    }

    fn parse_non_indented_type_body(&mut self) -> Result<TypeDefinitionBody> {
        match self.current_token() {
            // struct
            Token::Identifier(_) if *self.peek_next_token() == Token::Colon => {
                let fields = self.delimited(
                    |this| {
                        let field_name = this.parse_ident_id()?;
                        this.expect(Token::Colon, "a colon separating the field name from its type")?;
                        // Can't allow a pair type if we're using `,` as a field separator
                        let field_type = this.parse_type_no_pair()?;
                        Ok((field_name, field_type))
                    },
                    Token::Comma,
                    true,
                );
                Ok(TypeDefinitionBody::Struct(fields))
            },
            // enum
            Token::Pipe => {
                let variants = self.many0(|this| {
                    this.expect(Token::Pipe, "`|`")?;
                    let variant_name = this.parse_type_name_id()?;
                    let parameters = this.many0(Self::parse_type); // TODO: arg type
                    Ok((variant_name, parameters))
                });
                Ok(TypeDefinitionBody::Enum(variants))
            },
            _ => match self.parse_type() {
                Ok(typ) => Ok(TypeDefinitionBody::Alias(typ)),
                Err(_) => self.expected("a field name or `|` to start this type body"),
            },
        }
    }

    /// Parse an indented block using the given failable parser.
    /// On failure recovers to the unindent token and returns T::error_default.
    /// This only fails if there was no indent to begin with.
    fn parse_indented<T>(&mut self, parser: impl FnOnce(&mut Self) -> Result<T>) -> Result<T>
    where
        T: ErrorDefault,
    {
        self.expect(Token::Indent, "an indent")?;

        let result = parser(self);

        let expect_unindent = if result.is_err() {
            // If this returns false we reached the end of input without finding an unindent
            self.recover_to(Token::Unindent, &[])
        } else {
            true
        };

        if expect_unindent {
            if let Err(error) = self.expect(Token::Unindent, "an unindent") {
                // If we stopped short of the unindent, skip everything until the unindent
                self.diagnostics.push(error);
                if self.recover_to(Token::Unindent, &[]) {
                    self.advance();
                }
            }
        }

        Ok(result.unwrap_or(T::error_default()))
    }

    fn parse_type(&mut self) -> Result<Type> {
        self.parse_pair_type()
    }

    // TODO: Parse lifetime & element type
    fn parse_reference_type(&mut self) -> Result<Type> {
        let (mutability, sharedness) = match self.current_token() {
            Token::Ref => (cst::Mutability::Immutable, cst::Sharedness::Shared),
            Token::Mut => (cst::Mutability::Mutable, cst::Sharedness::Shared),
            Token::Imm => (cst::Mutability::Immutable, cst::Sharedness::Owned),
            Token::Uniq => (cst::Mutability::Mutable, cst::Sharedness::Owned),
            _ => return self.expected("a reference type"),
        };

        self.advance();
        self.parse_reference_element_type(mutability, sharedness)
    }

    fn parse_reference_element_type(&mut self, mutability: cst::Mutability, shared: cst::Sharedness) -> Result<Type> {
        match self.parse_type_application() {
            Ok(application) => Ok(Type::Application(Box::new(Type::Reference(mutability, shared)), vec![application])),
            Err(_) => Ok(Type::Reference(mutability, shared)),
        }
    }

    fn parse_function_type(&mut self) -> Result<Type> {
        self.expect(Token::Fn, "`fn` to start this function type")?;

        let mut parameters = self.many0(Self::parse_type_arg);
        if parameters.is_empty() {
            parameters.push(Type::Unit);
        }

        // Temporarily allow the closure arrow as well
        if !self.accept(Token::FatArrow) {
            self.expect(Token::RightArrow, "`->` to separate this function type's parameters from its return type")?;
        }

        let return_type = Box::new(self.parse_type()?);
        let effects = self.parse_effects_clause();

        Ok(Type::Function(cst::FunctionType { parameters, return_type, effects }))
    }

    /// The effect clause on a function or function type.
    ///
    /// effects_clause: 'can' effect_type (',' effect_type)*
    ///               | 'pure'
    ///               | %empty
    fn parse_effects_clause(&mut self) -> Option<Vec<EffectType>> {
        match self.current_token() {
            Token::Can => {
                self.advance();
                Some(self.delimited(Self::parse_effect_type, Token::Comma, false))
            },
            Token::Pure => {
                self.advance();
                Some(Vec::new())
            },
            _ => None,
        }
    }

    fn parse_effect_type(&mut self) -> Result<EffectType> {
        match self.current_token() {
            Token::TypeName(_) => {
                let path = self.parse_type_path_id()?;
                let args = self.many0(Self::parse_type_arg);
                Ok(EffectType::Known(path, args))
            },
            Token::Identifier(_) => {
                let name = self.parse_ident_id()?;
                Ok(EffectType::Variable(name))
            },
            _ => self.expected("an effect name"),
        }
    }

    /// pair_type: type_no_pair ',' pair_type
    ///          | type_no_pair
    fn parse_pair_type(&mut self) -> Result<Type> {
        let typ = self.parse_type_no_pair()?;

        // Fast path: this is not a pair type
        if *self.current_token() != Token::Comma {
            return Ok(typ);
        }

        let mut types = vec![typ];
        while self.accept(Token::Comma) {
            types.push(self.parse_type_no_pair()?);
        }

        // `,` is right-associative
        let mut typ = types.pop().expect("Should always have at least one type");
        while let Some(lhs) = types.pop() {
            typ = Type::Application(Box::new(Type::Pair), vec![lhs, typ]);
        }

        Ok(typ)
    }

    fn parse_type_no_pair(&mut self) -> Result<Type> {
        match self.current_token() {
            Token::Fn => self.parse_function_type(),
            Token::Ref | Token::Mut | Token::Imm | Token::Uniq => self.parse_reference_type(),
            _ => self.parse_type_application(),
        }
    }

    /// Parses a type application or a single type argument
    /// type_application: parse_type_arg+
    fn parse_type_application(&mut self) -> Result<Type> {
        let typ = self.parse_type_arg()?;
        let args = self.many0(Self::parse_type_arg);

        if args.is_empty() {
            Ok(typ)
        } else {
            Ok(Type::Application(Box::new(typ), args))
        }
    }

    // Parse a type in a function argument position. e.g. `a` in `Foo a b c`
    fn parse_type_arg(&mut self) -> Result<Type> {
        match self.current_token() {
            Token::IntegerType(kind) => {
                self.advance();
                Ok(Type::Integer(*kind))
            },
            Token::FloatType(kind) => {
                self.advance();
                Ok(Type::Float(*kind))
            },
            Token::TypeName(_) => {
                let path = self.parse_type_path_id()?;
                Ok(Type::Named(path))
            },
            Token::Identifier(_) => {
                let name = self.parse_ident_id()?;
                Ok(Type::Variable(name))
            },
            Token::ParenthesisLeft => {
                self.advance();
                let too_far = &[Token::Newline, Token::Indent, Token::Unindent];
                let typ = self.parse_with_recovery(Self::parse_type, Token::ParenthesisRight, too_far)?;
                self.expect(Token::ParenthesisRight, "a `)` to close the opening `(` from the parameter")?;
                Ok(typ)
            },
            _ => self.expected("a type"),
        }
    }

    fn parse_type_name(&mut self) -> Result<String> {
        match self.current_token() {
            Token::TypeName(name) => {
                self.advance();
                Ok(name.clone())
            },
            _ => self.expected("a capitalized type name"),
        }
    }

    fn parse_ident(&mut self) -> Result<String> {
        match self.current_token() {
            Token::Identifier(name) => {
                self.advance();
                Ok(name.clone())
            },
            _ => self.expected("an identifier"),
        }
    }

    /// Parse 0 or more of `parser` items
    fn many0<T>(&mut self, mut parser: impl FnMut(&mut Self) -> Result<T>) -> Vec<T> {
        let mut items = Vec::new();
        let mut last_success_position = self.token_index;
        while let Ok(item) = parser(self) {
            items.push(item);
            last_success_position = self.token_index;
        }
        self.token_index = last_success_position;
        items
    }

    /// Parse 1 or more of `parser` items
    fn many1<T>(&mut self, mut parser: impl FnMut(&mut Self) -> Result<T>) -> Result<Vec<T>> {
        let mut items = vec![parser(self)?];
        let mut last_success_position = self.token_index;
        while let Ok(item) = parser(self) {
            items.push(item);
            last_success_position = self.token_index;
        }
        self.token_index = last_success_position;
        Ok(items)
    }

    fn delimited<T>(
        &mut self, mut parser: impl FnMut(&mut Self) -> Result<T>, delimiter: Token, allow_trailing: bool,
    ) -> Vec<T> {
        let mut items = Vec::new();
        // Some parsers consume input even on failure. Save the current position here to
        // recover to it when `parser` fails to parse.
        let mut last_success_position = self.token_index;

        match parser(self) {
            Ok(item) => items.push(item),
            Err(_) => {
                self.token_index = last_success_position;
                return items;
            },
        }

        last_success_position = self.token_index;

        while self.accept(delimiter.clone()) {
            // Update the success position to include the delimiter as well
            last_success_position = self.token_index;

            match parser(self) {
                Ok(item) => {
                    items.push(item);
                    last_success_position = self.token_index;
                },
                Err(_) if allow_trailing => break,
                Err(error) => {
                    eprintln!("1 push error {:?}", error);
                    self.diagnostics.push(error);
                    break;
                },
            }
        }

        self.token_index = last_success_position;
        items
    }

    /// function_parameters: function_parameter+
    fn parse_function_parameters(&mut self) -> Result<Vec<Parameter>> {
        self.many1(Self::parse_function_parameter)
    }

    /// An impl may be a value (0 parameters) or a function (1+ parameters)
    /// function_parameters: function_parameter+
    fn parse_impl_parameters(&mut self) -> Vec<Parameter> {
        self.many0(Self::parse_function_parameter)
    }

    /// function_parameter: '{' pattern '}'
    ///                   | function_parameter_pattern
    fn parse_function_parameter(&mut self) -> Result<Parameter> {
        let (implicit, pattern) = if *self.current_token() == Token::BraceLeft {
            self.advance();
            let pattern = self.with_pattern_id_and_location(|this| {
                this.parse_with_recovery(Self::parse_pattern_inner, Token::BraceRight, &[Token::Newline, Token::Equal])
            })?;
            self.expect(Token::BraceRight, "a `}` to close the opening `{` from the implicit parameter")?;
            (true, pattern)
        } else {
            (false, self.parse_function_parameter_pattern()?)
        };

        Ok(Parameter { implicit, pattern })
    }

    fn parse_pattern(&mut self) -> Result<PatternId> {
        self.with_pattern_id_and_location(Self::parse_pattern_inner)
    }

    /// An alias for `parse_tuple_pattern` to avoid remembering which pattern has least precedence
    fn parse_pattern_inner(&mut self) -> Result<Pattern> {
        self.parse_tuple_pattern()
    }

    fn parse_function_parameter_pattern(&mut self) -> Result<PatternId> {
        self.with_pattern_id_and_location(Self::parse_function_parameter_pattern_inner)
    }

    fn parse_function_parameter_pattern_inner(&mut self) -> Result<Pattern> {
        match self.current_token() {
            Token::UnitLiteral => {
                self.advance();
                Ok(Pattern::Literal(Literal::Unit))
            },
            Token::IntegerLiteral(value, kind) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Integer(*value, *kind)))
            },
            Token::CharLiteral(value) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Char(*value)))
            },
            Token::ParenthesisLeft if self.peek_next_token().is_overloadable_operator() => {
                self.parse_ident_id().map(Pattern::Variable)
            },
            Token::ParenthesisLeft => {
                self.advance();
                let pattern = self.parse_with_recovery(
                    Self::parse_pattern_inner,
                    Token::ParenthesisRight,
                    &[Token::Newline, Token::Equal],
                )?;
                self.expect(Token::ParenthesisRight, "a `)` to close the opening `(` from the parameter")?;
                Ok(pattern)
            },
            Token::Identifier(_) => self.parse_ident_id().map(Pattern::Variable),
            Token::TypeName(_) => {
                let path = self.parse_type_path_id()?;
                Ok(Pattern::Constructor(path, Vec::new()))
            },
            _ => self.expected("a parameter"),
        }
    }

    /// tuple_pattern: type_annotation_pattern (',' type_annotation_pattern)*
    fn parse_tuple_pattern(&mut self) -> Result<Pattern> {
        let start = self.current_token_span();
        let mut pattern = self.parse_type_annotation_pattern()?;
        let pattern_end = self.previous_token_span();

        while self.accept(Token::Comma) {
            let comma_location = self.previous_token_location();
            let location = start.to(&pattern_end).in_file(self.file_id);

            let lhs = self.push_pattern(pattern, location);

            let rhs = self.with_pattern_id_and_location(Self::parse_type_annotation_pattern)?;

            let components = vec![(",".to_string(), comma_location.clone())];
            let comma_path = self.push_path(Path { components }, comma_location);
            pattern = Pattern::Constructor(comma_path, vec![lhs, rhs]);
        }

        Ok(pattern)
    }

    /// type_annotation_pattern: constructor_pattern (':' type)?
    fn parse_type_annotation_pattern(&mut self) -> Result<Pattern> {
        let start = self.current_token_span();
        let pattern = self.parse_constructor_pattern_inner()?;
        let end = self.previous_token_span();

        if self.accept(Token::Colon) {
            let location = start.to(&end).in_file(self.file_id);
            let pattern = self.push_pattern(pattern, location);
            let typ =
                self.parse_with_recovery(Self::parse_type, Token::ParenthesisRight, &[Token::Newline, Token::Equal])?;
            Ok(Pattern::TypeAnnotation(pattern, typ))
        } else {
            Ok(pattern)
        }
    }

    /// constructor_pattern: type_path function_parameter_pattern*
    ///                    | function_parameter_pattern
    fn parse_constructor_pattern_inner(&mut self) -> Result<Pattern> {
        match self.current_token() {
            Token::TypeName(_) => {
                let path = self.parse_type_path_id()?;
                let args = self.many0(Self::parse_function_parameter_pattern);
                Ok(Pattern::Constructor(path, args))
            },
            _ => self.parse_function_parameter_pattern_inner(),
        }
    }

    /// Returns the precedence of an operator along with
    /// whether or not it is right-associative.
    /// Returns None if the given Token is not an operator
    fn precedence(token: &Token) -> Option<(i8, bool)> {
        match token {
            Token::Semicolon => Some((0, false)),
            Token::ApplyRight => Some((1, false)),
            Token::ApplyLeft => Some((2, true)),
            Token::Comma => Some((3, true)),
            Token::Or => Some((4, false)),
            Token::And => Some((5, false)),
            Token::Is => Some((6, false)),
            Token::EqualEqual
            | Token::NotEqual
            | Token::GreaterThan
            | Token::LessThan
            | Token::GreaterThanOrEqual
            | Token::LessThanOrEqual => Some((7, false)),
            Token::In => Some((8, false)),
            Token::Append => Some((9, false)),
            Token::Range => Some((10, false)),
            Token::Add | Token::Subtract => Some((11, false)),
            Token::Multiply | Token::Divide | Token::Modulus => Some((12, false)),
            Token::Index => Some((14, false)),
            Token::As => Some((15, false)),
            _ => None,
        }
    }

    /// Should we push this operator onto our operator stack and keep parsing our expression?
    /// This handles the operator precedence and associativity parts of the shunting-yard algorithm.
    fn should_continue(operator_on_stack: &Token, r_prec: i8, r_is_right_assoc: bool) -> bool {
        let (l_prec, _) = Self::precedence(operator_on_stack).unwrap();

        l_prec > r_prec || (l_prec == r_prec && !r_is_right_assoc)
    }

    fn pop_operator(&mut self, operator_stack: &mut Vec<&(Token, Span)>, results: &mut Vec<ExprId>) {
        let rhs = results.pop().unwrap();
        let lhs = results.pop().unwrap();
        let location = self.expr_location(lhs).to(&self.expr_location(rhs));

        let call = self.reserve_expr();
        let function = self.reserve_expr();

        let (operator, span) = operator_stack.pop().unwrap().clone();
        let function_location = span.in_file(self.file_id);

        let components = vec![(operator.to_string(), function_location.clone())]; // TODO: Variable::operator
        let path_id = self.push_path(Path { components }, function_location.clone());
        self.insert_expr(function, Expr::Variable(path_id), function_location);

        let call_expr = Expr::Call(Call { function, arguments: vec![lhs, rhs] });
        self.insert_expr(call, call_expr, location);
        results.push(call);
    }

    fn parse_expression(&mut self) -> Result<ExprId> {
        match self.current_token() {
            Token::If => self.parse_if_expr(),
            Token::Match => self.parse_match(),
            Token::Handle => self.parse_handle(),
            Token::Loop => self.parse_loop(),
            Token::TypeName(_) => self.parse_named_constructor(),
            _ => self.parse_shunting_yard(),
        }
    }

    fn parse_named_constructor(&mut self) -> Result<ExprId> {
        let typ = self.parse_type()?;

        self.expect(Token::With, "with")?;

        let constructor_expr_id = self.reserve_expr();
        let fields = if self.current_token() == &Token::Indent {
            self.parse_indented(|this| {
                    Ok(this.delimited(Self::parse_named_constructor_field, Token::Newline, true))
                })?;
        } else {
            self.parse_named_constructor_fields()?
        };
        
        Ok(self.push_expr(Expr::Constructor(Constructor{ typ, fields }), self.expr_location(constructor_expr_id)))
    }

    // Parse a single named constructor field
    // ident ('=' expr)?
    fn parse_named_constructor_field(&mut self) -> Result<(NameId, ExprId)> {
        let field_identifier = self.parse_ident()?;
        let field_name = self.push_name(Arc::new(field_identifier), self.current_token_location());
        
        if self.accept(Token::Equal) {
            let value = self.parse_quark()?;
            Ok((field_name, value))
        } else {
            // implied field: doesn't point to a particular expression
            Ok((field_name, self.reserve_expr()))
        }
    }

    // Parse comma-separated named constructor fields.
    fn parse_named_constructor_fields(&mut self) -> Result<Vec<(NameId, ExprId)>> {
        // keep parsing while we can successfully parse constructor fields 
        // but break if we don't see a comma.
        let mut fields = Vec::new();
        while let Ok(field) = self.parse_named_constructor_field() {
            fields.push(field);

            if !self.accept(Token::Comma) {
                break;
            }
        }
        
        Ok(fields)
    }

    /// Parse an arbitrary infix expression using the shunting-yard algorithm
    fn parse_shunting_yard(&mut self) -> Result<ExprId> {
        let value = self.parse_term()?;

        let mut operator_stack: Vec<&(Token, Span)> = vec![];
        let mut results = vec![value];

        // loop while the next token is an operator
        while let Some((prec, right_associative)) = Self::precedence(self.current_token()) {
            while !operator_stack.is_empty()
                && Self::should_continue(&operator_stack[operator_stack.len() - 1].0, prec, right_associative)
            {
                self.pop_operator(&mut operator_stack, &mut results);
            }

            operator_stack.push(self.current_token_and_span());
            self.advance();

            let value = self.parse_term()?;
            results.push(value);
        }

        while !operator_stack.is_empty() {
            assert!(results.len() >= 2);
            self.pop_operator(&mut operator_stack, &mut results);
        }

        assert!(operator_stack.is_empty());
        assert!(results.len() == 1);
        Ok(results.pop().unwrap())
    }

    /// term: term ':' type
    ///     | term_inner
    fn parse_term(&mut self) -> Result<ExprId> {
        let start = self.current_token_span();
        let mut lhs = self.parse_term_inner()?;

        while self.accept(Token::Colon) {
            let typ = self.parse_type()?;
            let end = self.previous_token_span();
            let location = start.to(&end).in_file(self.file_id);

            let expr = Expr::TypeAnnotation(cst::TypeAnnotation { lhs, rhs: typ });
            lhs = self.push_expr(expr, location);
        }
        Ok(lhs)
    }

    fn parse_term_inner(&mut self) -> Result<ExprId> {
        match self.current_token() {
            Token::Subtract | Token::Ref | Token::Mut | Token::Imm | Token::Uniq | Token::At | Token::Not => {
                self.parse_left_unary()
            },
            _ => self.parse_function_call_or_atom(),
        }
    }

    fn parse_left_unary(&mut self) -> Result<ExprId> {
        match self.current_token() {
            operator @ (Token::Subtract
            | Token::Ref
            | Token::Mut
            | Token::Imm
            | Token::Uniq
            | Token::At
            | Token::Not) => {
                let call_id = self.reserve_expr();
                let function_id = self.reserve_expr();

                let operator_location = self.current_token_location();
                self.advance();
                let rhs = self.parse_left_unary()?;
                let location = operator_location.to(&self.expr_location(rhs));

                let components = vec![(operator.to_string(), operator_location.clone())];
                let path_id = self.push_path(Path { components }, operator_location.clone());
                self.insert_expr(function_id, Expr::Variable(path_id), operator_location);

                let call = Expr::Call(Call { function: function_id, arguments: vec![rhs] });
                self.insert_expr(call_id, call, location);
                Ok(call_id)
            },
            _ => self.parse_atom(),
        }
    }

    /// Very similar to `parse_unary` but excludes unary minus since otherwise
    /// we may parse `{function_name} -{arg}` instead of `{lhs} - {rhs}`.
    fn parse_function_arg(&mut self) -> Result<ExprId> {
        match self.current_token() {
            Token::At => self.with_expr_id_and_location(|this| {
                let operator_location = this.current_token_location();
                this.advance();
                let rhs = this.parse_left_unary()?;
                let components = vec![(Token::At.to_string(), operator_location.clone())];
                let path_id = this.push_path(Path { components }, operator_location.clone());
                let function = this.push_expr(Expr::Variable(path_id), operator_location);
                Ok(Expr::Call(Call { function, arguments: vec![rhs] }))
            }),
            operator @ (Token::ExclamationMark | Token::Ampersand) => {
                let mutability = match operator {
                    Token::ExclamationMark => Mutability::Mutable,
                    Token::Ampersand => Mutability::Immutable,
                    _ => unreachable!(),
                };

                self.with_expr_id_and_location(|this| {
                    this.advance();
                    let rhs = this.parse_left_unary()?;
                    let sharedness = Sharedness::Shared;
                    Ok(Expr::Reference(cst::Reference { mutability, sharedness, rhs }))
                })
            },
            _ => self.parse_atom(),
        }
    }

    /// An atom is a very small unit of parsing, but one that can still be divided further.
    /// In this case it is made up of quarks connected by `.` or unary expressions
    fn parse_atom(&mut self) -> Result<ExprId> {
        let mut result = self.parse_quark()?;

        loop {
            let token = self.current_token();
            match token {
                Token::MemberAccess | Token::MemberRef | Token::MemberMut => {
                    result = self.with_expr_id_and_location(|this| {
                        this.advance();
                        let ownership = OwnershipMode::from_token(token).unwrap();
                        let member = this.parse_ident()?;
                        Ok(Expr::MemberAccess(MemberAccess { object: result, member, ownership }))
                    })?;
                },
                Token::Index | Token::IndexRef | Token::IndexMut => {
                    result = self.with_expr_id_and_location(|this| {
                        this.advance();
                        let ownership = OwnershipMode::from_token(token).unwrap();
                        let index = this.parse_expression()?;
                        this.expect(Token::BracketRight, "a `]` to terminate the index expression")?;
                        Ok(Expr::Index(Index { object: result, index, ownership }))
                    })?;
                },
                _ => break Ok(result),
            }
        }
    }

    /// Parse an indivisible expression which is valid anywhere a value is expected
    fn parse_quark(&mut self) -> Result<ExprId> {
        match self.current_token() {
            Token::IntegerLiteral(value, kind) => {
                let (value, kind) = (*value, *kind);
                let location = self.current_token_location();
                self.advance();
                let expr = Expr::Literal(Literal::Integer(value, kind));
                Ok(self.push_expr(expr, location))
            },
            Token::StringLiteral(s) => self.parse_string(s.clone()),
            Token::CharLiteral(c) => self.parse_char(*c),
            Token::BooleanLiteral(value) => {
                let location = self.current_token_location();
                self.advance();
                let expr = Expr::Literal(Literal::Bool(*value));
                Ok(self.push_expr(expr, location))
            },
            Token::FloatLiteral(value, kind) => {
                let (value, kind) = (*value, *kind);
                let location = self.current_token_location();
                self.advance();
                let expr = Expr::Literal(Literal::Float(value, kind));
                Ok(self.push_expr(expr, location))
            },
            Token::Identifier(_) | Token::TypeName(_) => self.parse_variable(),
            Token::ParenthesisLeft => {
                self.advance();
                // These `too_far` tokens aren't accurate, they may appear in an expression.
                // What we really want is a recover with balanced parens such that if a mismatched
                // `]`, `)` or unindent is found we halt the recovery. `recover_to_next_newline` is
                // almost this.
                let too_far = &[Token::Newline, Token::Indent, Token::Unindent];
                let expr = self.parse_expr_with_recovery(Self::parse_expression, Token::ParenthesisRight, too_far)?;
                self.expect(Token::ParenthesisRight, "a `)` to close the opening `(` from the parameter")?;
                Ok(expr)
            },
            Token::UnitLiteral => {
                let location = self.current_token_location();
                self.advance();
                Ok(self.push_expr(Expr::Literal(Literal::Unit), location))
            },
            Token::Fn => self.parse_lambda(),
            _ => self.expected("an expression"),
        }
    }

    /// Parse a loop expression
    /// TODO: These aren't handled currently, we just need the stdlib to parse so we return another
    /// expression.
    fn parse_loop(&mut self) -> Result<ExprId> {
        self.with_expr_id_and_location(|this| {
            this.expect(Token::Loop, "`loop` to start a loop expression")?;
            let _parameters = this.many1(Self::loop_parameter)?;
            this.expect(Token::RightArrow, "`->` to separate the loop parameters from its body")?;
            let _body = this.parse_block_or_expression();

            // Now throw everything away because we don't have a loop Cst node.
            Ok(Expr::Error)
        })
    }

    fn loop_parameter(&mut self) -> Result<(PatternId, ExprId)> {
        match self.current_token() {
            Token::Identifier(name) => {
                // Parse the same name twice as a pattern and expression
                let arc_name = Arc::new(name.clone());
                let location = self.current_token_location();
                let name_id = self.push_name(arc_name, location.clone());
                let pattern_id = self.push_pattern(Pattern::Variable(name_id), location.clone());

                let path = Path { components: vec![(name.clone(), location.clone())] };
                let path_id = self.push_path(path, location.clone());
                let name_expr = self.push_expr(Expr::Variable(path_id), location);
                self.advance();
                Ok((pattern_id, name_expr))
            },
            Token::ParenthesisLeft => {
                self.advance();
                let pattern = self.parse_pattern()?;
                self.expect(Token::Equal, "`=` to separate the loop parameter's name from its initial valeu")?;
                let expr = self.parse_expression()?;
                self.expect(Token::ParenthesisRight, "`)` to close the opening `(` of this loop parameter")?;
                Ok((pattern, expr))
            },
            _ => self.expected("an identifier or `(` to begin a loop parameter"),
        }
    }

    fn parse_lambda(&mut self) -> Result<ExprId> {
        self.with_expr_id_and_location(|this| {
            this.expect(Token::Fn, "`fn` to start this lambda")?;
            let parameters = this.parse_function_parameters()?;

            let return_type = if this.accept(Token::Colon) {
                Some(this.parse_with_recovery(Self::parse_type, Token::RightArrow, &[Token::Newline, Token::Indent])?)
            } else {
                None
            };

            let effects = this.parse_effects_clause();

            this.expect(Token::RightArrow, "a `->` to separate this lambda's parameters from its body")?;
            let body = this.parse_expression()?;

            Ok(Expr::Lambda(Lambda { parameters, return_type, effects, body }))
        })
    }

    fn parse_sequence_item(&mut self) -> Result<SequenceItem> {
        let comments = self.parse_comments();
        let expr = self.parse_statement()?;
        Ok(SequenceItem { comments, expr })
    }

    /// Run the given parser, resetting to the original token position on error.
    ///
    /// Useful for parsers which parse some input, then fail without restoring the
    /// previous token index. Otherwise we could end up in a state where input was
    /// skipped and not parsed.
    fn try_<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let start_position = self.token_index;
        let diagnostic_count = self.diagnostics.len();
        let result = f(self);
        if result.is_err() {
            self.token_index = start_position;
            self.diagnostics.truncate(diagnostic_count);
        }
        result
    }

    fn parse_statement(&mut self) -> Result<ExprId> {
        let start = self.current_token_span();

        if let Ok(definition) = self.try_(Self::parse_definition) {
            let end = self.previous_token_span();
            let location = start.to(&end).in_file(self.file_id);
            let expr = Expr::Definition(definition);
            return Ok(self.push_expr(expr, location));
        }

        if *self.current_token() == Token::Return {
            return self.parse_return();
        }

        let expression = self.parse_expression()?;

        // Try to parse an assignment
        if self.accept(Token::Assignment) {
            let rhs = self.parse_expression()?;
            let location = self.expr_location(expression).to(&self.expr_location(rhs));
            // TODO: CST node for assignments
            Ok(self.push_expr(Expr::Error, location))
        } else {
            Ok(expression)
        }
    }

    fn with_expr_id(&mut self, f: impl FnOnce(&mut Self) -> Result<(Expr, Location)>) -> Result<ExprId> {
        let id = self.reserve_expr();
        let (expr, location) = f(self)?;
        self.insert_expr(id, expr, location);
        Ok(id)
    }

    fn with_expr_id_and_location(&mut self, f: impl FnOnce(&mut Self) -> Result<Expr>) -> Result<ExprId> {
        self.with_expr_id(|this| this.with_location(f))
    }

    fn with_pattern_id_and_location(&mut self, f: impl FnOnce(&mut Self) -> Result<Pattern>) -> Result<PatternId> {
        let id = self.reserve_pattern();
        let (pattern, location) = self.with_location(f)?;
        self.insert_pattern(id, pattern, location);
        Ok(id)
    }

    fn parse_return(&mut self) -> Result<ExprId> {
        // TODO: Cst node for return
        self.with_expr_id_and_location(|this| {
            this.expect(Token::Return, "`return` to begin a return statement")?;
            let _expr = this.parse_block_or_expression()?;
            Ok(Expr::Error)
        })
    }

    fn parse_if(&mut self, mut body: impl Copy + FnMut(&mut Self) -> Result<ExprId>) -> Result<ExprId> {
        self.with_expr_id_and_location(|this| {
            this.expect(Token::If, "a `if` to begin an if expression")?;

            let condition =
                this.parse_expr_with_recovery(Self::parse_block_or_expression, Token::Then, &[Token::Newline])?;

            this.accept(Token::Newline);
            this.expect(Token::Then, "a `then` to end this if condition")?;

            let then = this.parse_expr_with_recovery(body, Token::Else, &[Token::Newline])?;

            // If we allow an optional newline without an else, a lone `if a then b` could
            // eat the newline meant to separate two statements.
            if *this.peek_next_token() == Token::Else {
                this.accept(Token::Newline);
            }

            let else_ = if this.accept(Token::Else) { Some(body(this)?) } else { None };
            Ok(Expr::If(cst::If { condition, then, else_ }))
        })
    }

    fn parse_if_expr(&mut self) -> Result<ExprId> {
        self.parse_if(Self::parse_block_or_expression)
    }

    /// A comptime if, unlike a regular if, requires a block so that we can quote
    /// every token until we find the matching unindent.
    fn parse_comptime_if(&mut self) -> Result<ExprId> {
        self.parse_if(Self::parse_quoted_block)
    }

    fn parse_match(&mut self) -> Result<ExprId> {
        self.with_expr_id_and_location(|this| {
            this.expect(Token::Match, "`match` to start this match expression")?;

            let expression = this.parse_expression()?;

            let cases = this.many0(|this| {
                if *this.peek_next_token() == Token::Pipe {
                    this.accept(Token::Newline);
                }

                this.expect(Token::Pipe, "a `|` to start a new pattern")?;
                let pattern = this.parse_pattern()?;
                this.expect(Token::RightArrow, "a `->` to separate the match pattern from the match branch")?;
                let branch = this.parse_block_or_expression()?;
                Ok((pattern, branch))
            });

            Ok(Expr::Match(cst::Match { expression, cases }))
        })
    }

    fn parse_handle(&mut self) -> Result<ExprId> {
        self.with_expr_id_and_location(|this| {
            this.expect(Token::Handle, "`handle` to start this match expression")?;

            let expression = this.parse_block_or_expression()?;

            let cases = this.many0(|this| {
                if *this.peek_next_token() == Token::Pipe {
                    this.accept(Token::Newline);
                }

                this.expect(Token::Pipe, "a `|` to start a new pattern")?;
                let pattern = this.parse_handle_pattern()?;
                this.expect(Token::RightArrow, "a `->` to separate the handle pattern from the match branch")?;
                let branch = this.parse_block_or_expression()?;
                Ok((pattern, branch))
            });

            Ok(Expr::Handle(cst::Handle { expression, cases }))
        })
    }

    fn parse_handle_pattern(&mut self) -> Result<HandlePattern> {
        let function_name = self.parse_ident_id()?;

        let parameter_patterns = self.many0(|this| this.parse_pattern());

        Ok(cst::HandlePattern { function: function_name, args: parameter_patterns })
    }

    /// Parse an indent followed by any arbitrary tokens until a matching unindent
    fn parse_quoted_block(&mut self) -> Result<ExprId> {
        self.expect(Token::Indent, "an indent to start a quoted block")?;
        let mut indent_count = 0;
        let mut tokens = Vec::new();

        self.with_expr_id_and_location(|this| {
            loop {
                this.advance();
                match this.current_token() {
                    Token::Indent => {
                        indent_count += 1;
                        tokens.push(Token::Indent);
                    },
                    Token::Unindent => {
                        if indent_count == 0 {
                            break;
                        }
                        indent_count -= 1;
                    },
                    // This should be unreachable since the lexer should guarantee indents are
                    // always matched.
                    Token::EndOfInput => break,
                    other => tokens.push(other.clone()),
                }
            }
            Ok(Expr::Quoted(cst::Quoted { tokens }))
        })
    }

    fn parse_block_or_expression(&mut self) -> Result<ExprId> {
        match self.current_token() {
            Token::Indent => self.parse_block(),
            Token::Return => self.parse_return(),
            _ => self.parse_expression(),
        }
    }

    fn parse_block(&mut self) -> Result<ExprId> {
        let (expr, location) = self.with_location(|this| {
            this.parse_indented(|this| {
                let statements = this.delimited(Self::parse_sequence_item, Token::Newline, true);
                Ok(Expr::Sequence(statements))
            })
        })?;
        Ok(self.push_expr(expr, location))
    }

    /// Create a location from the current token before running the given parse function to the
    /// current token (end exclusive) after running the given parse function.
    fn with_location<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<(T, Location)> {
        let start = self.current_token_span();
        let ret = f(self)?;
        let end = self.previous_token_span();
        Ok((ret, start.to(&end).in_file(self.file_id)))
    }

    fn parse_variable(&mut self) -> Result<ExprId> {
        let path = self.parse_value_path_id()?;
        let location = self.current_context.path_locations[path].clone();
        Ok(self.push_expr(Expr::Variable(path), location))
    }

    fn parse_function_call_or_atom(&mut self) -> Result<ExprId> {
        let function = self.parse_atom()?;

        if let Ok(arguments) = self.many1(Self::parse_function_arg) {
            let last_arg_location = self.expr_location(*arguments.last().unwrap());
            let location = self.expr_location(function).to(&last_arg_location);
            let call = Expr::Call(Call { function, arguments });
            Ok(self.push_expr(call, location))
        } else {
            Ok(function)
        }
    }

    fn parse_char(&mut self, char: char) -> Result<ExprId> {
        let location = self.current_token_location();
        self.advance();
        Ok(self.push_expr(Expr::Literal(Literal::Char(char)), location))
    }

    fn parse_string(&mut self, contents: String) -> Result<ExprId> {
        let location = self.current_token_location();
        self.advance();
        Ok(self.push_expr(Expr::Literal(Literal::String(contents)), location))
    }

    fn parse_comptime(&mut self) -> Result<Comptime> {
        // Skip `#`
        self.advance();

        match self.current_token() {
            Token::If => {
                let if_ = self.parse_comptime_if()?;
                Ok(Comptime::Expr(if_))
            },
            Token::Identifier(_) | Token::TypeName(_) => {
                let call = self.parse_expr_with_recovery(Self::parse_function_call_or_atom, Token::Newline, &[])?;
                Ok(Comptime::Expr(call))
            },
            _ => self.expected("a compile-time item"),
        }
    }

    fn parse_type_path_id(&mut self) -> Result<PathId> {
        let (path, location) = self.with_location(Self::parse_type_path)?;
        Ok(self.push_path(path, location))
    }

    fn parse_value_path_id(&mut self) -> Result<PathId> {
        let (path, location) = self.with_location(Self::parse_value_path)?;
        Ok(self.push_path(path, location))
    }

    /// ident_id: ident
    ///         | '(' overloadable_operator ')'
    fn parse_ident_id(&mut self) -> Result<NameId> {
        match self.current_token() {
            Token::Identifier(name) => {
                let name = Arc::new(name.clone());
                let location = self.previous_token_location();
                self.advance();
                Ok(self.push_name(name, location))
            },
            Token::ParenthesisLeft if self.peek_next_token().is_overloadable_operator() => {
                self.advance();
                let location = self.current_token_location();
                let name = Arc::new(self.current_token().to_string());
                self.advance();
                self.expect(Token::ParenthesisRight, "`)` to close the opening `(`")?;
                Ok(self.push_name(name, location))
            },
            _ => self.expected("an identifier"),
        }
    }

    fn parse_type_name_id(&mut self) -> Result<NameId> {
        let name = Arc::new(self.parse_type_name()?);
        let location = self.previous_token_location();
        Ok(self.push_name(name, location))
    }

    fn parse_declaration(&mut self) -> Result<Declaration> {
        let name = self.parse_ident_id()?;
        self.expect(Token::Colon, "a `:` to separate this declaration's name from its type")?;
        let typ = self.parse_type()?;
        Ok(Declaration { name, typ })
    }

    fn parse_extern(&mut self, mut comments: Vec<String>, items: &mut Vec<Arc<TopLevelItem>>) -> Result<()> {
        self.expect(Token::Extern, "`extern` to begin external declarations")?;

        let declarations = if *self.current_token() == Token::Indent {
            self.parse_indented(|this| {
                let declaration = |this: &mut Self| {
                    comments.extend(this.parse_comments());
                    let declaration = this.parse_declaration()?;
                    let id = this.new_top_level_id_from_name_id(declaration.name);
                    Ok((std::mem::take(&mut comments), id, declaration))
                };
                Ok(this.delimited(declaration, Token::Newline, true))
            })?
        } else {
            let declaration = self.parse_declaration()?;
            let id = self.new_top_level_id_from_name_id(declaration.name);
            vec![(comments, id, declaration)]
        };

        for (comments, id, declaration) in declarations {
            let kind = TopLevelItemKind::Extern(cst::Extern { declaration });
            items.push(Arc::new(TopLevelItem { id, comments, kind }));
        }

        Ok(())
    }

    fn parse_declaration_block(&mut self) -> Result<Vec<Declaration>> {
        if *self.current_token() == Token::Indent {
            self.parse_indented(|this| Ok(this.delimited(Self::parse_declaration, Token::Newline, true)))
        } else {
            Ok(vec![self.parse_declaration()?])
        }
    }

    fn parse_trait_definition(&mut self) -> Result<cst::TraitDefinition> {
        self.expect(Token::Trait, "`trait` to start this trait definition")?;
        let name = self.parse_type_name_id()?;
        let generics = self.parse_generics();

        let functional_dependencies = if self.accept(Token::RightArrow) { self.parse_generics() } else { Vec::new() };

        self.expect(Token::With, "`with` to separate this trait's signature from its body")?;
        let body = self.parse_declaration_block()?;

        Ok(cst::TraitDefinition { name, generics, functional_dependencies, body })
    }

    fn parse_trait_impl(&mut self) -> Result<cst::TraitImpl> {
        self.expect(Token::Impl, "`impl` to start this trait implementation")?;

        let name = self.parse_ident_id()?;
        let parameters = self.parse_impl_parameters();

        self.accept(Token::Newline);
        self.expect(Token::Colon, "a `:` to separate this impl's name from its type")?;

        let trait_path = self.parse_type_path_id()?;
        let trait_arguments = self.many0(Self::parse_type_arg);
        self.expect(Token::With, "`with` to separate this trait impl's signature from its body")?;

        let body =
            vecmap(self.parse_impl_body()?, |definition| match &self.current_context.patterns[definition.pattern] {
                Pattern::Variable(name) => (*name, definition.rhs),
                _ => {
                    let location = self.current_context.pattern_locations[definition.pattern].clone();
                    let name = self.push_name(Arc::new("(placeholder)".to_string()), location.clone());
                    self.diagnostics.push(Diagnostic::ParserComplexImplItemName { location });
                    (name, definition.rhs)
                },
            });
        Ok(cst::TraitImpl { name, parameters, trait_path, trait_arguments, body })
    }

    fn parse_impl_body(&mut self) -> Result<Vec<Definition>> {
        match self.current_token() {
            Token::Indent => {
                self.parse_indented(|this| Ok(this.delimited(Self::parse_definition, Token::Newline, true)))
            },
            _ => self.parse_definition().map(|definition| vec![definition]),
        }
    }

    fn parse_effect_definition(&mut self) -> Result<cst::EffectDefinition> {
        self.expect(Token::Effect, "`effect` to start this effect definition")?;
        let name = self.parse_type_name_id()?;
        let generics = self.parse_generics();

        self.expect(Token::With, "`with` to separate this effect's signature from its body")?;
        let body = self.parse_declaration_block()?;

        Ok(cst::EffectDefinition { name, generics, body })
    }
}
