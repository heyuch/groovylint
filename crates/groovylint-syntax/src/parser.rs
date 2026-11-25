//! Recursive-descent parser for the supported Groovy subset.
//!
//! This is intentionally not a full Groovy parser. It covers the constructs
//! needed by the first rule slice and common class/script shapes, and will be
//! extended alongside the rule set.

use groovylint_core::Span;

use crate::ast::{
    Annotation, AssertStmt, AssignOp, BinaryOp, Block, CatchClause, ClassDecl, ClassKind,
    DoWhileStmt, Expr, ExprKind, FieldDecl, ForEachStmt, ForStmt, IfStmt, ImportDecl,
    InitializerDecl, Item, Literal, MethodDecl, PackageDecl, Param, ReturnStmt, Script, Stmt,
    SwitchCase, SwitchStmt, SynchronizedStmt, ThrowStmt, TryStmt, UnaryOp, VarDecl, WhileStmt,
};
use crate::lexer::{LexError, lex};
use crate::token::{Token, TokenKind};

/// A parser error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    /// Human-readable message.
    pub message: String,
    /// Error location.
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(error: LexError) -> Self {
        Self {
            message: error.message,
            span: error.span,
        }
    }
}

/// The parsed script and the token stream used to produce it.
#[derive(Clone, Debug)]
pub struct ParseOutput {
    /// Parsed syntax tree.
    pub script: Script,
    /// Original token stream.
    pub tokens: Vec<Token>,
}

/// Parses Groovy source.
pub fn parse(source: &str) -> Result<ParseOutput, ParseError> {
    let tokens = lex(source)?;
    let script = Parser::new(source, &tokens).parse_script()?;
    Ok(ParseOutput { script, tokens })
}

struct Parser<'a> {
    source: &'a str,
    tokens: &'a [Token],
    pos: usize,
    /// Nesting depth of `(...)` and `[...]`, where newlines are insignificant.
    nesting: usize,
    last_type_annotations: Vec<String>,
    /// Declaration kind of the class body being parsed.
    class_kind: Option<ClassKind>,
}

impl<'a> Parser<'a> {
    const fn new(source: &'a str, tokens: &'a [Token]) -> Self {
        Self {
            source,
            tokens,
            pos: 0,
            nesting: 0,
            last_type_annotations: Vec::new(),
            class_kind: None,
        }
    }

    fn parse_script(&mut self) -> Result<Script, ParseError> {
        let start = self.current().span.start;
        self.skip_separators();

        let mut package = None;
        let mut imports = Vec::new();
        let mut items = Vec::new();

        while !self.at(TokenKind::Eof) {
            if self.at(TokenKind::Package) && package.is_none() && items.is_empty() {
                package = Some(self.parse_package()?);
            } else if self.at(TokenKind::Import) {
                imports.push(self.parse_import()?);
            } else {
                items.push(self.parse_item(false)?);
            }
            self.skip_separators();
        }

        Ok(Script {
            package,
            imports,
            items,
            span: Span::new(start, self.current().span.end),
        })
    }

    fn parse_package(&mut self) -> Result<PackageDecl, ParseError> {
        let start = self.expect(TokenKind::Package)?.span.start;
        let name = self.parse_qualified_name()?;
        self.eat(TokenKind::Semicolon);
        self.skip_newlines();
        Ok(PackageDecl {
            name,
            span: Span::new(start, self.previous_span().end),
        })
    }

    fn parse_import(&mut self) -> Result<ImportDecl, ParseError> {
        let start = self.expect(TokenKind::Import)?.span.start;
        let is_static = self.eat(TokenKind::Static);
        let mut path = self.parse_qualified_name()?;
        if self.eat(TokenKind::Dot) {
            self.expect(TokenKind::Star)?;
            path.push_str(".*");
        }
        let alias = if self.eat(TokenKind::As) {
            Some(self.expect_ident()?.text(self.source).to_string())
        } else {
            None
        };
        self.eat(TokenKind::Semicolon);
        self.skip_newlines();
        Ok(ImportDecl {
            path,
            alias,
            is_static,
            span: Span::new(start, self.previous_span().end),
        })
    }

    fn parse_item(&mut self, in_class: bool) -> Result<Item, ParseError> {
        if self.at(TokenKind::At) && self.peek_kind(1) == TokenKind::Interface {
            return self.parse_annotation_type().map(Item::Class);
        }

        let prefix = if !in_class && self.at(TokenKind::Synchronized) {
            PrefixInfo::default()
        } else {
            self.parse_prefix()?
        };
        let start = self.current().span.start;

        if self.at(TokenKind::Lt) {
            self.skip_balanced(TokenKind::Lt, TokenKind::Gt)?;
        }

        if self.at(TokenKind::At) && self.peek_kind(1) == TokenKind::Interface {
            return self.parse_annotation_type().map(Item::Class);
        }

        if self.is_class_keyword() {
            return self.parse_class(start, prefix).map(Item::Class);
        }

        if in_class && self.at(TokenKind::LeftBrace) {
            let body = self.parse_block()?;
            let span = body.span;
            return Ok(Item::Initializer(InitializerDecl {
                is_static: prefix.is_static,
                body,
                span,
            }));
        }

        // A declaration without a type: `private static SALT = "salt"`.
        if in_class && self.at_ident_like() && self.peek_kind(1) == TokenKind::Assign {
            let name = self.expect_ident_text()?;
            return self.parse_field_or_var(start, name, true, None, &prefix);
        }

        if self.at(TokenKind::Def) {
            self.advance();
            if self.at(TokenKind::Lt) {
                self.skip_balanced(TokenKind::Lt, TokenKind::Gt)?;
            }
            if self.at_ident_like() && self.peek_kind(1) == TokenKind::LeftParen {
                return self.parse_method(start, prefix, None).map(Item::Method);
            }
            let saved = self.pos;
            if let Ok(type_name) = self.parse_type_name()
                && is_type_like(&type_name)
                && self.at_ident_like()
            {
                if self.peek_kind(1) == TokenKind::LeftParen {
                    let method_prefix = prefix;
                    return self
                        .parse_method(start, method_prefix, Some(type_name))
                        .map(Item::Method);
                }
                let name = self.expect_ident_text()?;
                return self.parse_field_or_var(
                    start,
                    name,
                    in_class,
                    Some(type_name.as_str()),
                    &prefix,
                );
            }
            self.pos = saved;
            let name = self.expect_ident_text()?;
            return self.parse_field_or_var(start, name, in_class, None, &prefix);
        }

        if in_class && self.at(TokenKind::String) && self.peek_kind(1) == TokenKind::LeftParen {
            return self.parse_method(start, prefix, None).map(Item::Method);
        }

        if self.at(TokenKind::Void) {
            self.advance();
            return self
                .parse_method(start, prefix, Some("void".to_string()))
                .map(Item::Method);
        }

        if self.at(TokenKind::Ident) {
            if in_class && self.peek_kind(1) == TokenKind::LeftParen {
                return self.parse_method(start, prefix, None).map(Item::Method);
            }

            let saved = self.pos;
            if let Ok(type_name) = self.parse_type_name()
                && is_type_like(&type_name)
            {
                self.skip_newlines_before_method_name();
                if self.at_ident_like() {
                    if self.peek_kind(1) == TokenKind::LeftParen {
                        let method_prefix = prefix;
                        return self
                            .parse_method(start, method_prefix, Some(type_name))
                            .map(Item::Method);
                    }
                    let name = self.expect_ident_text()?;
                    return self.parse_field_or_var(
                        start,
                        name,
                        in_class,
                        Some(type_name.as_str()),
                        &prefix,
                    );
                }
            }
            self.pos = saved;
        }

        self.parse_statement().map(Item::Statement)
    }

