//! Recursive-descent parser for the supported Groovy subset.
//!
//! This is intentionally not a full Groovy parser. It covers the constructs
//! needed by the first rule slice and common class/script shapes, and will be
//! extended alongside the rule set.

use groovylint_core::Span;

use crate::ast::{
    AssertStmt, AssignOp, BinaryOp, Block, CatchClause, ClassDecl, ClassKind, Expr, ExprKind,
    FieldDecl, ForEachStmt, ForStmt, IfStmt, ImportDecl, Item, Literal, MethodDecl, PackageDecl,
    Param, ReturnStmt, Script, Stmt, ThrowStmt, TryStmt, UnaryOp, VarDecl, WhileStmt,
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
}

impl<'a> Parser<'a> {
    const fn new(source: &'a str, tokens: &'a [Token]) -> Self {
        Self {
            source,
            tokens,
            pos: 0,
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
        self.eat(TokenKind::Static);
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
            span: Span::new(start, self.previous_span().end),
        })
    }

    fn parse_item(&mut self, in_class: bool) -> Result<Item, ParseError> {
        self.skip_prefix();
        let start = self.current().span.start;

        if self.is_class_keyword() {
            return self.parse_class().map(Item::Class);
        }

        if self.at(TokenKind::Def) {
            self.advance();
            if self.at(TokenKind::Ident) && self.peek_kind(1) == TokenKind::LeftParen {
                return self.parse_method(start).map(Item::Method);
            }
            let name = self.expect_ident_text()?;
            return self.parse_field_or_var(start, name, in_class);
        }

        if self.at(TokenKind::Void) {
            self.advance();
            return self.parse_method(start).map(Item::Method);
        }

        if self.at(TokenKind::Ident) {
            if in_class && self.peek_kind(1) == TokenKind::LeftParen {
                return self.parse_method(start).map(Item::Method);
            }

            let saved = self.pos;
            if let Ok(_) = self.parse_type_name()
                && self.at(TokenKind::Ident)
            {
                if self.peek_kind(1) == TokenKind::LeftParen {
                    return self.parse_method(start).map(Item::Method);
                }
                let name = self.expect_ident_text()?;
                return self.parse_field_or_var(start, name, in_class);
            }
            self.pos = saved;
        }

        self.parse_statement().map(Item::Statement)
    }

    fn parse_class(&mut self) -> Result<ClassDecl, ParseError> {
        let start = self.current().span.start;
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

        while !self.at(TokenKind::LeftBrace) && !self.at(TokenKind::Eof) {
            self.advance();
        }

        let _ = self.expect(TokenKind::LeftBrace)?;
        let mut members = Vec::new();
        self.skip_separators();

        if kind == ClassKind::Enum {
            self.skip_enum_constants();
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
        Ok(ClassDecl {
            name,
            kind,
            members,
            span: Span::new(start, self.previous_span().end),
        })
    }

    fn skip_enum_constants(&mut self) {
        // Enum constants are not needed by the first rules. Skip to the first
        // top-level semicolon or the closing brace.
        let mut depth = 0usize;
        while !self.at(TokenKind::Eof) {
            match self.current().kind {
                TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::LeftBrace => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace => {
                    if self.current().kind == TokenKind::RightBrace && depth == 0 {
                        return;
                    }
                    depth = depth.saturating_sub(1);
                    self.advance();
                }
                TokenKind::Semicolon if depth == 0 => {
                    self.advance();
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn parse_method(&mut self, start: usize) -> Result<MethodDecl, ParseError> {
        let name = self.expect_ident_text()?;
        self.expect(TokenKind::LeftParen)?;
        let mut params = Vec::new();
        self.skip_newlines();

        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            self.skip_prefix();
            if self.at(TokenKind::Ident)
                && matches!(self.peek_kind(1), TokenKind::Comma | TokenKind::RightParen)
            {
                let token = self.advance();
                params.push(Param {
                    name: token.text(self.source).to_string(),
                    span: token.span,
                });
            } else {
                if self.at(TokenKind::Def) || self.at(TokenKind::Var) {
                    self.advance();
                } else if self.at(TokenKind::Ident) {
                    let _ = self.parse_type_name()?;
                }
                let token = self.expect_ident()?;
                params.push(Param {
                    name: token.text(self.source).to_string(),
                    span: token.span,
                });
            }

            if self.eat(TokenKind::Assign) {
                let _ = self.parse_expression()?;
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }

        self.expect(TokenKind::RightParen)?;
        self.skip_newlines();

        if self.eat(TokenKind::Throws) {
            while !self.at(TokenKind::LeftBrace)
                && !self.at(TokenKind::Semicolon)
                && !self.at(TokenKind::Newline)
                && !self.at(TokenKind::Eof)
            {
                self.advance();
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
            params,
            body,
            span: Span::new(start, self.previous_span().end),
        })
    }

    fn parse_field_or_var(
        &mut self,
        start: usize,
        name: String,
        in_class: bool,
    ) -> Result<Item, ParseError> {
        let init = if self.eat(TokenKind::Assign) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.eat(TokenKind::Semicolon);
        self.skip_newlines();

        if in_class {
            Ok(Item::Field(FieldDecl {
                name,
                init,
                span: Span::new(start, self.previous_span().end),
            }))
        } else {
            Ok(Item::Statement(Stmt::VarDecl(VarDecl {
                name,
                init,
                span: Span::new(start, self.previous_span().end),
            })))
        }
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        self.skip_separators();
        let start = self.current().span.start;

        match self.current().kind {
            TokenKind::LeftBrace => return self.parse_block().map(Stmt::Block),
            TokenKind::If => return self.parse_if(start),
            TokenKind::While => return self.parse_while(start),
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
            TokenKind::Synchronized => {
                self.advance();
                self.expect(TokenKind::LeftParen)?;
                let _ = self.parse_expression()?;
                self.expect(TokenKind::RightParen)?;
                self.skip_newlines();
                return self.parse_statement();
            }
            _ => {}
        }

        if self.at(TokenKind::At) {
            self.skip_prefix();
        }

        if self.looks_like_local_declaration() {
            self.skip_prefix();
            let start = self.current().span.start;
            if !self.eat(TokenKind::Def) && !self.eat(TokenKind::Var) {
                let _ = self.parse_type_name()?;
            }
            let name = self.expect_ident_text()?;
            let init = if self.eat(TokenKind::Assign) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.eat(TokenKind::Semicolon);
            self.skip_newlines();
            return Ok(Stmt::VarDecl(VarDecl {
                name,
                init,
                span: Span::new(start, self.previous_span().end),
            }));
        }

        let expr = self.parse_expression()?;
        let expr = self.parse_command_arguments(expr)?;
        self.eat(TokenKind::Semicolon);
        self.skip_newlines();

        Ok(Stmt::Expr(expr))
    }

    fn parse_if(&mut self, start: usize) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::If)?;
        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
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
        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        self.skip_newlines();
        let body = Box::new(self.parse_statement()?);
        Ok(Stmt::While(WhileStmt {
            condition,
            body,
            span: Span::new(start, self.previous_span().end),
        }))
    }

    fn parse_for(&mut self, start: usize) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::For)?;
        self.expect(TokenKind::LeftParen)?;
        self.skip_newlines();

        let after_paren = self.pos;
        let foreach = self.try_parse_foreach(start);
        if let Ok(stmt) = foreach {
            return Ok(stmt);
        }
        self.pos = after_paren;

        let init = if self.at(TokenKind::Semicolon) {
            None
        } else if self.looks_like_local_declaration() {
            self.skip_prefix();
            let init_start = self.current().span.start;
            if self.at(TokenKind::Def) || self.at(TokenKind::Var) {
                self.advance();
            } else {
                let _ = self.parse_type_name()?;
            }
            let name = self.expect_ident_text()?;
            let value = if self.eat(TokenKind::Assign) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            Some(Box::new(Stmt::VarDecl(VarDecl {
                name,
                init: value,
                span: Span::new(init_start, self.previous_span().end),
            })))
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

    fn try_parse_foreach(&mut self, start: usize) -> Result<Stmt, ParseError> {
        self.skip_prefix();
        if self.at(TokenKind::Def) || self.at(TokenKind::Var) {
            self.advance();
        } else if self.at(TokenKind::Ident) && self.peek_kind(1) == TokenKind::Ident {
            let _ = self.parse_type_name()?;
        }
        let var_name = self.expect_ident_text()?;
        if !self.eat(TokenKind::In) && !self.eat(TokenKind::Colon) {
            return Err(self.error_here("expected `in`"));
        }
        let iterable = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        self.skip_newlines();
        let body = Box::new(self.parse_statement()?);
        Ok(Stmt::ForEach(ForEachStmt {
            var_name,
            iterable,
            body,
            span: Span::new(start, self.previous_span().end),
        }))
    }

    fn parse_try(&mut self, start: usize) -> Result<Stmt, ParseError> {
        self.expect(TokenKind::Try)?;
        let body = self.parse_block()?;
        let mut catches = Vec::new();
        self.skip_newlines();

        while self.eat(TokenKind::Catch) {
            let catch_start = self.previous_span().start;
            self.expect(TokenKind::LeftParen)?;
            while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
                self.advance();
            }
            self.expect(TokenKind::RightParen)?;
            self.skip_newlines();
            let block = self.parse_block()?;
            catches.push(CatchClause {
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
        let message = if self.eat(TokenKind::Colon) {
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

    fn parse_assignment(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_ternary()?;
        let op = match self.current().kind {
            TokenKind::Assign => Some(AssignOp::Assign),
            TokenKind::PlusAssign => Some(AssignOp::Add),
            TokenKind::MinusAssign => Some(AssignOp::Sub),
            TokenKind::StarAssign => Some(AssignOp::Mul),
            TokenKind::SlashAssign => Some(AssignOp::Div),
            TokenKind::PercentAssign => Some(AssignOp::Mod),
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
        if self.eat(TokenKind::Question) {
            self.skip_newlines();
            let then_expr = self.parse_expression()?;
            self.expect(TokenKind::Colon)?;
            self.skip_newlines();
            let else_expr = self.parse_ternary()?;
            let span = condition.span.cover(else_expr.span);
            return Ok(Expr {
                kind: ExprKind::Ternary {
                    condition: Box::new(condition),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                },
                span,
            });
        }
        if self.eat(TokenKind::Elvis) {
            self.skip_newlines();
            let else_expr = self.parse_ternary()?;
            let span = condition.span.cover(else_expr.span);
            return Ok(Expr {
                kind: ExprKind::Ternary {
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
            ],
        )
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(
            Self::parse_shift,
            &[
                (TokenKind::Lt, BinaryOp::Lt),
                (TokenKind::Le, BinaryOp::Le),
                (TokenKind::Gt, BinaryOp::Gt),
                (TokenKind::Ge, BinaryOp::Ge),
                (TokenKind::Instanceof, BinaryOp::InstanceOf),
                (TokenKind::Range, BinaryOp::Range),
                (TokenKind::RangeExclusive, BinaryOp::RangeExclusive),
            ],
        )
    }

    fn parse_shift(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary(
            Self::parse_additive,
            &[
                (TokenKind::ShiftLeft, BinaryOp::ShiftLeft),
                (TokenKind::ShiftRight, BinaryOp::ShiftRight),
            ],
        )
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
        while let Some((_, op)) = operators.iter().find(|(kind, _)| self.at(*kind)).copied() {
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
            match self.current().kind {
                TokenKind::Dot | TokenKind::SafeDot => {
                    let safe = self.current().kind == TokenKind::SafeDot;
                    self.advance();
                    let name = self.expect_ident_text()?;
                    if self.at(TokenKind::LeftParen) {
                        let args = self.parse_arguments()?;
                        let end = self.previous_span().end;
                        let start = expr.span.start;
                        let receiver = Some(Box::new(expr));
                        expr = Expr {
                            kind: ExprKind::Call {
                                receiver,
                                name,
                                args,
                                safe,
                            },
                            span: Span::new(start, end),
                        };
                    } else {
                        let span = Span::new(expr.span.start, self.previous_span().end);
                        expr = Expr {
                            kind: ExprKind::Property {
                                receiver: Box::new(expr),
                                name,
                                safe,
                            },
                            span,
                        };
                    }
                }
                TokenKind::LeftBracket => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(TokenKind::RightBracket)?;
                    let span = Span::new(expr.span.start, self.previous_span().end);
                    expr = Expr {
                        kind: ExprKind::Index {
                            target: Box::new(expr),
                            index: Box::new(index),
                            safe: false,
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
                    let span = Span::new(expr.span.start, closure.span.end);
                    match expr.kind {
                        ExprKind::Call {
                            receiver,
                            name,
                            mut args,
                            safe,
                        } => {
                            args.push(closure);
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
                        ExprKind::Variable(name) => {
                            expr = Expr {
                                kind: ExprKind::Call {
                                    receiver: None,
                                    name,
                                    args: vec![closure],
                                    safe: false,
                                },
                                span,
                            };
                        }
                        ExprKind::Property {
                            receiver,
                            name,
                            safe,
                        } => {
                            expr = Expr {
                                kind: ExprKind::Call {
                                    receiver: Some(receiver),
                                    name,
                                    args: vec![closure],
                                    safe,
                                },
                                span,
                            };
                        }
                        _ => break,
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
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
            TokenKind::Ident => {
                self.advance();
                let name = token.text(self.source).to_string();
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
                self.advance();
                let inner = self.parse_expression()?;
                self.expect(TokenKind::RightParen)?;
                Ok(Expr {
                    kind: ExprKind::Paren(Box::new(inner)),
                    span: Span::new(start, self.previous_span().end),
                })
            }
            TokenKind::LeftBracket => self.parse_list_or_map(start),
            TokenKind::LeftBrace => self.parse_closure(),
            _ => Err(self.error_here(format!(
                "expected expression, found {}",
                token.kind.describe()
            ))),
        }
    }

    fn parse_new(&mut self, start: usize) -> Result<Expr, ParseError> {
        self.expect(TokenKind::New)?;
        let type_name = self.parse_type_name()?;
        let args = self.parse_arguments()?;
        Ok(Expr {
            kind: ExprKind::New { type_name, args },
            span: Span::new(start, self.previous_span().end),
        })
    }

    fn parse_arguments(&mut self) -> Result<Vec<Expr>, ParseError> {
        self.expect(TokenKind::LeftParen)?;
        let mut args = Vec::new();
        self.skip_newlines();

        while !self.at(TokenKind::RightParen) && !self.at(TokenKind::Eof) {
            args.push(self.parse_expression()?);
            self.skip_newlines();
            if !self.eat(TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }

        self.expect(TokenKind::RightParen)?;
        Ok(args)
    }

    fn parse_list_or_map(&mut self, start: usize) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LeftBracket)?;
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

        let first = self.parse_expression()?;
        if self.eat(TokenKind::Colon) {
            let mut entries = vec![(first, self.parse_expression()?)];
            while self.eat(TokenKind::Comma) {
                self.skip_newlines();
                if self.at(TokenKind::RightBracket) {
                    break;
                }
                let key = self.parse_expression()?;
                self.expect(TokenKind::Colon)?;
                let value = self.parse_expression()?;
                entries.push((key, value));
            }
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
            items.push(self.parse_expression()?);
        }
        self.expect(TokenKind::RightBracket)?;
        Ok(Expr {
            kind: ExprKind::List(items),
            span: Span::new(start, self.previous_span().end),
        })
    }

    fn parse_closure(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect(TokenKind::LeftBrace)?.span.start;
        self.skip_newlines();

        let saved = self.pos;
        let mut params = Vec::new();
        let mut has_arrow = self.eat(TokenKind::Arrow);

        while !has_arrow && self.at(TokenKind::Ident) {
            let token = self.advance();
            params.push(Param {
                name: token.text(self.source).to_string(),
                span: token.span,
            });
            if self.eat(TokenKind::Comma) {
                self.skip_newlines();
                continue;
            }
            has_arrow = self.eat(TokenKind::Arrow);
            break;
        }

        if !has_arrow {
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

        let mut command_args = Vec::new();
        loop {
            command_args.push(self.parse_expression()?);
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

    fn starts_argument(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Ident
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

    fn looks_like_local_declaration(&mut self) -> bool {
        if self.at(TokenKind::Def) || self.at(TokenKind::Var) {
            return true;
        }

        if !self.at(TokenKind::Ident) {
            return false;
        }

        let saved = self.pos;
        let result = self.parse_type_name().is_ok() && self.at(TokenKind::Ident);
        self.pos = saved;
        result
    }

    fn skip_prefix(&mut self) {
        loop {
            if self.eat(TokenKind::At) {
                let _ = self.parse_qualified_name();
                if self.at(TokenKind::LeftParen) {
                    let _ = self.skip_balanced(TokenKind::LeftParen, TokenKind::RightParen);
                }
            } else if matches!(
                self.current().kind,
                TokenKind::Public
                    | TokenKind::Private
                    | TokenKind::Protected
                    | TokenKind::Static
                    | TokenKind::Final
                    | TokenKind::Abstract
                    | TokenKind::Synchronized
                    | TokenKind::Transient
                    | TokenKind::Volatile
            ) {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn parse_type_name(&mut self) -> Result<String, ParseError> {
        let mut name = self.expect_ident_text()?;
        if self.at(TokenKind::Lt) {
            self.skip_balanced(TokenKind::Lt, TokenKind::Gt)?;
        }
        while self.at(TokenKind::Dot) && self.peek_kind(1) == TokenKind::Ident {
            self.advance();
            name.push('.');
            name.push_str(&self.expect_ident_text()?);
        }
        while self.at(TokenKind::LeftBracket) && self.peek_kind(1) == TokenKind::RightBracket {
            self.advance();
            self.advance();
            name.push_str("[]");
        }
        Ok(name)
    }

    fn parse_qualified_name(&mut self) -> Result<String, ParseError> {
        let mut name = self.expect_ident_text()?;
        while self.eat(TokenKind::Dot) {
            name.push('.');
            name.push_str(&self.expect_ident_text()?);
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

    fn expect_ident(&mut self) -> Result<Token, ParseError> {
        self.expect(TokenKind::Ident)
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