    fn parse_anonymous_body(&mut self) -> Result<Vec<Item>, ParseError> {
        self.expect(TokenKind::LeftBrace)?;
        let mut members = Vec::new();
        self.skip_separators();
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            members.push(self.parse_item(true)?);
            self.skip_separators();
        }
        self.expect(TokenKind::RightBrace)?;
        Ok(members)
    }

    fn parse_annotation_type(&mut self) -> Result<ClassDecl, ParseError> {
        let start = self.current().span.start;
        self.expect(TokenKind::At)?;
        self.expect(TokenKind::Interface)?;
        let name = self.expect_ident_text()?;
        while !self.at(TokenKind::LeftBrace) && !self.at(TokenKind::Eof) {
            self.advance();
        }
        self.skip_balanced(TokenKind::LeftBrace, TokenKind::RightBrace)?;
        Ok(ClassDecl {
            name,
            kind: ClassKind::Annotation,
            is_abstract: false,
            is_final: false,
            is_public: false,
            is_protected: false,
            extends: None,
            implements: Vec::new(),
            annotations: Vec::new(),
            annotation_details: Vec::new(),
            suppressed_rules: Vec::new(),
            members: Vec::new(),
            span: Span::new(start, self.previous_span().end),
        })
    }

    fn parse_class(&mut self, start: usize, prefix: PrefixInfo) -> Result<ClassDecl, ParseError> {
        let kind = if self.eat(TokenKind::Class) {
            ClassKind::Class
        } else if self.eat(TokenKind::Interface) {
            ClassKind::Interface
        } else if self.eat(TokenKind::Trait) {
            ClassKind::Trait
        } else if self.eat(TokenKind::Enum) {
            ClassKind::Enum
        } else {
            self.expect(TokenKind::Record)?;
            ClassKind::Record
        };

        let name = self.expect_ident_text()?;
        if self.at(TokenKind::Lt) {
            self.skip_balanced(TokenKind::Lt, TokenKind::Gt)?;
        }

        let mut extends = None;
        let mut implements = Vec::new();
        while !self.at(TokenKind::LeftBrace) && !self.at(TokenKind::Eof) {
            if self.eat(TokenKind::Extends) {
                self.skip_newlines();
                extends = Some(self.parse_type_name()?);
            } else if self.eat(TokenKind::Implements) {
                self.skip_newlines();
                implements.push(self.parse_type_name()?);
                loop {
                    self.skip_newlines();
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                    self.skip_newlines();
                    implements.push(self.parse_type_name()?);
                }
            } else {
                self.advance();
            }
        }

        let _ = self.expect(TokenKind::LeftBrace)?;
        let mut members = Vec::new();
        let previous_kind = self.class_kind;
        self.class_kind = Some(kind);
        self.skip_separators();

        if kind == ClassKind::Enum {
            members = self.parse_enum_constants(&name);
            self.skip_separators();
        }

        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            if self.at(TokenKind::Semicolon) {
                self.advance();
                self.skip_separators();
                continue;
            }
            members.push(self.parse_item(true)?);
            self.skip_separators();
        }

        self.expect(TokenKind::RightBrace)?;
        self.class_kind = previous_kind;
        Ok(ClassDecl {
            name,
            kind,
            is_abstract: prefix.is_abstract,
            is_final: prefix.is_final,
            is_public: prefix.is_public,
            is_protected: prefix.is_protected,
            extends,
            implements,
            annotations: prefix.annotations,
            annotation_details: prefix.annotation_details,
            suppressed_rules: prefix.suppressed_rules,
            members,
            span: Span::new(start, self.previous_span().end),
        })
    }

    /// Consumes the enum constant list. Constants are `NAME`, `NAME(args)` or
    /// `NAME { ... }`, separated by commas and optionally terminated by `;`.
    /// Everything after them is parsed as regular class members.
    /// Parses the enum constant list and models each constant as a field.
    /// Groovy represents them as `public static final` fields of the enum type.
    fn parse_enum_constants(&mut self, enum_name: &str) -> Vec<Item> {
        let mut constants = Vec::new();
        loop {
            self.skip_separators();
            // Constants may be annotated; rewind when what follows is a member.
            let checkpoint = self.pos;
            let prefix = self.parse_prefix().unwrap_or_default();
            if !self.at_ident_like() {
                self.pos = checkpoint;
                return constants;
            }
            let next = self.peek_kind(1);
            let is_constant = matches!(
                next,
                TokenKind::LeftParen
                    | TokenKind::LeftBrace
                    | TokenKind::Comma
                    | TokenKind::Semicolon
                    | TokenKind::Newline
                    | TokenKind::At
                    | TokenKind::RightBrace
                    | TokenKind::Eof
            );
            if !is_constant {
                self.pos = checkpoint;
                return constants;
            }

            let token = self.advance();
            let start = token.span.start;
            if self.at(TokenKind::LeftParen) {
                let _ = self.skip_balanced(TokenKind::LeftParen, TokenKind::RightParen);
            }
            if self.at(TokenKind::LeftBrace) {
                let _ = self.skip_balanced(TokenKind::LeftBrace, TokenKind::RightBrace);
            }
            constants.push(Item::Field(FieldDecl {
                name: token.text(self.source).to_string(),
                type_name: Some(enum_name.to_string()),
                annotations: prefix.annotations.clone(),
                annotation_details: prefix.annotation_details.clone(),
                suppressed_rules: prefix.suppressed_rules.clone(),
                is_static: true,
                is_final: true,
                is_private: false,
                is_public: true,
                is_protected: false,
                is_transient: false,
                is_volatile: false,
                init: None,
                span: Span::new(start, self.previous_span().end),
            }));
            if !self.eat(TokenKind::Comma) {
                self.eat(TokenKind::Semicolon);
                return constants;
            }
        }
    }

    fn parse_method(
        &mut self,
        start: usize,
        prefix: PrefixInfo,
        return_type: Option<String>,
    ) -> Result<MethodDecl, ParseError> {
        let name = self.expect_name_text()?;
        self.expect(TokenKind::LeftParen)?;
        let mut params = Vec::new();
        self.skip_newlines();

        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            let param_prefix = self.parse_prefix()?;
            if self.at_ident_like()
                && matches!(self.peek_kind(1), TokenKind::Comma | TokenKind::RightParen)
            {
                let token = self.advance();
                params.push(Param {
                    name: token.text(self.source).to_string(),
                    type_name: None,
                    annotations: param_prefix.annotations,
                    annotation_details: param_prefix.annotation_details,
                    type_annotations: Vec::new(),
                    span: token.span,
                });
            } else {
                let type_name = if self.at(TokenKind::Def) || self.at(TokenKind::Var) {
                    self.advance();
                    None
                } else {
                    Some(self.parse_type_name()?)
                };
                let type_annotations = self.last_type_annotations.clone();
                let token = self.expect_ident()?;
                params.push(Param {
                    name: token.text(self.source).to_string(),
                    type_name,
                    annotations: param_prefix.annotations,
                    annotation_details: param_prefix.annotation_details,
                    type_annotations,
                    span: token.span,
                });
            }

            if self.eat(TokenKind::Assign) {
                let _ = self.parse_expression()?;
            }
            self.skip_newlines();
            if !self.eat(TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }

        self.skip_newlines();
        self.expect(TokenKind::RightParen)?;
        self.skip_newlines();

        let mut throws = Vec::new();
        if self.eat(TokenKind::Throws) {
            loop {
                self.skip_newlines();
                if !self.at_ident_like() {
                    break;
                }
                throws.push(self.parse_type_name()?);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }

        self.skip_newlines();
        let body = if self.at(TokenKind::LeftBrace) {
            Some(self.parse_block()?)
        } else {
            self.eat(TokenKind::Semicolon);
            None
        };

        Ok(MethodDecl {
            name,
            return_type,
            is_static: prefix.is_static,
            is_private: prefix.is_private,
            is_public: prefix.is_public,
            is_protected: prefix.is_protected,
            is_final: prefix.is_final,
            is_abstract: prefix.is_abstract,
            is_synchronized: prefix.is_synchronized,
            throws,
            params,
            annotations: prefix.annotations,
            annotation_details: prefix.annotation_details,
            suppressed_rules: prefix.suppressed_rules,
            body,
            span: Span::new(start, self.previous_span().end),
        })
    }

    fn parse_field_or_var(
        &mut self,
        start: usize,
        name: String,
        in_class: bool,
        type_name: Option<&str>,
        prefix: &PrefixInfo,
    ) -> Result<Item, ParseError> {
        let mut items = Vec::new();
        let mut next_name = name;
        let mut next_start = start;
        loop {
            let init = if self.eat(TokenKind::Assign) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            let span = Span::new(next_start, self.previous_span().end);
            // Members of an interface are implicitly `public static final`.
            let in_interface = self
                .class_kind
                .is_some_and(|kind| matches!(kind, ClassKind::Interface | ClassKind::Annotation));
            let item = if in_class {
                Item::Field(FieldDecl {
                    name: next_name,
                    type_name: type_name.map(ToString::to_string),
                    annotations: prefix.annotations.clone(),
                    annotation_details: prefix.annotation_details.clone(),
                    suppressed_rules: prefix.suppressed_rules.clone(),
                    is_static: prefix.is_static || in_interface,
                    is_final: prefix.is_final || in_interface,
                    is_private: prefix.is_private,
                    is_public: prefix.is_public || in_interface,
                    is_protected: prefix.is_protected,
                    is_transient: prefix.is_transient,
                    is_volatile: prefix.is_volatile,
                    init,
                    span,
                })
            } else {
                Item::Statement(Stmt::VarDecl(VarDecl {
                    name: next_name,
                    type_name: type_name.map(ToString::to_string),
                    annotations: prefix.annotations.clone(),
                    annotation_details: prefix.annotation_details.clone(),
                    init,
                    span,
                }))
            };
            items.push(item);
            if !self.eat(TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
            next_start = self.current().span.start;
            next_name = self.expect_ident_text()?;
        }
        self.eat(TokenKind::Semicolon);
        self.skip_newlines();
        if items.len() == 1 {
            Ok(items.pop().expect("one item"))
        } else {
            Ok(Item::Group(items))
        }
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        self.skip_separators();
        let start = self.current().span.start;

        match self.current().kind {
            TokenKind::LeftBrace => return self.parse_block().map(Stmt::Block),
            TokenKind::If => return self.parse_if(start),
            TokenKind::While => return self.parse_while(start),
            TokenKind::Do => return self.parse_do_while(start),
            TokenKind::For => return self.parse_for(start),
            TokenKind::Try => return self.parse_try(start),
            TokenKind::Return => return self.parse_return(start),
            TokenKind::Throw => return self.parse_throw(start),
            TokenKind::Break => {
                let token = self.advance();
                return Ok(Stmt::Break(token.span));
            }
            TokenKind::Continue => {
                let token = self.advance();
                return Ok(Stmt::Continue(token.span));
            }
            TokenKind::Assert => return self.parse_assert(start),
            TokenKind::Switch => return self.parse_switch(start),
            TokenKind::Synchronized => {
                self.advance();
                let expression = self.parse_parenthesized_expression()?;
                self.skip_newlines();
                let body = Box::new(self.parse_statement()?);
                return Ok(Stmt::Synchronized(SynchronizedStmt {
                    expression,
                    body,
                    span: Span::new(start, self.previous_span().end),
                }));
            }
            _ => {}
        }

        let leading_prefix = if self.at(TokenKind::At)
            || matches!(
                self.current().kind,
                TokenKind::Final
                    | TokenKind::Static
                    | TokenKind::Abstract
                    | TokenKind::Public
                    | TokenKind::Private
                    | TokenKind::Protected
            ) {
            self.parse_prefix()?
        } else {
            PrefixInfo::default()
        };

        if self.looks_like_local_declaration() {
            let start = self.current().span.start;
            let mut type_name = None;
            if !self.eat(TokenKind::Def) && !self.eat(TokenKind::Var) {
                type_name = Some(self.parse_type_name()?);
            }
            let name = self.expect_ident_text()?;
            let init = if self.eat(TokenKind::Assign) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            let mut declarations = vec![Stmt::VarDecl(VarDecl {
                name,
                type_name,
                annotations: leading_prefix.annotations.clone(),
                annotation_details: leading_prefix.annotation_details.clone(),
                init,
                span: Span::new(start, self.previous_span().end),
            })];
            while self.eat(TokenKind::Comma) {
                declarations.push(self.parse_for_declaration()?);
            }
            self.eat(TokenKind::Semicolon);
            self.skip_newlines();
            return Ok(combine_declarations(declarations));
        }

        let expr = self.parse_expression()?;
        let expr = self.parse_command_arguments(expr)?;
        self.eat(TokenKind::Semicolon);
        self.skip_newlines();

        Ok(Stmt::Expr(expr))
    }

    fn parse_if(&mut self, start: usize) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::If)?;
        let condition = self.parse_parenthesized_expression()?;
        self.skip_newlines();
        let then_branch = Box::new(self.parse_statement()?);
        self.skip_newlines();

        let mut else_branch = None;
        let mut else_span = None;
        if self.eat(TokenKind::Else) {
            let else_start = self.previous_span().start;
            self.skip_newlines();
            let branch = Box::new(self.parse_statement()?);
            else_span = Some(Span::new(else_start, branch.span().end));
            else_branch = Some(branch);
        }

        Ok(Stmt::If(IfStmt {
            condition,
            then_branch,
            else_branch,
            else_span,
            span: Span::new(start, self.previous_span().end),
        }))
    }

    fn parse_while(&mut self, start: usize) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::While)?;
        let condition = self.parse_parenthesized_expression()?;
        self.skip_newlines();
        let body = Box::new(self.parse_statement()?);
        Ok(Stmt::While(WhileStmt {
            condition,
            body,
            span: Span::new(start, self.previous_span().end),
        }))
    }

    fn parse_do_while(&mut self, start: usize) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::Do)?;
        self.skip_newlines();
        let body = Box::new(self.parse_statement()?);
        self.skip_newlines();
        self.expect(TokenKind::While)?;
        let condition = self.parse_parenthesized_expression()?;
        self.eat(TokenKind::Semicolon);
        self.skip_newlines();
        Ok(Stmt::DoWhile(DoWhileStmt {
            body,
            condition,
            span: Span::new(start, self.previous_span().end),
        }))
    }

    fn parse_for(&mut self, start: usize) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::For)?;
        self.expect(TokenKind::LeftParen)?;
        self.nesting += 1;
        self.skip_newlines();

        let after_paren = self.pos;
        if let Some(stmt) = self.try_parse_foreach(start)? {
            return Ok(stmt);
        }
        self.pos = after_paren;

        let init = if self.at(TokenKind::Semicolon) {
            None
        } else if self.looks_like_local_declaration() {
            let mut declarations = vec![self.parse_for_declaration()?];
            while self.eat(TokenKind::Comma) {
                declarations.push(self.parse_for_declaration()?);
            }
            Some(Box::new(combine_declarations(declarations)))
        } else {
            Some(Box::new(Stmt::Expr(self.parse_expression()?)))
        };
        self.expect(TokenKind::Semicolon)?;

        let condition = if self.at(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(TokenKind::Semicolon)?;
        self.skip_newlines();

        let update = if self.at(TokenKind::RightParen) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(TokenKind::RightParen)?;
        self.nesting -= 1;
        self.skip_newlines();
        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::For(ForStmt {
            init,
            condition,
            update,
            body,
            span: Span::new(start, self.previous_span().end),
        }))
    }

    /// Parses one declarator of a classic `for` header: `int i = 0`, or the
    /// untyped `size = rows.size()` that follows a comma.
    fn parse_for_declaration(&mut self) -> Result<Stmt, ParseError> {
        let start = self.current().span.start;
        let prefix = self.parse_prefix()?;
        let mut type_name = None;
        if self.eat(TokenKind::Def) || self.eat(TokenKind::Var) {
        } else if self.looks_like_local_declaration() {
            type_name = Some(self.parse_type_name()?);
        }

        let name = self.expect_ident_text()?;
        let init = if self.eat(TokenKind::Assign) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Stmt::VarDecl(VarDecl {
            name,
            type_name,
            annotations: prefix.annotations,
            annotation_details: prefix.annotation_details,
            init,
            span: Span::new(start, self.previous_span().end),
        }))
    }

    /// Parses a `for (x in xs)` header and its body. Returns `None` without
    /// consuming input when the header is not a for-each header, so that the
    /// caller can fall back to a classic `for`.
    fn try_parse_foreach(&mut self, start: usize) -> Result<Option<Stmt>, ParseError> {
        let saved = self.pos;
        let header = self.parse_foreach_header();

        let Ok(Some((var_name, var_type, iterable))) = header else {
            self.pos = saved;
            return Ok(None);
        };

        self.skip_newlines();
        let body = Box::new(self.parse_statement()?);
        Ok(Some(Stmt::ForEach(ForEachStmt {
            var_name,
            var_type,
            iterable,
            body,
            span: Span::new(start, self.previous_span().end),
        })))
    }

    /// Parses `Type name : expr)` or `name in expr)`, returning `None` when the
    /// header is not a for-each header.
    fn parse_foreach_header(
        &mut self,
    ) -> Result<Option<(String, Option<String>, Expr)>, ParseError> {
        let _ = self.parse_prefix()?;
        let var_type = if self.at(TokenKind::Def) || self.at(TokenKind::Var) {
            self.advance();
            None
        } else if self.looks_like_local_declaration() {
            Some(self.parse_type_name()?)
        } else {
            None
        };

        if !self.at_ident_like() {
            return Ok(None);
        }
        let var_name = self.expect_ident_text()?;
        self.skip_newlines();
        if !self.eat(TokenKind::In) && !self.eat(TokenKind::Colon) {
            return Ok(None);
        }

        let iterable = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        self.nesting -= 1;
        Ok(Some((var_name, var_type, iterable)))
    }

    fn parse_try(&mut self, start: usize) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::Try)?;
        let mut resources = Vec::new();
        if self.at(TokenKind::LeftParen) {
            self.advance();
            self.nesting += 1;
            self.skip_newlines();
            while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
                resources.push(self.parse_for_declaration()?);
                self.skip_newlines();
                if !self.eat(TokenKind::Comma)
                    && !self.eat(TokenKind::Semicolon)
                    && !self.at_ident_like()
                {
                    break;
                }
                self.skip_newlines();
            }
            self.expect(TokenKind::RightParen)?;
            self.nesting -= 1;
        }

        let mut body = self.parse_block()?;
        if !resources.is_empty() {
            let mut statements = resources;
            statements.extend(body.statements);
            body.statements = statements;
        }
        let mut catches = Vec::new();
        self.skip_newlines();

        while self.eat(TokenKind::Catch) {
            let catch_start = self.previous_span().start;
            self.expect(TokenKind::LeftParen)?;
            let mut names: Vec<String> = Vec::new();
            while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
                if self.at_ident_like() {
                    names.push(self.current().text(self.source).to_string());
                }
                self.advance();
            }
            self.expect(TokenKind::RightParen)?;
            let param_name = names.pop();
            let type_names = names.split_off(0);
            self.skip_newlines();
            let block = self.parse_block()?;
            catches.push(CatchClause {
                param_name,
                type_names,
                body: block,
                span: Span::new(catch_start, self.previous_span().end),
            });
            self.skip_newlines();
        }

        let mut finally = None;
        let mut finally_span = None;
        if self.eat(TokenKind::Finally) {
            let finally_start = self.previous_span().start;
            self.skip_newlines();
            let block = self.parse_block()?;
            finally_span = Some(Span::new(finally_start, block.span.end));
            finally = Some(block);
        }

        Ok(Stmt::Try(TryStmt {
            body,
            catches,
            finally,
            finally_span,
            span: Span::new(start, self.previous_span().end),
        }))
    }

    fn parse_switch(&mut self, start: usize) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::Switch)?;
        let expression = self.parse_parenthesized_expression()?;
        self.skip_newlines();
        self.expect(TokenKind::LeftBrace)?;
        self.skip_separators();

        let mut cases = Vec::new();
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let case_start = self.current().span.start;
            let label = if self.eat(TokenKind::Case) {
                let label = self.parse_expression()?;
                self.expect(TokenKind::Colon)?;
                Some(label)
            } else if self.eat(TokenKind::Default) {
                self.expect(TokenKind::Colon)?;
                None
            } else {
                return Err(self.error_here("expected `case` or `default`"));
            };

            self.skip_separators();
            let mut body = Vec::new();
            while !matches!(
                self.current().kind,
                TokenKind::Case | TokenKind::Default | TokenKind::RightBrace | TokenKind::Eof
            ) {
                body.push(self.parse_statement()?);
                self.skip_separators();
            }

            cases.push(SwitchCase {
                label,
                body,
                span: Span::new(case_start, self.previous_span().end),
            });
        }

        self.expect(TokenKind::RightBrace)?;
        Ok(Stmt::Switch(SwitchStmt {
            expression,
            cases,
            span: Span::new(start, self.previous_span().end),
        }))
    }

    fn parse_return(&mut self, start: usize) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::Return)?;
        let value = if self.current().kind.is_statement_end() {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.eat(TokenKind::Semicolon);
        self.skip_newlines();
        Ok(Stmt::Return(ReturnStmt {
            value,
            span: Span::new(start, self.previous_span().end),
        }))
    }

    fn parse_throw(&mut self, start: usize) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::Throw)?;
        let value = self.parse_expression()?;
        self.eat(TokenKind::Semicolon);
        self.skip_newlines();
        Ok(Stmt::Throw(ThrowStmt {
            value,
            span: Span::new(start, self.previous_span().end),
        }))
    }

    fn parse_assert(&mut self, start: usize) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::Assert)?;
        let condition = self.parse_expression()?;
        let message = if self.eat(TokenKind::Colon) || self.eat(TokenKind::Comma) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.eat(TokenKind::Semicolon);
        self.skip_newlines();
        Ok(Stmt::Assert(AssertStmt {
            condition,
            message,
            span: Span::new(start, self.previous_span().end),
        }))
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let start = self.expect(TokenKind::LeftBrace)?.span.start;
        let mut statements = Vec::new();
        self.skip_separators();

        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            statements.push(self.parse_statement()?);
            self.skip_separators();
        }

        let end = self.expect(TokenKind::RightBrace)?.span.end;
        Ok(Block {
            statements,
            span: Span::new(start, end),
        })
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.skip_newlines();
        self.parse_assignment()
    }

    /// Parses `( expression )`, marking the inside as newline-insensitive.
    fn parse_parenthesized_expression(&mut self) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LeftParen)?;
        self.nesting += 1;
        let expr = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        self.nesting -= 1;
        Ok(expr)
    }

    fn parse_assignment(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_ternary()?;
        let op = match self.current().kind {
            TokenKind::Assign => Some(AssignOp::Assign),
            TokenKind::PlusAssign => Some(AssignOp::Add),
            TokenKind::MinusAssign => Some(AssignOp::Sub),
            TokenKind::StarAssign => Some(AssignOp::Mul),
            TokenKind::SlashAssign => Some(AssignOp::Div),
            TokenKind::PercentAssign => Some(AssignOp::Mod),
            TokenKind::AmpAssign => Some(AssignOp::BitAnd),
            TokenKind::PipeAssign => Some(AssignOp::BitOr),
            TokenKind::CaretAssign => Some(AssignOp::BitXor),
            TokenKind::ShiftLeftAssign => Some(AssignOp::ShiftLeft),
            TokenKind::ShiftRightAssign => Some(AssignOp::ShiftRight),
            TokenKind::PowerAssign => Some(AssignOp::Power),
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            self.skip_newlines();
            let value = self.parse_assignment()?;
            let span = left.span.cover(value.span);
            return Ok(Expr {
                kind: ExprKind::Assign {
                    target: Box::new(left),
                    op,
                    value: Box::new(value),
                },
                span,
            });
        }

        Ok(left)
    }

    fn parse_ternary(&mut self) -> Result<Expr, ParseError> {
        let condition = self.parse_or()?;
        self.skip_continuation_newline(TokenKind::Question);
        if self.eat(TokenKind::Question) {
            self.skip_newlines();
            let then_expr = self.parse_expression()?;
            self.skip_continuation_newline(TokenKind::Colon);
            self.expect(TokenKind::Colon)?;
            self.skip_newlines();
            let else_expr = self.parse_ternary()?;
            let span = condition.span.cover(else_expr.span);
            return Ok(Expr {
                kind: ExprKind::Ternary {
                    is_elvis: false,
                    condition: Box::new(condition),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                },
                span,
            });
        }
        self.skip_continuation_newline(TokenKind::Elvis);
        if self.eat(TokenKind::Elvis) {
            self.skip_newlines();
            let else_expr = self.parse_ternary()?;
            let span = condition.span.cover(else_expr.span);
            return Ok(Expr {
                kind: ExprKind::Ternary {
                    is_elvis: true,
                    condition: Box::new(condition.clone()),
                    then_expr: Box::new(condition),
                    else_expr: Box::new(else_expr),
                },
                span,
            });
        }
        Ok(condition)
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(Self::parse_and, &[(TokenKind::OrOr, BinaryOp::Or)])
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(Self::parse_bit_or, &[(TokenKind::AndAnd, BinaryOp::And)])
    }

    fn parse_bit_or(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(Self::parse_bit_xor, &[(TokenKind::Pipe, BinaryOp::BitOr)])
    }

    fn parse_bit_xor(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(Self::parse_bit_and, &[(TokenKind::Caret, BinaryOp::BitXor)])
    }

    fn parse_bit_and(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(Self::parse_equality, &[(TokenKind::Amp, BinaryOp::BitAnd)])
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(
            Self::parse_comparison,
            &[
                (TokenKind::Eq, BinaryOp::Eq),
                (TokenKind::NotEq, BinaryOp::NotEq),
                (TokenKind::RegexFind, BinaryOp::RegexFind),
                (TokenKind::RegexMatch, BinaryOp::RegexMatch),
            ],
        )
    }

    /// Parses `as` casts and the negated `!in` / `!instanceof` forms, which bind
    /// looser than the comparisons below them.
    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_instanceof()?;
        loop {
            self.skip_continuation_newline(TokenKind::As);
            if self.at_negated_comparison() {
                expr = self.parse_negated_comparison(expr)?;
                continue;
            }
            if !self.eat(TokenKind::As) {
                break;
            }
            let type_name = self.parse_type_name()?;
            let span = Span::new(expr.span.start, self.previous_span().end);
            expr = Expr {
                kind: ExprKind::Cast {
                    expr: Box::new(expr),
                    type_name,
                },
                span,
            };
        }
        Ok(expr)
    }

    fn at_negated_comparison(&self) -> bool {
        self.at(TokenKind::Not)
            && matches!(self.peek_kind(1), TokenKind::In | TokenKind::Instanceof)
    }

    /// Parses `value !in values` and `value !instanceof Type`.
    fn parse_negated_comparison(&mut self, expr: Expr) -> Result<Expr, ParseError> {
        let not_start = self.advance().span.start;
        let op = if self.eat(TokenKind::In) {
            BinaryOp::In
        } else {
            self.expect(TokenKind::Instanceof)?;
            BinaryOp::InstanceOf
        };
        let right = self.parse_comparison_operand()?;
        let inner_span = expr.span.cover(right.span);
        let comparison = Expr {
            kind: ExprKind::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            },
            span: inner_span,
        };
        let span = Span::new(not_start, inner_span.end);
        Ok(Expr {
            kind: ExprKind::Unary {
                op: UnaryOp::Not,
                operand: Box::new(comparison),
            },
            span,
        })
    }

    /// Parses relational, range and `in` comparisons.
    fn parse_instanceof(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_binary(
            Self::parse_shift,
            &[
                (TokenKind::Lt, BinaryOp::Lt),
                (TokenKind::Le, BinaryOp::Le),
                (TokenKind::Gt, BinaryOp::Gt),
                (TokenKind::Ge, BinaryOp::Ge),
                (TokenKind::In, BinaryOp::In),
                (TokenKind::Spaceship, BinaryOp::Spaceship),
                (TokenKind::Range, BinaryOp::Range),
                (TokenKind::RangeExclusive, BinaryOp::RangeExclusive),
            ],
        )?;
        while self.eat(TokenKind::Instanceof) {
            let right = self.parse_comparison_operand()?;
            let span = expr.span.cover(right.span);
            expr = Expr {
                kind: ExprKind::Binary {
                    op: BinaryOp::InstanceOf,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(expr)
    }

    /// Parses the right-hand side of `instanceof`, which may name a
    /// parameterized type such as `List<String>`.
    fn parse_comparison_operand(&mut self) -> Result<Expr, ParseError> {
        if self.at_ident_like() && self.peek_kind(1) == TokenKind::Lt {
            let type_name = self.parse_type_name()?;
            let span = self.previous_span();
            return Ok(Expr {
                kind: ExprKind::Variable(type_name),
                span,
            });
        }
        self.parse_shift()
    }

    fn parse_shift(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_additive()?;
        loop {
            let op = match self.current().kind {
                TokenKind::ShiftLeft => BinaryOp::ShiftLeft,
                TokenKind::ShiftRight => BinaryOp::ShiftRight,
                _ => break,
            };
            self.advance();
            // `>>` is lexed twice inside `>>>`, which is still one shift.
            if op == BinaryOp::ShiftRight && self.at(TokenKind::Gt) {
                self.advance();
            }
            self.skip_newlines();
            let right = self.parse_additive()?;
            let span = expr.span.cover(right.span);
            expr = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(
            Self::parse_multiplicative,
            &[
                (TokenKind::Plus, BinaryOp::Add),
                (TokenKind::Minus, BinaryOp::Sub),
            ],
        )
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(
            Self::parse_power,
            &[
                (TokenKind::Star, BinaryOp::Mul),
                (TokenKind::Slash, BinaryOp::Div),
                (TokenKind::Percent, BinaryOp::Mod),
            ],
        )
    }

    fn parse_power(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_unary()?;
        if self.eat(TokenKind::Power) {
            self.skip_newlines();
            let right = self.parse_power()?;
            let span = left.span.cover(right.span);
            return Ok(Expr {
                kind: ExprKind::Binary {
                    op: BinaryOp::Power,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            });
        }
        Ok(left)
    }

    fn parse_binary(
        &mut self,
        next: fn(&mut Self) -> Result<Expr, ParseError>,
        operators: &[(TokenKind, BinaryOp)],
    ) -> Result<Expr, ParseError> {
        let mut left = next(self)?;
        loop {
            self.skip_newlines_before_operator(operators);
            let Some((_, op)) = operators.iter().find(|(kind, _)| self.at(*kind)).copied() else {
                break;
            };
            self.advance();
            self.skip_newlines();
            let right = next(self)?;
            let span = left.span.cover(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        let op = match self.current().kind {
            TokenKind::Not => Some(UnaryOp::Not),
            TokenKind::Minus => Some(UnaryOp::Neg),
            TokenKind::Plus => Some(UnaryOp::Pos),
            TokenKind::Tilde => Some(UnaryOp::BitNot),
            TokenKind::Inc => Some(UnaryOp::PreInc),
            TokenKind::Dec => Some(UnaryOp::PreDec),
            _ => None,
        };

        if let Some(op) = op {
            let start = self.advance().span.start;
            let operand = self.parse_unary()?;
            let span = Span::new(start, operand.span.end);
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op,
                    operand: Box::new(operand),
                },
                span,
            });
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.at(TokenKind::Newline)
                && (self.next_code_starts_chain() || self.next_code_starts_closure_argument(&expr))
            {
                self.skip_newlines();
            }
            match self.current().kind {
                TokenKind::Star if self.peek_kind(1) == TokenKind::Dot => {
                    self.advance();
                    self.advance();
                    expr = self.parse_member(expr, false)?;
                }
                TokenKind::LeftParen
                    if matches!(
                        expr.kind,
                        ExprKind::This | ExprKind::Super | ExprKind::Closure { .. }
                    ) =>
                {
                    let args = self.parse_arguments()?;
                    let end = self.previous_span().end;
                    let start = expr.span.start;
                    expr = Expr {
                        kind: ExprKind::Call {
                            receiver: Some(Box::new(expr)),
                            name: "call".to_string(),
                            args,
                            safe: false,
                        },
                        span: Span::new(start, end),
                    };
                }
                TokenKind::Dot | TokenKind::SafeDot => {
                    let safe = self.current().kind == TokenKind::SafeDot;
                    self.advance();
                    expr = self.parse_member(expr, safe)?;
                }
                TokenKind::LeftBracket | TokenKind::SafeIndex => {
                    let safe = self.current().kind == TokenKind::SafeIndex;
                    self.advance();
                    self.nesting += 1;
                    let index = self.parse_expression()?;
                    self.expect(TokenKind::RightBracket)?;
                    self.nesting -= 1;
                    let span = Span::new(expr.span.start, self.previous_span().end);
                    expr = Expr {
                        kind: ExprKind::Index {
                            target: Box::new(expr),
                            index: Box::new(index),
                            safe,
                        },
                        span,
                    };
                }
                TokenKind::Inc | TokenKind::Dec => {
                    let op = if self.current().kind == TokenKind::Inc {
                        UnaryOp::PostInc
                    } else {
                        UnaryOp::PostDec
                    };
                    let end = self.advance().span.end;
                    let span = Span::new(expr.span.start, end);
                    expr = Expr {
                        kind: ExprKind::Unary {
                            op,
                            operand: Box::new(expr),
                        },
                        span,
                    };
                }
                TokenKind::LeftBrace
                    if matches!(
                        expr.kind,
                        ExprKind::Variable(_) | ExprKind::Property { .. } | ExprKind::Call { .. }
                    ) =>
                {
                    let closure = self.parse_closure()?;
                    let (receiver, name, args, safe) = match expr.kind {
                        ExprKind::Call {
                            receiver,
                            name,
                            args,
                            safe,
                        } => (receiver, name, args, safe),
                        ExprKind::Variable(name) => (None, name, Vec::new(), false),
                        ExprKind::Property {
                            receiver,
                            name,
                            safe,
                        } => (Some(receiver), name, Vec::new(), safe),
                        _ => break,
                    };
                    let mut args = args;
                    args.push(closure);
                    self.parse_trailing_arguments(&mut args)?;
                    let span = Span::new(expr.span.start, self.previous_span().end);
                    expr = Expr {
                        kind: ExprKind::Call {
                            receiver,
                            name,
                            args,
                            safe,
                        },
                        span,
                    };
                }
                TokenKind::MethodRef => {
                    self.advance();
                    let name = format!("::{}", self.expect_name_text()?);
                    let span = Span::new(expr.span.start, self.previous_span().end);
                    expr = Expr {
                        kind: ExprKind::Property {
                            receiver: Box::new(expr),
                            name,
                            safe: false,
                        },
                        span,
                    };
                }
                TokenKind::Lt if self.at_generic_class_literal() => {
                    self.skip_balanced(TokenKind::Lt, TokenKind::Gt)?;
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parses the member name, and the argument list if present, after `.`,
    /// `?.` or `*.`. Groovy allows the name to start on the next line.
    fn parse_member(&mut self, expr: Expr, safe: bool) -> Result<Expr, ParseError> {
        self.skip_newlines();
        if self.at(TokenKind::Lt) {
            self.skip_balanced(TokenKind::Lt, TokenKind::Gt)?;
        }
        let name = if self.eat(TokenKind::Amp) {
            format!("&{}", self.expect_ident_text()?)
        } else if self.eat(TokenKind::At) {
            format!("@{}", self.expect_name_text()?)
        } else {
            self.expect_name_text()?
        };

        if !self.at(TokenKind::LeftParen) {
            let span = Span::new(expr.span.start, self.previous_span().end);
            let receiver = Box::new(expr);
            return Ok(Expr {
                kind: ExprKind::Property {
                    receiver,
                    name,
                    safe,
                },
                span,
            });
        }

        let args = self.parse_arguments()?;
        let span = Span::new(expr.span.start, self.previous_span().end);
        let receiver = Some(Box::new(expr));
        Ok(Expr {
            kind: ExprKind::Call {
                receiver,
                name,
                args,
                safe,
            },
            span,
        })
    }

    /// Continues a command-style argument list after a trailing closure, as in
    /// `verify { ... }, never()`. Inside parentheses or brackets a comma
    /// belongs to the enclosing list, so it is left alone.
    fn parse_trailing_arguments(&mut self, args: &mut Vec<Expr>) -> Result<(), ParseError> {
        if self.nesting > 0 {
            return Ok(());
        }
        while self.eat(TokenKind::Comma) {
            self.skip_newlines();
            args.push(self.parse_argument_expression()?);
        }
        Ok(())
    }

    /// Parses one argument or list item, allowing the spread operator `*`.
    fn parse_argument_expression(&mut self) -> Result<Expr, ParseError> {
        if !self.at(TokenKind::Star) {
            return self.parse_expression();
        }
        let start = self.advance().span.start;
        let operand = self.parse_expression()?;
        let span = Span::new(start, operand.span.end);
        Ok(Expr {
            kind: ExprKind::Unary {
                op: UnaryOp::Spread,
                operand: Box::new(operand),
            },
            span,
        })
    }

    /// Returns whether the current `<` directly follows the previous token, as
    /// in `List<String>`, rather than starting a comparison.
    fn type_arguments_are_adjacent(&self) -> bool {
        self.current().span.start == self.previous_span().end
    }

    /// Detects a type used as an expression (`Type<Arguments>.class`,
    /// `List<String> as Class`), where the type arguments are not comparisons.
    fn at_generic_class_literal(&self) -> bool {
        let mut depth = 0usize;
        let mut offset = 0;
        while offset <= 32 {
            match self.peek_kind(offset) {
                TokenKind::Lt => depth += 1,
                TokenKind::Gt => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let next = self.peek_kind(offset + 1);
                        if next == TokenKind::As {
                            return true;
                        }
                        if next != TokenKind::Dot {
                            return false;
                        }
                        let member = self.peek_kind(offset + 2);
                        let is_class_literal = member == TokenKind::Class;
                        let is_member_access = self.at_ident_like_after(offset + 2)
                            && self.type_arguments_are_adjacent();
                        return is_class_literal || is_member_access;
                    }
                }
                TokenKind::Eof | TokenKind::Newline | TokenKind::Semicolon => return false,
                _ => {}
            }
            offset += 1;
        }
        false
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.current();
        let start = token.span.start;

        match token.kind {
            TokenKind::Number => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Literal(Literal::Number(token.text(self.source).to_string())),
                    span: token.span,
                })
            }
            TokenKind::String => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Literal(Literal::String(token.text(self.source).to_string())),
                    span: token.span,
                })
            }
            TokenKind::Regex => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Literal(Literal::Regex(token.text(self.source).to_string())),
                    span: token.span,
                })
            }
            TokenKind::True | TokenKind::False => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Literal(Literal::Boolean(token.kind == TokenKind::True)),
                    span: token.span,
                })
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Literal(Literal::Null),
                    span: token.span,
                })
            }
            TokenKind::Ident | TokenKind::Record | TokenKind::Permits => {
                self.advance();
                let mut name = token.text(self.source).to_string();
                if self.at(TokenKind::LeftBracket) && self.peek_kind(1) == TokenKind::RightBracket {
                    self.advance();
                    self.advance();
                    name.push_str("[]");
                }
                if self.at(TokenKind::Arrow) {
                    self.advance();
                    let param = Param {
                        name,
                        type_name: None,
                        annotations: Vec::new(),
                        annotation_details: Vec::new(),
                        type_annotations: Vec::new(),
                        span: token.span,
                    };
                    return self.parse_lambda_body(start, vec![param]);
                }
                if self.at(TokenKind::LeftParen) {
                    let args = self.parse_arguments()?;
                    return Ok(Expr {
                        kind: ExprKind::Call {
                            receiver: None,
                            name,
                            args,
                            safe: false,
                        },
                        span: Span::new(start, self.previous_span().end),
                    });
                }
                Ok(Expr {
                    kind: ExprKind::Variable(name),
                    span: token.span,
                })
            }
            TokenKind::Void => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Variable("void".to_string()),
                    span: token.span,
                })
            }
            TokenKind::This => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::This,
                    span: token.span,
                })
            }
            TokenKind::Super => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Super,
                    span: token.span,
                })
            }
            TokenKind::New => self.parse_new(start),
            TokenKind::LeftParen => {
                let saved = self.pos;
                if let Some(lambda) = self.try_parse_parameterized_lambda()? {
                    return Ok(lambda);
                }
                self.pos = saved;

                if let Some(cast) = self.try_parse_cast(start)? {
                    return Ok(cast);
                }

                self.pos = saved;
                self.expect(TokenKind::LeftParen)?;
                self.nesting += 1;
                let inner = self.parse_expression()?;
                self.expect(TokenKind::RightParen)?;
                self.nesting -= 1;
                Ok(Expr {
                    kind: ExprKind::Paren(Box::new(inner)),
                    span: Span::new(start, self.previous_span().end),
                })
            }
            TokenKind::Switch => self.parse_switch_expression(start),
            TokenKind::LeftBracket => self.parse_list_or_map(start),
            TokenKind::LeftBrace => self.parse_closure(),
            _ => Err(self.error_here(format!(
                "expected expression, found {}",
                token.kind.describe()
            ))),
        }
    }

    /// Parses `(a, B b) -> expr`, returning `None` when the input is a plain
    /// parenthesized expression.
    fn try_parse_parameterized_lambda(&mut self) -> Result<Option<Expr>, ParseError> {
        let saved = self.pos;
        let saved_nesting = self.nesting;
        let start = self.current().span.start;
        self.expect(TokenKind::LeftParen)?;
        self.nesting += 1;
        self.skip_newlines();

        let mut params = Vec::new();
        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            let prefix = self.parse_prefix()?;
            let type_name = if self.looks_like_local_declaration() {
                Some(self.parse_type_name()?)
            } else {
                None
            };
            if !self.at_ident_like() {
                self.pos = saved;
                self.nesting = saved_nesting;
                return Ok(None);
            }
            let token = self.advance();
            params.push(Param {
                name: token.text(self.source).to_string(),
                type_name,
                annotations: prefix.annotations,
                annotation_details: prefix.annotation_details,
                type_annotations: self.last_type_annotations.clone(),
                span: token.span,
            });
            self.skip_newlines();
            if !self.eat(TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }

        if !self.eat(TokenKind::RightParen) {
            self.pos = saved;
            self.nesting = saved_nesting;
            return Ok(None);
        }
        self.nesting -= 1;
        self.skip_newlines();
        if !self.eat(TokenKind::Arrow) {
            self.pos = saved;
            self.nesting = saved_nesting;
            return Ok(None);
        }
        self.parse_lambda_body(start, params).map(Some)
    }

    /// Parses a Java-style lambda body: a single expression or a block.
    fn parse_lambda_body(&mut self, start: usize, params: Vec<Param>) -> Result<Expr, ParseError> {
        self.skip_newlines();
        let statement = if self.at(TokenKind::LeftBrace) {
            Stmt::Block(self.parse_block()?)
        } else {
            Stmt::Expr(self.parse_expression()?)
        };
        let end = statement.span().end;
        Ok(Expr {
            kind: ExprKind::Closure {
                params,
                has_arrow: true,
                body: Block {
                    statements: vec![statement],
                    span: Span::new(start, end),
                },
            },
            span: Span::new(start, end),
        })
    }

    /// Parses a cast such as `(int) value`, whose operand may start on the next
    /// line. Returns `None` when the parentheses hold an expression instead.
    fn try_parse_cast(&mut self, start: usize) -> Result<Option<Expr>, ParseError> {
        let saved = self.pos;
        self.expect(TokenKind::LeftParen)?;
        let Ok(type_name) = self.parse_type_name() else {
            self.pos = saved;
            return Ok(None);
        };
        if !is_type_like(&type_name) || !self.eat(TokenKind::RightParen) {
            self.pos = saved;
            return Ok(None);
        }

        self.skip_newlines();
        if !self.starts_expression() {
            self.pos = saved;
            return Ok(None);
        }

        let operand = self.parse_unary()?;
        let span = Span::new(start, operand.span.end);
        Ok(Some(Expr {
            kind: ExprKind::Cast {
                expr: Box::new(operand),
                type_name,
            },
            span,
        }))
    }

    /// Parses a `switch` used as an expression: `switch (x) { case a -> b }`.
    fn parse_switch_expression(&mut self, start: usize) -> Result<Expr, ParseError> {
        self.expect(TokenKind::Switch)?;
        let scrutinee = self.parse_parenthesized_expression()?;
        self.skip_newlines();
        self.expect(TokenKind::LeftBrace)?;
        self.skip_separators();

        let mut cases = Vec::new();
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let case_start = self.current().span.start;
            let label = if self.eat(TokenKind::Case) {
                Some(self.parse_expression()?)
            } else {
                self.expect(TokenKind::Default)?;
                None
            };
            if !self.eat(TokenKind::Arrow) {
                self.expect(TokenKind::Colon)?;
            }
            self.skip_newlines();
            let value = self.parse_expression()?;
            cases.push(SwitchCase {
                label,
                span: Span::new(case_start, value.span.end),
                body: vec![Stmt::Expr(value)],
            });
            self.skip_separators();
        }

        let end = self.expect(TokenKind::RightBrace)?.span.end;
        Ok(Expr {
            kind: ExprKind::Switch {
                scrutinee: Box::new(scrutinee),
                cases,
            },
            span: Span::new(start, end),
        })
    }

    fn parse_new(&mut self, start: usize) -> Result<Expr, ParseError> {
        self.expect(TokenKind::New)?;
        let mut type_name = self.parse_type_name()?;
        if self.at(TokenKind::LeftBracket) {
            let mut dimensions = 0;
            while self.eat(TokenKind::LeftBracket) {
                dimensions += 1;
                if !self.at(TokenKind::RightBracket) {
                    let _ = self.parse_expression()?;
                }
                self.expect(TokenKind::RightBracket)?;
            }
            for _ in 0..dimensions {
                type_name.push_str("[]");
            }
            if self.at(TokenKind::LeftBrace) {
                self.skip_balanced(TokenKind::LeftBrace, TokenKind::RightBrace)?;
            }
            return Ok(Expr {
                kind: ExprKind::New {
                    type_name,
                    args: Vec::new(),
                    body: None,
                },
                span: Span::new(start, self.previous_span().end),
            });
        }
        if self.at(TokenKind::LeftBrace) {
            self.skip_balanced(TokenKind::LeftBrace, TokenKind::RightBrace)?;
            return Ok(Expr {
                kind: ExprKind::New {
                    type_name,
                    args: Vec::new(),
                    body: None,
                },
                span: Span::new(start, self.previous_span().end),
            });
        }
        let args = self.parse_arguments()?;
        let body = if self.at(TokenKind::LeftBrace) {
            Some(self.parse_anonymous_body()?)
        } else {
            None
        };
        Ok(Expr {
            kind: ExprKind::New {
                type_name,
                args,
                body,
            },
            span: Span::new(start, self.previous_span().end),
        })
    }

    fn parse_arguments(&mut self) -> Result<Vec<Expr>, ParseError> {
        self.expect(TokenKind::LeftParen)?;
        self.nesting += 1;
        let mut args = Vec::new();
        self.skip_newlines();

        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            if self.at_named_argument() {
                let name_start = self.current().span.start;
                let name = self.expect_name_text()?;
                self.advance();
                let value = self.parse_expression()?;
                args.push(Expr {
                    span: Span::new(name_start, value.span.end),
                    kind: ExprKind::NamedArg {
                        name,
                        value: Box::new(value),
                    },
                });
            } else {
                args.push(self.parse_argument_expression()?);
            }
            self.skip_newlines();
            if !self.eat(TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }

        self.skip_newlines();
        self.expect(TokenKind::RightParen)?;
        self.nesting -= 1;
        Ok(args)
    }

    fn parse_list_or_map(&mut self, start: usize) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LeftBracket)?;
        self.nesting += 1;
        let result = self.parse_list_or_map_body(start);
        self.nesting -= 1;
        result
    }

    fn parse_list_or_map_body(&mut self, start: usize) -> Result<Expr, ParseError> {
        self.skip_newlines();

        if self.eat(TokenKind::Colon) {
            self.expect(TokenKind::RightBracket)?;
            return Ok(Expr {
                kind: ExprKind::Map(Vec::new()),
                span: Span::new(start, self.previous_span().end),
            });
        }

        if self.at(TokenKind::RightBracket) {
            self.advance();
            return Ok(Expr {
                kind: ExprKind::List(Vec::new()),
                span: Span::new(start, self.previous_span().end),
            });
        }

        if self.at(TokenKind::Ident) && self.peek_kind(1) == TokenKind::Colon {
            return self.parse_map_body(start);
        }
        if is_property_name_keyword(self.current().kind) && self.peek_kind(1) == TokenKind::Colon {
            return self.parse_map_body(start);
        }

        let first = self.parse_argument_expression()?;
        if self.eat(TokenKind::Colon) {
            let mut entries = vec![(first, self.parse_argument_expression()?)];
            while self.eat(TokenKind::Comma) {
                self.skip_newlines();
                if self.at(TokenKind::RightBracket) {
                    break;
                }
                let key = self.parse_expression()?;
                self.expect(TokenKind::Colon)?;
                let value = self.parse_argument_expression()?;
                entries.push((key, value));
            }
            self.skip_newlines();
            self.expect(TokenKind::RightBracket)?;
            return Ok(Expr {
                kind: ExprKind::Map(entries),
                span: Span::new(start, self.previous_span().end),
            });
        }

        let mut items = vec![first];
        while self.eat(TokenKind::Comma) {
            self.skip_newlines();
            if self.at(TokenKind::RightBracket) {
                break;
            }
            items.push(self.parse_argument_expression()?);
        }
        self.skip_newlines();
        self.expect(TokenKind::RightBracket)?;
        Ok(Expr {
            kind: ExprKind::List(items),
            span: Span::new(start, self.previous_span().end),
        })
    }

    /// Parses map entries whose keys are names or keywords, e.g. `[class: 1]`.
    fn parse_map_body(&mut self, start: usize) -> Result<Expr, ParseError> {
        let mut entries = Vec::new();
        loop {
            let key = self.parse_map_key()?;
            self.expect(TokenKind::Colon)?;
            let value = self.parse_argument_expression()?;
            entries.push((key, value));
            self.skip_newlines();
            if !self.eat(TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
            if self.at(TokenKind::RightBracket) {
                break;
            }
        }

        self.skip_newlines();
        self.expect(TokenKind::RightBracket)?;
        Ok(Expr {
            kind: ExprKind::Map(entries),
            span: Span::new(start, self.previous_span().end),
        })
    }

    fn parse_map_key(&mut self) -> Result<Expr, ParseError> {
        if is_property_name_keyword(self.current().kind) && self.peek_kind(1) == TokenKind::Colon {
            let token = self.advance();
            let span = token.span;
            let name = token.text(self.source).to_string();
            return Ok(Expr {
                kind: ExprKind::Variable(name),
                span,
            });
        }
        self.parse_expression()
    }

    fn parse_closure(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect(TokenKind::LeftBrace)?.span.start;
        self.skip_newlines();

        let saved = self.pos;
        let mut params = Vec::new();
        let mut has_arrow = self.eat(TokenKind::Arrow);
        let mut valid = true;

        if !has_arrow {
            loop {
                if self.at(TokenKind::RightBrace) {
                    valid = false;
                    break;
                }
                let prefix = self.parse_prefix()?;
                let type_name = if self.at(TokenKind::Def) || self.at(TokenKind::Var) {
                    self.advance();
                    None
                } else if self.looks_like_local_declaration() {
                    Some(self.parse_type_name()?)
                } else {
                    None
                };
                if !self.at_ident_like() {
                    valid = false;
                    break;
                }
                let token = self.advance();
                params.push(Param {
                    name: token.text(self.source).to_string(),
                    type_name,
                    annotations: prefix.annotations,
                    annotation_details: prefix.annotation_details,
                    type_annotations: self.last_type_annotations.clone(),
                    span: token.span,
                });
                if self.eat(TokenKind::Assign) {
                    let _ = self.parse_expression()?;
                }
                if self.eat(TokenKind::Comma) {
                    self.skip_newlines();
                    continue;
                }
                has_arrow = self.eat(TokenKind::Arrow);
                break;
            }
        }

        if !has_arrow || !valid {
            self.pos = saved;
            params.clear();
        }

        let mut statements = Vec::new();
        self.skip_separators();
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            statements.push(self.parse_statement()?);
            self.skip_separators();
        }
        let end = self.expect(TokenKind::RightBrace)?.span.end;

        Ok(Expr {
            kind: ExprKind::Closure {
                params,
                has_arrow,
                body: Block {
                    statements,
                    span: Span::new(start, end),
                },
            },
            span: Span::new(start, end),
        })
    }

    fn parse_command_arguments(&mut self, expr: Expr) -> Result<Expr, ParseError> {
        if !self.starts_argument() {
            return Ok(expr);
        }
        // Inside parentheses a line break is insignificant, so the parser may
        // already have consumed it. A command argument must still start on the
        // line of the callee: otherwise the next statement's first token would
        // be read as an argument (`check(a, b)` followed by `Foo x = ..`).
        let between = expr.span.end..self.current().span.start;
        if self
            .source
            .get(between)
            .is_some_and(|text| text.contains('\n'))
        {
            return Ok(expr);
        }

        let mut command_args = Vec::new();
        loop {
            if self.at_named_argument() {
                let name_start = self.current().span.start;
                let name = self.expect_name_text()?;
                self.advance();
                let value = self.parse_expression()?;
                command_args.push(Expr {
                    span: Span::new(name_start, value.span.end),
                    kind: ExprKind::NamedArg {
                        name,
                        value: Box::new(value),
                    },
                });
            } else {
                command_args.push(self.parse_expression()?);
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }

        let span = Span::new(expr.span.start, self.previous_span().end);
        match expr.kind {
            ExprKind::Variable(name) => Ok(Expr {
                kind: ExprKind::Call {
                    receiver: None,
                    name,
                    args: command_args,
                    safe: false,
                },
                span,
            }),
            ExprKind::Property {
                receiver,
                name,
                safe,
            } => Ok(Expr {
                kind: ExprKind::Call {
                    receiver: Some(receiver),
                    name,
                    args: command_args,
                    safe,
                },
                span,
            }),
            ExprKind::Call {
                receiver,
                name,
                args: mut existing,
                safe,
            } => {
                existing.extend(command_args);
                Ok(Expr {
                    kind: ExprKind::Call {
                        receiver,
                        name,
                        args: existing,
                        safe,
                    },
                    span,
                })
            }
            _ => Ok(expr),
        }
    }

    fn starts_expression(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Ident
                | TokenKind::Record
                | TokenKind::Number
                | TokenKind::String
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::New
                | TokenKind::This
                | TokenKind::Super
                | TokenKind::LeftParen
                | TokenKind::LeftBracket
                | TokenKind::LeftBrace
                | TokenKind::Not
                | TokenKind::Minus
                | TokenKind::Plus
                | TokenKind::Tilde
                | TokenKind::Inc
                | TokenKind::Dec
        )
    }

    fn starts_argument(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Not
                | TokenKind::Record
                | TokenKind::Ident
                | TokenKind::Number
                | TokenKind::String
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::New
                | TokenKind::This
                | TokenKind::Super
                | TokenKind::LeftBracket
                | TokenKind::LeftBrace
        )
    }

    /// Returns whether the tokens after the current line break resume a method
    /// chain (`.name`), which may follow any number of blank lines.
    /// Returns whether the tokens after the current line break open a closure
    /// argument for this expression, as in `items?.find` on one line and
    /// `{ ... }` on the next.
    fn next_code_starts_closure_argument(&self, expr: &Expr) -> bool {
        let takes_closure = matches!(expr.kind, ExprKind::Call { .. } | ExprKind::Property { .. });
        if !takes_closure {
            return false;
        }
        let mut offset = 1;
        while self.peek_kind(offset) == TokenKind::Newline {
            offset += 1;
        }
        self.peek_kind(offset) == TokenKind::LeftBrace
    }

    fn next_code_starts_chain(&self) -> bool {
        let mut offset = 1;
        while self.peek_kind(offset) == TokenKind::Newline {
            offset += 1;
        }
        matches!(self.peek_kind(offset), TokenKind::Dot | TokenKind::SafeDot)
    }

    /// Detects a `name: value` argument, whose name may be an identifier or a
    /// string literal.
    fn at_named_argument(&self) -> bool {
        let is_name = matches!(self.current().kind, TokenKind::Ident | TokenKind::String)
            || is_property_name_keyword(self.current().kind);
        is_name && self.peek_kind(1) == TokenKind::Colon
    }

    fn at_ident_like(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Ident | TokenKind::Record | TokenKind::Permits
        )
    }

    /// Groovy allows a return type and its method name on separate lines.
    fn skip_newlines_before_method_name(&mut self) {
        if !self.at(TokenKind::Newline) {
            return;
        }
        let mut offset = 1;
        while self.peek_kind(offset) == TokenKind::Newline {
            offset += 1;
        }
        if !self.at_ident_like_after(offset) {
            return;
        }
        if self.peek_kind(offset + 1) == TokenKind::LeftParen {
            self.skip_newlines();
        }
    }

    fn at_ident_like_after(&self, offset: usize) -> bool {
        matches!(
            self.peek_kind(offset),
            TokenKind::Ident | TokenKind::Record | TokenKind::Permits
        )
    }

    fn looks_like_local_declaration(&mut self) -> bool {
        if self.at(TokenKind::Def) || self.at(TokenKind::Var) {
            return true;
        }

        if !self.at(TokenKind::Ident) {
            return false;
        }

        let saved = self.pos;
        let result = self
            .parse_type_name()
            .is_ok_and(|name| is_type_like(&name) && self.at_ident_like());
        self.pos = saved;
        result
    }

    fn parse_prefix(&mut self) -> Result<PrefixInfo, ParseError> {
        let mut prefix = PrefixInfo::default();
        loop {
            self.skip_newlines();
            if self.at(TokenKind::At) && self.peek_kind(1) == TokenKind::Interface {
                break;
            }
            if self.eat(TokenKind::At) {
                let name = self.parse_qualified_name()?;
                let simple = simple_name(&name);
                prefix.annotations.push(simple.clone());
                let mut raw_arguments = None;
                if self.at(TokenKind::LeftParen) {
                    let open = self.current().span.start;
                    let args_start = self.pos + 1;
                    self.skip_balanced(TokenKind::LeftParen, TokenKind::RightParen)?;
                    let close = self.previous_span().start;
                    if close > open + 1 {
                        raw_arguments = Some(self.source[open + 1..close].trim().to_string());
                    }
                    if simple == "SuppressWarnings" {
                        for token in &self.tokens[args_start..self.pos.saturating_sub(1)] {
                            if token.kind == TokenKind::String {
                                prefix.suppressed_rules.push(
                                    token
                                        .text(self.source)
                                        .trim_matches('"')
                                        .trim_matches('\'')
                                        .to_string(),
                                );
                            }
                        }
                    }
                }
                prefix.annotation_details.push(Annotation {
                    name: simple,
                    raw_arguments,
                });
            } else if self.at(TokenKind::Static) {
                prefix.is_static = true;
                self.advance();
            } else if self.at(TokenKind::Abstract) {
                prefix.is_abstract = true;
                self.advance();
            } else if self.at(TokenKind::Final) {
                prefix.is_final = true;
                self.advance();
            } else if self.at(TokenKind::Public) {
                prefix.is_public = true;
                self.advance();
            } else if self.at(TokenKind::Private) {
                prefix.is_private = true;
                self.advance();
            } else if self.at(TokenKind::Protected) {
                prefix.is_protected = true;
                self.advance();
            } else if self.at(TokenKind::Synchronized) {
                prefix.is_synchronized = true;
                self.advance();
            } else if self.at(TokenKind::Transient) {
                prefix.is_transient = true;
                self.advance();
            } else if self.at(TokenKind::Volatile) {
                prefix.is_volatile = true;
                self.advance();
            } else if self.at(TokenKind::Default) {
                self.advance();
            } else {
                break;
            }
        }
        self.skip_newlines();
        Ok(prefix)
    }

    fn parse_type_name(&mut self) -> Result<String, ParseError> {
        self.last_type_annotations.clear();
        let mut name = self.expect_ident_text()?;
        loop {
            if self.at(TokenKind::Lt) {
                let start = self.pos + 1;
                self.skip_balanced(TokenKind::Lt, TokenKind::Gt)?;
                let end = self.pos.saturating_sub(1);
                let mut index = start;
                while index < end {
                    if self.tokens[index].kind == TokenKind::At
                        && index + 1 < end
                        && self.tokens[index + 1].kind == TokenKind::Ident
                    {
                        let annotation = self.tokens[index + 1].text(self.source).to_string();
                        if matches!(
                            annotation.as_str(),
                            "Nullable" | "NonNull" | "NotNull" | "MonotonicNonNull"
                        ) {
                            self.last_type_annotations.push(annotation);
                        }
                    }
                    index += 1;
                }
            } else if self.at(TokenKind::Dot) && self.peek_kind(1) == TokenKind::Ident {
                self.advance();
                let segment = self.expect_ident_text()?;
                name.push('.');
                name.push_str(&segment);
            } else {
                break;
            }
        }
        while self.at(TokenKind::LeftBracket) && self.peek_kind(1) == TokenKind::RightBracket {
            self.advance();
            self.advance();
            name.push_str("[]");
        }
        if self.eat(TokenKind::Ellipsis) {
            name.push_str("...");
        }
        Ok(name)
    }

    fn parse_qualified_name(&mut self) -> Result<String, ParseError> {
        let mut name = self.expect_ident_text()?;
        while self.at(TokenKind::Dot) && self.at_ident_like_after(1) {
            self.advance();
            let segment = self.expect_ident_text()?;
            name.push('.');
            name.push_str(&segment);
        }
        Ok(name)
    }

    fn skip_balanced(&mut self, open: TokenKind, close: TokenKind) -> Result<(), ParseError> {
        self.expect(open)?;
        let mut depth = 1usize;
        while !self.at(TokenKind::Eof) {
            if self.at(open) {
                depth += 1;
            } else if self.at(close) {
                depth -= 1;
                if depth == 0 {
                    self.advance();
                    return Ok(());
                }
            } else if self.at(TokenKind::ShiftRight) && depth >= 2 {
                depth -= 2;
                if depth == 0 {
                    self.advance();
                    return Ok(());
                }
            }
            self.advance();
        }
        Err(self.error_here(format!("unclosed {}", open.describe())))
    }

    fn is_class_keyword(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Class
                | TokenKind::Interface
                | TokenKind::Trait
                | TokenKind::Enum
                | TokenKind::Record
        )
    }

    fn expect_name_text(&mut self) -> Result<String, ParseError> {
        let token = self.current();
        match token.kind {
            TokenKind::Ident => {
                self.advance();
                Ok(token.text(self.source).to_string())
            }
            TokenKind::String => {
                self.advance();
                Ok(token
                    .text(self.source)
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string())
            }
            kind if is_property_name_keyword(kind) => {
                self.advance();
                Ok(token.text(self.source).to_string())
            }
            _ => Err(self.error_here(format!(
                "expected property name, found {}",
                token.kind.describe()
            ))),
        }
    }

    fn expect_ident(&mut self) -> Result<Token, ParseError> {
        if self.at_ident_like() {
            return Ok(self.advance());
        }
        Err(self.error_here(format!(
            "expected identifier, found {}",
            self.current().kind.describe()
        )))
    }

    fn expect_ident_text(&mut self) -> Result<String, ParseError> {
        let token = self.expect_ident()?;
        Ok(token.text(self.source).to_string())
    }

    fn current(&self) -> Token {
        self.tokens
            .get(self.pos)
            .copied()
            .unwrap_or_else(|| *self.tokens.last().expect("token stream has EOF"))
    }

    fn peek_kind(&self, offset: usize) -> TokenKind {
        self.tokens
            .get(self.pos + offset)
            .map_or(TokenKind::Eof, |token| token.kind)
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, ParseError> {
        if self.at(kind) {
            Ok(self.advance())
        } else {
            Err(self.error_here(format!(
                "expected {}, found {}",
                kind.describe(),
                self.current().kind.describe()
            )))
        }
    }

    fn advance(&mut self) -> Token {
        let token = self.current();
        if !self.at(TokenKind::Eof) {
            self.pos += 1;
        }
        token
    }

    fn previous_span(&self) -> Span {
        if self.pos == 0 {
            return Span::empty(0);
        }
        self.tokens
            .get(self.pos - 1)
            .map_or_else(|| Span::empty(0), |token| token.span)
    }

    fn skip_newlines(&mut self) {
        while self.eat(TokenKind::Newline) {}
    }

    /// Skips a line break before one of `operators`. Inside parentheses and
    /// brackets any line break is insignificant; at statement level only a line
    /// that starts with an operator unable to begin a statement continues the
    /// expression (`value\n    && other`).
    fn skip_newlines_before_operator(&mut self, operators: &[(TokenKind, BinaryOp)]) {
        if self.nesting > 0 {
            self.skip_newlines();
            return;
        }
        if !self.at(TokenKind::Newline) {
            return;
        }
        let mut offset = 1;
        while self.peek_kind(offset) == TokenKind::Newline {
            offset += 1;
        }
        let next = self.peek_kind(offset);
        if !continues_expression(next) {
            return;
        }
        if operators.iter().any(|(kind, _)| *kind == next) {
            self.skip_newlines();
        }
    }

    /// Skips a line break that continues the current expression, which is only
    /// unambiguous when the next token can never start a statement.
    fn skip_continuation_newline(&mut self, follows: TokenKind) {
        if !self.at(TokenKind::Newline) {
            return;
        }
        let mut offset = 1;
        while self.peek_kind(offset) == TokenKind::Newline {
            offset += 1;
        }
        if self.peek_kind(offset) == follows {
            self.skip_newlines();
        }
    }

    fn skip_separators(&mut self) {
        while matches!(
            self.current().kind,
            TokenKind::Newline | TokenKind::Semicolon
        ) {
            self.advance();
        }
    }

    fn error_here(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            span: self.current().span,
        }
    }
}

/// Returns whether a line starting with this token continues the previous
/// expression instead of starting a new statement.
fn continues_expression(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::AndAnd
            | TokenKind::OrOr
            | TokenKind::Eq
            | TokenKind::NotEq
            | TokenKind::RegexFind
            | TokenKind::RegexMatch
            | TokenKind::Le
            | TokenKind::Ge
            | TokenKind::Range
            | TokenKind::RangeExclusive
            | TokenKind::Instanceof
            | TokenKind::In
            | TokenKind::ShiftLeft
            | TokenKind::ShiftRight
            | TokenKind::Amp
            | TokenKind::Pipe
            | TokenKind::Caret
            | TokenKind::Power
            | TokenKind::Star
    )
}

/// Keeps a single `for` declarator as is, and groups several into a block.
fn combine_declarations(mut declarations: Vec<Stmt>) -> Stmt {
    // One declaration shares its written type between the declarators:
    // `int i = 0, size = rows.size()` declares both as `int`.
    let shared_type = match declarations.first() {
        Some(Stmt::VarDecl(first)) => first.type_name.clone(),
        _ => None,
    };
    if let Some(type_name) = shared_type {
        for declaration in &mut declarations {
            if let Stmt::VarDecl(declaration) = declaration
                && declaration.type_name.is_none()
            {
                declaration.type_name = Some(type_name.clone());
            }
        }
    }

    if declarations.len() == 1
        && let Some(declaration) = declarations.pop()
    {
        return declaration;
    }
    let span = match (declarations.first(), declarations.last()) {
        (Some(first), Some(last)) => first.span().cover(last.span()),
        _ => Span::empty(0),
    };
    Stmt::Block(Block {
        statements: declarations,
        span,
    })
}

fn is_type_like(name: &str) -> bool {
    let name = name.trim_end_matches("...");
    let name = name.trim_end_matches("[]");
    let last = name.rsplit('.').next().unwrap_or(name);
    last.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
        || matches!(
            last,
            "boolean" | "byte" | "char" | "short" | "int" | "long" | "float" | "double" | "void"
        )
}

fn is_property_name_keyword(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Package
            | TokenKind::Import
            | TokenKind::As
            | TokenKind::Class
            | TokenKind::Interface
            | TokenKind::Trait
            | TokenKind::Enum
            | TokenKind::Record
            | TokenKind::Def
            | TokenKind::Var
            | TokenKind::If
            | TokenKind::Else
            | TokenKind::While
            | TokenKind::For
            | TokenKind::In
            | TokenKind::Return
            | TokenKind::Throw
            | TokenKind::Try
            | TokenKind::Catch
            | TokenKind::Finally
            | TokenKind::Break
            | TokenKind::Continue
            | TokenKind::New
            | TokenKind::This
            | TokenKind::Super
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Null
            | TokenKind::Void
            | TokenKind::Assert
            | TokenKind::Switch
            | TokenKind::Case
            | TokenKind::Default
            | TokenKind::Do
            | TokenKind::Static
            | TokenKind::Final
            | TokenKind::Abstract
            | TokenKind::Public
            | TokenKind::Private
            | TokenKind::Protected
            | TokenKind::Synchronized
            | TokenKind::Transient
            | TokenKind::Volatile
            | TokenKind::Throws
            | TokenKind::Instanceof
            | TokenKind::Extends
            | TokenKind::Implements
            | TokenKind::Permits
    )
}

#[derive(Default)]
#[allow(clippy::struct_excessive_bools)]
struct PrefixInfo {
    annotations: Vec<String>,
    annotation_details: Vec<Annotation>,
    suppressed_rules: Vec<String>,
    is_static: bool,
    is_abstract: bool,
    is_final: bool,
    is_public: bool,
    is_private: bool,
    is_protected: bool,
    is_synchronized: bool,
    is_transient: bool,
    is_volatile: bool,
}

fn simple_name(name: &str) -> String {
    name.rsplit('.').next().unwrap_or(name).to_string()
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::ast::{ExprKind, Item, Script, Stmt};

    fn script(source: &str) -> Script {
        parse(source).expect("source should parse").script
    }

    fn first_method(script: &Script) -> &crate::ast::MethodDecl {
        let Item::Class(class) = &script.items[0] else {
            panic!("expected class");
        };
        let Item::Method(method) = &class.members[0] else {
            panic!("expected method");
        };
        method
    }

    #[test]
    fn keeps_statements_apart_inside_call_parentheses() {
        // A line break inside parentheses is insignificant for operators, but it
        // still ends a statement: `Foo scope = ..` must not become another
        // argument of `check(..)`.
        let script = script(
            "class Example {\n \
                 void run() {\n \
                     svc.exec(1, {\n \
                         Check.check(a, b)\n \
                         Foo scope = Foo.restore()\n \
                     })\n \
                 }\n \
             }\n",
        );
        let method = first_method(&script);
        let statements = &method.body.as_ref().expect("body").statements;
        assert_eq!(statements.len(), 1, "{statements:?}");
        let Stmt::Expr(exec) = &statements[0] else {
            panic!("expected a call");
        };
        let ExprKind::Call { args, .. } = &exec.kind else {
            panic!("expected a call");
        };
        let ExprKind::Closure { body, .. } = &args[1].kind else {
            panic!("expected a closure");
        };
        assert_eq!(body.statements.len(), 2, "{:?}", body.statements);
        assert!(
            matches!(body.statements[1], Stmt::VarDecl(_)),
            "{:?}",
            body.statements[1]
        );
    }

    #[test]
    fn parses_empty_control_flow_blocks() {
        let script = script(
            r"
class Example {
    void run() {
        if (true) {
        } else {
        }
        while (false) {
        }
        for (def i = 0; i < 3; i++) {
        }
        try {
        } catch (Exception error) {
        } finally {
        }
    }
}
",
        );

        let method = first_method(&script);
        let body = method.body.as_ref().expect("method body");
        assert_eq!(body.statements.len(), 4);

        let Stmt::If(if_stmt) = &body.statements[0] else {
            panic!("expected if");
        };
        assert!(if_stmt.then_branch.is_empty_block());
        assert!(
            if_stmt
                .else_branch
                .as_deref()
                .is_some_and(Stmt::is_empty_block)
        );

        assert!(matches!(&body.statements[1], Stmt::While(stmt) if stmt.body.is_empty_block()));
        assert!(matches!(&body.statements[2], Stmt::For(stmt) if stmt.body.is_empty_block()));

        let Stmt::Try(try_stmt) = &body.statements[3] else {
            panic!("expected try");
        };
        assert!(try_stmt.body.statements.is_empty());
        assert!(try_stmt.catches[0].body.statements.is_empty());
        assert!(
            try_stmt
                .finally
                .as_ref()
                .is_some_and(|block| block.statements.is_empty())
        );
    }

    #[test]
    fn parses_command_style_calls_and_declarations() {
        let script = script("println \"hello\"\ndef value = 1\n");
        assert_eq!(script.items.len(), 2);

        let Item::Statement(Stmt::Expr(expr)) = &script.items[0] else {
            panic!("expected expression statement");
        };
        assert!(matches!(&expr.kind, ExprKind::Call { name, .. } if name == "println"));

        let Item::Statement(Stmt::VarDecl(decl)) = &script.items[1] else {
            panic!("expected variable declaration");
        };
        assert_eq!(decl.name, "value");
    }

    #[test]
    fn parses_closures_with_parameters() {
        let script = script("def double = { value -> value * 2 }\n");
        let Item::Statement(Stmt::VarDecl(decl)) = &script.items[0] else {
            panic!("expected variable declaration");
        };
        let init = decl.init.as_ref().expect("initializer");
        assert!(matches!(&init.kind, ExprKind::Closure { params, .. } if params.len() == 1));
    }

    #[test]
    fn parses_common_groovy_syntax_variants() {
        let source = r"
enum Mode {
    A,
    B;
}

class Example<T> {
    void varargs(String... names) {
        for (Map.Entry<String, List<T>> entry : mappings.entrySet()) {
        }
        for (Class<? extends Object> type : types) {
        }
        try (InputStream stream = new FileInputStream(file)) {
            stream.read()
        }
        def typed = items.collect { List<String> values -> values }
        def lambda = items.map(() -> factory.create())
        boolean ok = left != null
                && right != null
        Comparable sorted = left <=> right
        def reference = Objects::isNull
        def explicit = Collections.<String>emptyList()
        def literal = ListContextValue<String>.class
        def chained = builder.build()

                .next()
        def map = [key: 1]
    }
}
";
        let script = script(source);
        let Item::Class(class) = &script.items[1] else {
            panic!("expected class");
        };
        let Item::Method(method) = &class.members[0] else {
            panic!("expected method");
        };
        let body = method.body.as_ref().expect("method body");
        assert_eq!(body.statements.len(), 12);
        assert!(matches!(body.statements[0], Stmt::ForEach(_)));
        assert!(matches!(body.statements[2], Stmt::Try(_)));
    }

    #[test]
    fn parses_further_groovy_syntax() {
        let source = r#"
class Further {
    static <T> T[] varargsHolder(Object... args) {
        def spread = [*args, 1]
        call(*args)
        boolean negated = args !in others
        boolean identity = args === others
        int shifted = value >>> 4
        def voidClass = void.class
        def keywordKey = [class: "value", default: 1]
        def genericCall = Collections.<String>emptyList()
        def genericMember = APIResp<String>.wrapResult { 1 }
        def chained = items?.find
                { item -> item.value }
        return null
    }

    do {
        tick()
    } while (ready)

    String routed = switch (kind) {
        case Type.FULL -> "full"
        default -> "other"
    }

    Value<?> lookup(Object ext) {
        def cast = (Value)
                ext
        return (Value) ext?[0]
    }
}
"#;
        let script = script(source);
        assert_eq!(script.items.len(), 1);
        let Item::Class(class) = &script.items[0] else {
            panic!("expected class");
        };
        assert_eq!(class.members.len(), 4);
        assert!(matches!(
            class.members[1],
            Item::Statement(Stmt::DoWhile(_))
        ));

        let Item::Method(method) = &class.members[0] else {
            panic!("expected method");
        };
        let body = method.body.as_ref().expect("method body");
        assert_eq!(body.statements.len(), 11);
        assert!(matches!(
            &body.statements[8],
            Stmt::VarDecl(decl) if matches!(
                &decl.init.as_ref().expect("initializer").kind,
                ExprKind::Call { name, .. } if name == "wrapResult"
            )
        ));
    }

    #[test]
    fn keeps_semicolons_in_token_stream() {
        let output = parse("def value = 1;\n").expect("source should parse");
        assert!(
            output
                .tokens
                .iter()
                .any(|token| token.kind == crate::token::TokenKind::Semicolon)
        );
    }
}
