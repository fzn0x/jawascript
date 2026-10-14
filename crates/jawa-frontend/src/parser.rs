//! A recursive-descent parser from tokens to the [`ast`](crate::ast).
//!
//! Expression precedence, loosest first:
//!
//! | Level        | Operators                       |
//! | ------------ | ------------------------------- |
//! | or           | `utawa`                         |
//! | and          | `lan`                           |
//! | not          | `ora` (prefix)                  |
//! | equality     | `==` `!=`                       |
//! | relational   | `<` `<=` `>` `>=`               |
//! | additive     | `+` `-`                         |
//! | multiplicative | `*` `/` `%`                   |
//! | unary        | `-` (prefix)                    |
//! | postfix      | calls, `a[i]`, `a.field`        |
//!
//! `ora` sits below the comparisons, as `not` does in Python, so
//! `ora x == 0` means `ora (x == 0)`.
//!
//! Errors are collected rather than returned: after a bad statement the parser
//! skips to the end of it and carries on, so one run reports every problem.

use crate::ast::*;
use crate::diag::Diagnostic;
use crate::hints;
use crate::lexer;
use crate::source::Span;
use crate::token::{TemplatePiece, Token, TokenKind};

pub struct Parsed {
    pub program: Program,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse(text: &str) -> Parsed {
    let lexed = lexer::lex(text);
    let mut parser = Parser {
        tokens: lexed.tokens,
        pos: 0,
        diagnostics: lexed.diagnostics,
        no_struct: false,
        depth: 0,
    };
    let program = parser.program();
    Parsed {
        program,
        diagnostics: parser.diagnostics,
    }
}

/// The error has already been reported; the caller only needs to recover.
type PResult<T> = Result<T, ()>;

/// How deeply statements and expressions may nest before the parser gives up,
/// so hostile input cannot overflow the stack.
const MAX_DEPTH: u32 = 256;

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    /// Set while parsing a `yen`/`sajrone`/`kanggo` header, where `Name {`
    /// starts the body rather than a struct literal.
    no_struct: bool,
    depth: u32,
}

impl Parser {
    // ---- Token helpers -------------------------------------------------

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_at(&self, offset: usize) -> &TokenKind {
        let index = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[index].kind
    }

    fn span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn prev_span(&self) -> Span {
        self.tokens[self.pos.saturating_sub(1)].span
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if token.kind != TokenKind::Eof {
            self.pos += 1;
        }
        token
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn error_at(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(span, message));
    }

    fn error_here(&mut self, message: impl Into<String>) {
        let span = self.span();
        self.error_at(span, message);
    }

    fn expect(&mut self, kind: &TokenKind, context: &str) -> PResult<Token> {
        if self.at(kind) {
            return Ok(self.advance());
        }
        let found = self.peek().clone();
        let mut diag = Diagnostic::error(
            self.span(),
            format!("expected {kind} {context}, found {found}"),
        );
        if matches!(found, TokenKind::PlusPlus | TokenKind::MinusMinus) {
            diag = diag.with_note(format!(
                "{found} only works as a statement of its own, such as `i++`"
            ));
        }
        self.diagnostics.push(diag);
        Err(())
    }

    fn ident(&mut self, context: &str) -> PResult<Ident> {
        match self.peek().clone() {
            TokenKind::Ident(name) => {
                let span = self.span();
                self.advance();
                Ok(Ident { name, span })
            }
            found => {
                self.error_here(format!("expected a name {context}, found {found}"));
                Err(())
            }
        }
    }

    fn skip_terminators(&mut self) {
        while self.peek().is_terminator() {
            self.advance();
        }
    }

    fn skip_newlines(&mut self) {
        while self.at(&TokenKind::Newline) {
            self.advance();
        }
    }

    fn enter(&mut self) -> PResult<()> {
        if self.depth >= MAX_DEPTH {
            self.error_here("this code nests too deeply");
            return Err(());
        }
        self.depth += 1;
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    // ---- Items ---------------------------------------------------------

    fn program(&mut self) -> Program {
        let mut items = Vec::new();
        loop {
            self.skip_terminators();
            match self.peek() {
                TokenKind::Eof => break,
                TokenKind::Gawe => match self.function() {
                    Ok(function) => items.push(Item::Function(function)),
                    Err(()) => self.recover_item(),
                },
                TokenKind::Wujud => match self.struct_decl() {
                    Ok(decl) => items.push(Item::Struct(decl)),
                    Err(()) => self.recover_item(),
                },
                found => {
                    let found = found.clone();
                    let mut diag = Diagnostic::error(
                        self.span(),
                        format!("expected `gawe` or `wujud`, found {found}"),
                    );
                    if starts_statement(&found) {
                        diag = diag.with_note(
                            "statements go inside a function, such as `gawe utama() { … }`",
                        );
                    }
                    if let TokenKind::Ident(word) = &found {
                        if let Some(note) = hints::note_for(word) {
                            diag = diag.with_note(note);
                        }
                    }
                    self.diagnostics.push(diag);
                    self.advance();
                    self.recover_item();
                }
            }
        }
        Program { items }
    }

    /// Skips to the next top-level `gawe` or `wujud`.
    fn recover_item(&mut self) {
        while !matches!(
            self.peek(),
            TokenKind::Gawe | TokenKind::Wujud | TokenKind::Eof
        ) {
            self.advance();
        }
    }

    fn function(&mut self) -> PResult<FunctionDecl> {
        let start = self.advance().span;
        let name = self.ident("after `gawe`")?;
        self.expect(&TokenKind::LParen, "after the function name")?;
        let mut params = Vec::new();
        while !self.at(&TokenKind::RParen) {
            let param = self.ident("for a parameter")?;
            if !self.at(&TokenKind::Colon) {
                let found = self.peek().clone();
                let diag = Diagnostic::error(
                    self.span(),
                    format!("expected `:` after the parameter name, found {found}"),
                )
                .with_note(format!(
                    "parameters need a type, such as `{}: wilangan`",
                    param.name
                ));
                self.diagnostics.push(diag);
                return Err(());
            }
            self.advance();
            let ty = self.type_expr()?;
            params.push(Param { name: param, ty });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "to close the parameter list")?;
        // `: wilangan`, as in TypeScript, or `-> wilangan`.
        let ret = if self.eat(&TokenKind::Colon) || self.eat(&TokenKind::Arrow) {
            Some(self.type_expr()?)
        } else {
            None
        };
        let body = self.block()?;
        Ok(FunctionDecl {
            name,
            params,
            ret,
            span: start.to(self.prev_span()),
            body,
        })
    }

    fn struct_decl(&mut self) -> PResult<StructDecl> {
        let start = self.advance().span;
        let name = self.ident("after `wujud`")?;
        let open = self.expect(&TokenKind::LBrace, "to open the field list")?;
        let mut fields = Vec::new();
        loop {
            while matches!(
                self.peek(),
                TokenKind::Comma | TokenKind::Semi | TokenKind::Newline
            ) {
                self.advance();
            }
            if self.eat(&TokenKind::RBrace) {
                break;
            }
            if self.at(&TokenKind::Eof) {
                self.error_at(open.span, "this `{` is never closed");
                return Err(());
            }
            let field = self.ident("for a field")?;
            self.expect(&TokenKind::Colon, "after the field name")?;
            let ty = self.type_expr()?;
            fields.push(FieldDecl { name: field, ty });
            if !matches!(
                self.peek(),
                TokenKind::Comma | TokenKind::Semi | TokenKind::Newline | TokenKind::RBrace
            ) {
                let found = self.peek().clone();
                self.error_here(format!("expected `,` between fields, found {found}"));
                return Err(());
            }
        }
        Ok(StructDecl {
            name,
            fields,
            span: start.to(self.prev_span()),
        })
    }

    fn type_expr(&mut self) -> PResult<TypeExpr> {
        self.enter()?;
        let result = self.type_expr_inner();
        self.leave();
        result
    }

    /// A type, with any number of `[]` after it: `wilangan[]` is an array of
    /// `wilangan`, as in TypeScript, and so is `[wilangan]`.
    fn type_expr_inner(&mut self) -> PResult<TypeExpr> {
        let start = self.span();
        let mut ty = self.base_type()?;
        while self.at(&TokenKind::LBracket) && self.peek_at(1) == &TokenKind::RBracket {
            self.advance();
            self.advance();
            ty = TypeExpr {
                kind: TypeExprKind::Array(Box::new(ty)),
                span: start.to(self.prev_span()),
            };
        }
        Ok(ty)
    }

    fn base_type(&mut self) -> PResult<TypeExpr> {
        let start = self.span();
        let kind = match self.peek().clone() {
            TokenKind::Wilangan => TypeExprKind::Wilangan,
            TokenKind::Pecahan => TypeExprKind::Pecahan,
            TokenKind::Logika => TypeExprKind::Logika,
            TokenKind::Ukara => TypeExprKind::Ukara,
            TokenKind::Suwung => TypeExprKind::Suwung,
            TokenKind::Ident(name) => TypeExprKind::Named(name),
            TokenKind::LBracket => {
                self.advance();
                let element = self.type_expr()?;
                self.expect(&TokenKind::RBracket, "to close the array type")?;
                return Ok(TypeExpr {
                    kind: TypeExprKind::Array(Box::new(element)),
                    span: start.to(self.prev_span()),
                });
            }
            found => {
                self.error_here(format!("expected a type, found {found}"));
                return Err(());
            }
        };
        self.advance();
        Ok(TypeExpr { kind, span: start })
    }

    // ---- Statements ----------------------------------------------------

    fn block(&mut self) -> PResult<Block> {
        if !self.at(&TokenKind::LBrace) {
            let found = self.peek().clone();
            let mut diag = Diagnostic::error(
                self.span(),
                format!("expected `{{` to open a block, found {found}"),
            );
            if found == TokenKind::Newline {
                diag = diag.with_note("put the `{` on the same line");
            }
            self.diagnostics.push(diag);
            return Err(());
        }
        let open = self.advance();
        let mut stmts = Vec::new();
        loop {
            self.skip_terminators();
            match self.peek() {
                TokenKind::RBrace => {
                    self.advance();
                    break;
                }
                TokenKind::Eof => {
                    self.error_at(open.span, "this `{` is never closed");
                    break;
                }
                _ => {
                    let before = self.pos;
                    match self.statement() {
                        Ok(stmt) => stmts.push(stmt),
                        Err(()) => {
                            self.synchronize();
                            if self.pos == before {
                                self.advance();
                            }
                        }
                    }
                }
            }
        }
        Ok(Block {
            stmts,
            span: open.span.to(self.prev_span()),
        })
    }

    /// Skips the rest of a statement that failed to parse.
    fn synchronize(&mut self) {
        loop {
            match self.peek() {
                TokenKind::Semi | TokenKind::Newline => {
                    self.advance();
                    return;
                }
                TokenKind::RBrace | TokenKind::Eof => return,
                kind if starts_statement(kind) => return,
                TokenKind::LBrace => self.skip_braces(),
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn skip_braces(&mut self) {
        let mut depth = 0usize;
        loop {
            match self.advance().kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                TokenKind::Eof => return,
                _ => {}
            }
        }
    }

    fn statement(&mut self) -> PResult<Stmt> {
        self.enter()?;
        let result = self.statement_inner();
        self.leave();
        result
    }

    fn statement_inner(&mut self) -> PResult<Stmt> {
        let start = self.span();
        match self.peek() {
            TokenKind::Ayo | TokenKind::Tetep => self.let_stmt(),
            TokenKind::Yen => self.if_stmt(),
            TokenKind::Sajrone => {
                self.advance();
                let cond = self.header_expr()?;
                let body = self.block()?;
                Ok(Stmt {
                    kind: StmtKind::While { cond, body },
                    span: start.to(self.prev_span()),
                })
            }
            TokenKind::Kanggo => self.for_stmt(),
            TokenKind::Bali => {
                self.advance();
                let value = if self.at_statement_end() {
                    None
                } else {
                    Some(self.expr()?)
                };
                let span = start.to(self.prev_span());
                self.terminator()?;
                Ok(Stmt {
                    kind: StmtKind::Return(value),
                    span,
                })
            }
            TokenKind::Mandheg | TokenKind::Terusna => {
                let kind = if self.advance().kind == TokenKind::Mandheg {
                    StmtKind::Break
                } else {
                    StmtKind::Continue
                };
                self.terminator()?;
                Ok(Stmt { kind, span: start })
            }
            TokenKind::LBrace => {
                let block = self.block()?;
                Ok(Stmt {
                    span: block.span,
                    kind: StmtKind::Block(block),
                })
            }
            TokenKind::Gawe | TokenKind::Wujud => {
                self.error_here("functions and structs are declared at the top level");
                Err(())
            }
            TokenKind::Liyane => {
                self.error_here("`liyane` without a `yen` before it");
                Err(())
            }
            // `let x = 1`, `return x`, `if (x) { … }`: a JavaScript keyword
            // where a Jawa one belongs. A variable that happens to be called
            // `let` is still fine: `let = 1`, `let.x`, `let[0]`.
            TokenKind::Ident(word)
                if hints::starts_javascript_statement(word)
                    && !matches!(
                        self.peek_at(1),
                        TokenKind::Eq
                            | TokenKind::PlusEq
                            | TokenKind::MinusEq
                            | TokenKind::StarEq
                            | TokenKind::SlashEq
                            | TokenKind::PercentEq
                            | TokenKind::PlusPlus
                            | TokenKind::MinusMinus
                            | TokenKind::Dot
                            | TokenKind::LBracket
                            | TokenKind::Semi
                            | TokenKind::Newline
                            | TokenKind::RBrace
                            | TokenKind::Eof
                    ) =>
            {
                let word = word.clone();
                let mut diag =
                    Diagnostic::error(self.span(), format!("`{word}` is not a Jawa keyword"));
                if let Some(note) = hints::note_for(&word) {
                    diag = diag.with_note(note);
                }
                self.diagnostics.push(diag);
                Err(())
            }
            _ => self.expr_or_assign(),
        }
    }

    fn at_statement_end(&self) -> bool {
        matches!(
            self.peek(),
            TokenKind::Semi | TokenKind::Newline | TokenKind::RBrace | TokenKind::Eof
        )
    }

    fn terminator(&mut self) -> PResult<()> {
        match self.peek() {
            TokenKind::Semi | TokenKind::Newline => {
                self.advance();
                Ok(())
            }
            TokenKind::RBrace | TokenKind::Eof => Ok(()),
            found => {
                let found = found.clone();
                self.error_here(format!(
                    "expected `;` or a line break after the statement, found {found}"
                ));
                Err(())
            }
        }
    }

    fn let_stmt(&mut self) -> PResult<Stmt> {
        let stmt = self.let_decl()?;
        self.terminator()?;
        Ok(stmt)
    }

    /// `ayo name: type = value`, without its terminator.
    fn let_decl(&mut self) -> PResult<Stmt> {
        let keyword = self.advance();
        let mutable = keyword.kind == TokenKind::Ayo;
        let name = self.ident(if mutable {
            "after `ayo`"
        } else {
            "after `tetep`"
        })?;
        let ty = if self.eat(&TokenKind::Colon) {
            Some(self.type_expr()?)
        } else {
            None
        };
        if !self.at(&TokenKind::Eq) {
            let found = self.peek().clone();
            let diag = Diagnostic::error(
                self.span(),
                format!("expected `=` after the variable name, found {found}"),
            )
            .with_note(format!(
                "every variable starts with a value, such as `ayo {} = 0`",
                name.name
            ));
            self.diagnostics.push(diag);
            return Err(());
        }
        self.advance();
        let init = self.expr()?;
        let span = keyword.span.to(self.prev_span());
        Ok(Stmt {
            kind: StmtKind::Let {
                name,
                mutable,
                ty,
                init,
            },
            span,
        })
    }

    fn if_stmt(&mut self) -> PResult<Stmt> {
        let start = self.advance().span;
        let cond = self.header_expr()?;
        let then_block = self.block()?;
        // Accept `liyane` on the line after the closing brace too.
        if self.at(&TokenKind::Newline) && self.peek_at(1) == &TokenKind::Liyane {
            self.advance();
        }
        let else_branch = if self.eat(&TokenKind::Liyane) {
            if self.at(&TokenKind::Yen) {
                Some(Box::new(ElseBranch::If(self.if_stmt()?)))
            } else {
                Some(Box::new(ElseBranch::Block(self.block()?)))
            }
        } else {
            None
        };
        Ok(Stmt {
            kind: StmtKind::If {
                cond,
                then_block,
                else_branch,
            },
            span: start.to(self.prev_span()),
        })
    }

    /// `kanggo x ing xs { … }`, `kanggo i ing 0..10 { … }`, and the
    /// JavaScript-style `kanggo (tetep x ing xs) { … }` and
    /// `kanggo (ayo i = 0; i < 10; i++) { … }`.
    fn for_stmt(&mut self) -> PResult<Stmt> {
        let start = self.advance().span;
        if self.at(&TokenKind::LParen) {
            return self.parenthesized_for(start);
        }
        let var = self.ident("after `kanggo`")?;
        let iter = self.for_in(&var, false)?;
        let body = self.block()?;
        Ok(Stmt {
            kind: StmtKind::For { var, iter, body },
            span: start.to(self.prev_span()),
        })
    }

    /// The part of a `kanggo` loop after its variable: `ing start..end` or
    /// `ing array`.
    fn for_in(&mut self, var: &Ident, in_parentheses: bool) -> PResult<ForIter> {
        if !self.at(&TokenKind::Ing) {
            let found = self.peek().clone();
            let diag = Diagnostic::error(
                self.span(),
                format!("expected `ing` after the loop variable, found {found}"),
            )
            .with_note(format!(
                "loops read `kanggo {} ing 0..10 {{ … }}`",
                var.name
            ));
            self.diagnostics.push(diag);
            return Err(());
        }
        self.advance();
        let part = |parser: &mut Self| {
            if in_parentheses {
                parser.nested_expr()
            } else {
                parser.header_expr()
            }
        };
        let first = part(self)?;
        Ok(
            if matches!(self.peek(), TokenKind::DotDot | TokenKind::DotDotEq) {
                let inclusive = self.advance().kind == TokenKind::DotDotEq;
                let end = part(self)?;
                ForIter::Range {
                    start: first,
                    end,
                    inclusive,
                }
            } else {
                ForIter::Each(first)
            },
        )
    }

    fn parenthesized_for(&mut self, start: Span) -> PResult<Stmt> {
        // `(x ing …` or `(tetep x ing …` loops over something; anything else
        // is `(init; condition; update)`.
        let over = matches!(
            (self.peek_at(1), self.peek_at(2)),
            (TokenKind::Ident(_), TokenKind::Ing)
        ) || (matches!(self.peek_at(1), TokenKind::Ayo | TokenKind::Tetep)
            && matches!(self.peek_at(2), TokenKind::Ident(_))
            && self.peek_at(3) == &TokenKind::Ing);
        self.advance();

        if over {
            if matches!(self.peek(), TokenKind::Ayo | TokenKind::Tetep) {
                self.advance();
            }
            let var = self.ident("for the loop variable")?;
            let iter = self.for_in(&var, true)?;
            self.expect(&TokenKind::RParen, "to close the loop header")?;
            let body = self.block()?;
            return Ok(Stmt {
                kind: StmtKind::For { var, iter, body },
                span: start.to(self.prev_span()),
            });
        }

        let init = if self.at(&TokenKind::Semi) {
            None
        } else if matches!(self.peek(), TokenKind::Ayo | TokenKind::Tetep) {
            Some(Box::new(self.let_decl()?))
        } else {
            Some(Box::new(self.simple_stmt()?))
        };
        self.expect_loop_semicolon("after the loop's first part")?;
        let cond = if self.at(&TokenKind::Semi) {
            None
        } else {
            Some(self.nested_expr()?)
        };
        self.expect_loop_semicolon("after the loop condition")?;
        let update = if self.at(&TokenKind::RParen) {
            None
        } else {
            Some(Box::new(self.simple_stmt()?))
        };
        self.expect(&TokenKind::RParen, "to close the loop header")?;
        let body = self.block()?;
        Ok(Stmt {
            kind: StmtKind::ForC {
                init,
                cond,
                update,
                body,
            },
            span: start.to(self.prev_span()),
        })
    }

    fn expect_loop_semicolon(&mut self, context: &str) -> PResult<()> {
        if self.eat(&TokenKind::Semi) {
            return Ok(());
        }
        let found = self.peek().clone();
        let diag = Diagnostic::error(
            self.span(),
            format!("expected `;` {context}, found {found}"),
        )
        .with_note("loops read `kanggo (ayo i = 0; i < 10; i++) { … }`");
        self.diagnostics.push(diag);
        Err(())
    }

    fn expr_or_assign(&mut self) -> PResult<Stmt> {
        let stmt = self.simple_stmt()?;
        self.terminator()?;
        Ok(stmt)
    }

    /// An expression, an assignment, or `i++`, `i--`, `++i`, `--i`, without
    /// a terminator.
    fn simple_stmt(&mut self) -> PResult<Stmt> {
        if matches!(self.peek(), TokenKind::PlusPlus | TokenKind::MinusMinus) {
            let op = self.advance();
            let target = self.expr()?;
            return self.increment(target, op);
        }
        let target = self.expr()?;
        if matches!(self.peek(), TokenKind::PlusPlus | TokenKind::MinusMinus) {
            let op = self.advance();
            return self.increment(target, op);
        }
        let op = match self.peek() {
            TokenKind::Eq => Some(None),
            TokenKind::PlusEq => Some(Some(BinaryOp::Add)),
            TokenKind::MinusEq => Some(Some(BinaryOp::Sub)),
            TokenKind::StarEq => Some(Some(BinaryOp::Mul)),
            TokenKind::SlashEq => Some(Some(BinaryOp::Div)),
            TokenKind::PercentEq => Some(Some(BinaryOp::Rem)),
            _ => None,
        };
        let Some(op) = op else {
            return Ok(Stmt {
                span: target.span,
                kind: StmtKind::Expr(target),
            });
        };
        self.advance();
        self.check_assignable(&target)?;
        let value = self.expr()?;
        Ok(Stmt {
            span: target.span.to(self.prev_span()),
            kind: StmtKind::Assign { target, op, value },
        })
    }

    /// `i++` is `i += 1`, and `i--` is `i -= 1`.
    fn increment(&mut self, target: Expr, op: Token) -> PResult<Stmt> {
        self.check_assignable(&target)?;
        let binary = if op.kind == TokenKind::PlusPlus {
            BinaryOp::Add
        } else {
            BinaryOp::Sub
        };
        let one = Expr {
            kind: ExprKind::Int(1),
            span: op.span,
        };
        Ok(Stmt {
            span: target.span.to(op.span),
            kind: StmtKind::Assign {
                target,
                op: Some(binary),
                value: one,
            },
        })
    }

    fn check_assignable(&mut self, target: &Expr) -> PResult<()> {
        if matches!(
            target.kind,
            ExprKind::Name(_) | ExprKind::Index { .. } | ExprKind::Field { .. }
        ) {
            return Ok(());
        }
        self.error_at(target.span, "cannot assign to this expression");
        Err(())
    }

    // ---- Expressions ---------------------------------------------------

    pub fn expr(&mut self) -> PResult<Expr> {
        self.enter()?;
        let result = self.or_expr();
        self.leave();
        result
    }

    /// An expression in a statement header, where `{` opens the body.
    fn header_expr(&mut self) -> PResult<Expr> {
        let saved = std::mem::replace(&mut self.no_struct, true);
        let result = self.expr();
        self.no_struct = saved;
        result
    }

    /// An expression inside brackets, where struct literals are allowed again.
    fn nested_expr(&mut self) -> PResult<Expr> {
        let saved = std::mem::replace(&mut self.no_struct, false);
        let result = self.expr();
        self.no_struct = saved;
        result
    }

    fn binary(lhs: Expr, op: BinaryOp, rhs: Expr) -> Expr {
        let span = lhs.span.to(rhs.span);
        Expr {
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            span,
        }
    }

    fn or_expr(&mut self) -> PResult<Expr> {
        let mut lhs = self.and_expr()?;
        while self.eat(&TokenKind::Utawa) {
            let rhs = self.and_expr()?;
            lhs = Self::binary(lhs, BinaryOp::Or, rhs);
        }
        Ok(lhs)
    }

    fn and_expr(&mut self) -> PResult<Expr> {
        let mut lhs = self.not_expr()?;
        while self.eat(&TokenKind::Lan) {
            let rhs = self.not_expr()?;
            lhs = Self::binary(lhs, BinaryOp::And, rhs);
        }
        Ok(lhs)
    }

    fn not_expr(&mut self) -> PResult<Expr> {
        if !self.at(&TokenKind::Ora) {
            return self.eq_expr();
        }
        let start = self.advance().span;
        self.enter()?;
        let operand = self.not_expr();
        self.leave();
        let operand = operand?;
        Ok(Expr {
            span: start.to(operand.span),
            kind: ExprKind::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
            },
        })
    }

    fn eq_expr(&mut self) -> PResult<Expr> {
        let mut lhs = self.rel_expr()?;
        loop {
            let op = match self.peek() {
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::NotEq => BinaryOp::Ne,
                _ => return Ok(lhs),
            };
            self.advance();
            let rhs = self.rel_expr()?;
            lhs = Self::binary(lhs, op, rhs);
        }
    }

    fn rel_expr(&mut self) -> PResult<Expr> {
        let mut lhs = self.add_expr()?;
        loop {
            let op = match self.peek() {
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::Le => BinaryOp::Le,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::Ge => BinaryOp::Ge,
                _ => return Ok(lhs),
            };
            self.advance();
            let rhs = self.add_expr()?;
            lhs = Self::binary(lhs, op, rhs);
        }
    }

    fn add_expr(&mut self) -> PResult<Expr> {
        let mut lhs = self.mul_expr()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => return Ok(lhs),
            };
            self.advance();
            let rhs = self.mul_expr()?;
            lhs = Self::binary(lhs, op, rhs);
        }
    }

    fn mul_expr(&mut self) -> PResult<Expr> {
        let mut lhs = self.unary()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Rem,
                _ => return Ok(lhs),
            };
            self.advance();
            let rhs = self.unary()?;
            lhs = Self::binary(lhs, op, rhs);
        }
    }

    /// `-x`, and JavaScript's `!x`, which binds as tightly as `-` does (so
    /// `!a == b` means `(!a) == b`, as in JavaScript).
    fn unary(&mut self) -> PResult<Expr> {
        let op = match self.peek() {
            TokenKind::Minus => UnaryOp::Neg,
            TokenKind::Bang => UnaryOp::Not,
            _ => return self.postfix(),
        };
        let start = self.advance().span;
        self.enter()?;
        let operand = self.unary();
        self.leave();
        let operand = operand?;
        Ok(Expr {
            span: start.to(operand.span),
            kind: ExprKind::Unary {
                op,
                operand: Box::new(operand),
            },
        })
    }

    fn postfix(&mut self) -> PResult<Expr> {
        let mut expr = self.primary()?;
        loop {
            match self.peek() {
                TokenKind::LParen => {
                    let ExprKind::Name(name) = &expr.kind else {
                        self.error_here("only functions can be called, by name");
                        return Err(());
                    };
                    let callee = Ident {
                        name: name.clone(),
                        span: expr.span,
                    };
                    self.advance();
                    let args = self.comma_list(&TokenKind::RParen, "to close the argument list")?;
                    expr = Expr {
                        span: expr.span.to(self.prev_span()),
                        kind: ExprKind::Call { callee, args },
                    };
                }
                TokenKind::LBracket => {
                    self.advance();
                    let index = self.nested_expr()?;
                    self.expect(&TokenKind::RBracket, "to close the index")?;
                    expr = Expr {
                        span: expr.span.to(self.prev_span()),
                        kind: ExprKind::Index {
                            base: Box::new(expr),
                            index: Box::new(index),
                        },
                    };
                }
                TokenKind::Dot => {
                    self.advance();
                    let field = self.ident("after `.`")?;
                    if self.at(&TokenKind::LParen) {
                        let note = match &expr.kind {
                            ExprKind::Name(base) if base == "console" => {
                                "print with `tulis(…)`".to_string()
                            }
                            ExprKind::Name(base) => {
                                format!(
                                    "call it as a function instead: `{}({base}, …)`",
                                    field.name
                                )
                            }
                            _ => format!("call it as a function instead: `{}(…)`", field.name),
                        };
                        let diag = Diagnostic::error(field.span, "methods are not supported")
                            .with_note(note);
                        self.diagnostics.push(diag);
                        return Err(());
                    }
                    expr = Expr {
                        span: expr.span.to(field.span),
                        kind: ExprKind::Field {
                            base: Box::new(expr),
                            field,
                        },
                    };
                }
                _ => return Ok(expr),
            }
        }
    }

    /// Parses `item, item, …` up to and including `close`. The opening token
    /// has already been consumed.
    fn comma_list(&mut self, close: &TokenKind, context: &str) -> PResult<Vec<Expr>> {
        let mut items = Vec::new();
        loop {
            if self.eat(close) {
                return Ok(items);
            }
            items.push(self.nested_expr()?);
            if !self.eat(&TokenKind::Comma) {
                self.expect(close, context)?;
                return Ok(items);
            }
        }
    }

    fn primary(&mut self) -> PResult<Expr> {
        let span = self.span();
        let kind = match self.peek().clone() {
            TokenKind::Int(value) => ExprKind::Int(value),
            TokenKind::Float(value) => ExprKind::Float(value),
            TokenKind::Str(value) => ExprKind::Str(value),
            TokenKind::Template(pieces) => {
                self.advance();
                let mut parts = Vec::new();
                for piece in pieces {
                    parts.push(match piece {
                        TemplatePiece::Text(text) => TemplatePart::Text(text),
                        TemplatePiece::Code { source, offset } => {
                            TemplatePart::Expr(self.template_expr(&source, offset)?)
                        }
                    });
                }
                return Ok(Expr {
                    kind: ExprKind::Template(parts),
                    span,
                });
            }
            TokenKind::Bener => ExprKind::Bool(true),
            TokenKind::Salah => ExprKind::Bool(false),
            TokenKind::Kosong => ExprKind::Null,
            TokenKind::Ident(name) => {
                self.advance();
                if self.at(&TokenKind::LBrace) && !self.no_struct {
                    return self.struct_lit(Ident { name, span });
                }
                return Ok(Expr {
                    kind: ExprKind::Name(name),
                    span,
                });
            }
            TokenKind::LParen => {
                self.advance();
                let inner = self.nested_expr()?;
                self.expect(&TokenKind::RParen, "to close the parenthesis")?;
                return Ok(inner);
            }
            TokenKind::LBracket => {
                self.advance();
                let elements = self.comma_list(&TokenKind::RBracket, "to close the array")?;
                return Ok(Expr {
                    kind: ExprKind::Array(elements),
                    span: span.to(self.prev_span()),
                });
            }
            TokenKind::Wilangan | TokenKind::Pecahan | TokenKind::Logika | TokenKind::Ukara
                if self.peek_at(1) == &TokenKind::LParen =>
            {
                return self.conversion();
            }
            found => {
                let mut diag =
                    Diagnostic::error(span, format!("expected an expression, found {found}"));
                match found {
                    TokenKind::Ora => {
                        diag = diag.with_note("wrap it in parentheses: `(ora …)`");
                    }
                    TokenKind::Wilangan
                    | TokenKind::Pecahan
                    | TokenKind::Logika
                    | TokenKind::Ukara => {
                        diag = diag.with_note(format!(
                            "to convert a value, call the type: `{}(x)`",
                            found.to_string().trim_matches('`')
                        ));
                    }
                    _ => {}
                }
                self.diagnostics.push(diag);
                return Err(());
            }
        };
        self.advance();
        Ok(Expr { kind, span })
    }

    /// Parses the expression inside a template string's `${…}`, keeping the
    /// positions of its tokens in the file.
    fn template_expr(&mut self, source: &str, offset: usize) -> PResult<Expr> {
        if source.trim().is_empty() {
            self.error_at(
                Span::new(offset, offset + source.len()),
                "empty `${}` in a template string",
            );
            return Err(());
        }
        let lexed = lexer::lex_at(source, offset);
        let mut inner = Parser {
            tokens: lexed.tokens,
            pos: 0,
            diagnostics: lexed.diagnostics,
            no_struct: false,
            depth: self.depth,
        };
        let result = inner.expr();
        inner.skip_terminators();
        if result.is_ok() && !inner.at(&TokenKind::Eof) {
            let found = inner.peek().clone();
            inner.error_here(format!(
                "expected `}}` after the expression in `${{…}}`, found {found}"
            ));
        }
        let failed = inner.diagnostics.iter().any(|d| d.is_error());
        self.diagnostics.append(&mut inner.diagnostics);
        match result {
            Ok(expr) if !failed => Ok(expr),
            _ => Err(()),
        }
    }

    fn conversion(&mut self) -> PResult<Expr> {
        let to = self.type_expr()?;
        self.advance(); // `(`
        let args = self.comma_list(&TokenKind::RParen, "to close the conversion")?;
        let span = to.span.to(self.prev_span());
        let mut args = args.into_iter();
        match (args.next(), args.next()) {
            (Some(arg), None) => Ok(Expr {
                kind: ExprKind::Convert {
                    to,
                    arg: Box::new(arg),
                },
                span,
            }),
            _ => {
                self.error_at(span, "a conversion takes exactly one value");
                Err(())
            }
        }
    }

    fn struct_lit(&mut self, name: Ident) -> PResult<Expr> {
        let open = self.advance();
        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&TokenKind::RBrace) {
                break;
            }
            if self.at(&TokenKind::Eof) {
                self.error_at(open.span, "this `{` is never closed");
                return Err(());
            }
            let field = self.ident("for a field")?;
            self.expect(&TokenKind::Colon, "after the field name")?;
            let value = self.expr()?;
            fields.push((field, value));
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) {
                self.expect(&TokenKind::RBrace, "to close the struct literal")?;
                break;
            }
        }
        Ok(Expr {
            span: name.span.to(self.prev_span()),
            kind: ExprKind::StructLit { name, fields },
        })
    }
}

fn starts_statement(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Ayo
            | TokenKind::Tetep
            | TokenKind::Yen
            | TokenKind::Sajrone
            | TokenKind::Kanggo
            | TokenKind::Bali
            | TokenKind::Mandheg
            | TokenKind::Terusna
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(text: &str) -> Program {
        let parsed = parse(text);
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected diagnostics: {:#?}",
            parsed.diagnostics
        );
        parsed.program
    }

    fn errors(text: &str) -> Vec<String> {
        parse(text)
            .diagnostics
            .into_iter()
            .map(|d| d.message)
            .collect()
    }

    /// Renders an expression as an s-expression, for compact assertions.
    fn sexpr(expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Int(v) => v.to_string(),
            ExprKind::Float(v) => format!("{v:?}"),
            ExprKind::Str(s) => format!("{s:?}"),
            ExprKind::Template(parts) => {
                let parts: Vec<_> = parts
                    .iter()
                    .map(|part| match part {
                        TemplatePart::Text(text) => format!("{text:?}"),
                        TemplatePart::Expr(e) => sexpr(e),
                    })
                    .collect();
                format!("(template {})", parts.join(" "))
            }
            ExprKind::Bool(b) => if *b { "bener" } else { "salah" }.to_string(),
            ExprKind::Null => "kosong".to_string(),
            ExprKind::Name(n) => n.clone(),
            ExprKind::Unary { op, operand } => format!("({op} {})", sexpr(operand)),
            ExprKind::Binary { op, lhs, rhs } => {
                format!("({op} {} {})", sexpr(lhs), sexpr(rhs))
            }
            ExprKind::Call { callee, args } => {
                let args: Vec<_> = args.iter().map(sexpr).collect();
                format!("(call {} {})", callee.name, args.join(" "))
            }
            ExprKind::Convert { to, arg } => format!("(as {:?} {})", to.kind, sexpr(arg)),
            ExprKind::Array(items) => {
                let items: Vec<_> = items.iter().map(sexpr).collect();
                format!("[{}]", items.join(" "))
            }
            ExprKind::StructLit { name, fields } => {
                let fields: Vec<_> = fields
                    .iter()
                    .map(|(f, v)| format!("{}={}", f.name, sexpr(v)))
                    .collect();
                format!("({} {})", name.name, fields.join(" "))
            }
            ExprKind::Index { base, index } => format!("(index {} {})", sexpr(base), sexpr(index)),
            ExprKind::Field { base, field } => format!("(. {} {})", sexpr(base), field.name),
        }
    }

    fn body(program: &Program) -> &[Stmt] {
        match &program.items[0] {
            Item::Function(f) => &f.body.stmts,
            Item::Struct(_) => panic!("expected a function"),
        }
    }

    fn expr_of(text: &str) -> String {
        let program = parse_ok(&format!("gawe utama() {{ {text} }}"));
        match &body(&program)[0].kind {
            StmtKind::Expr(e) => sexpr(e),
            other => panic!("expected an expression statement, got {other:?}"),
        }
    }

    #[test]
    fn arithmetic_precedence() {
        assert_eq!(expr_of("1 + 2 * 3 - 4"), "(- (+ 1 (* 2 3)) 4)");
        assert_eq!(expr_of("(1 + 2) * 3"), "(* (+ 1 2) 3)");
        assert_eq!(expr_of("-a % b"), "(% (- a) b)");
    }

    #[test]
    fn logic_reads_like_python() {
        assert_eq!(expr_of("ora a lan b utawa c"), "(utawa (lan (ora a) b) c)");
        assert_eq!(expr_of("ora x == 0"), "(ora (== x 0))");
        assert_eq!(expr_of("a < b == c >= d"), "(== (< a b) (>= c d))");
    }

    #[test]
    fn postfix_chains() {
        assert_eq!(
            expr_of("titik[i].x + f(a, [1, 2])"),
            "(+ (. (index titik i) x) (call f a [1 2]))"
        );
        assert_eq!(expr_of("ukara(42)"), "(as Ukara 42)");
    }

    #[test]
    fn struct_literal_except_in_headers() {
        assert_eq!(expr_of("Titik { x: 1, y: 2 }"), "(Titik x=1 y=2)");
        let program = parse_ok("gawe utama() { yen apik { tulis(1) } }");
        assert!(matches!(body(&program)[0].kind, StmtKind::If { .. }));
    }

    #[test]
    fn multi_line_struct_literal() {
        let program = parse_ok("gawe utama() {\n  ayo p = Titik {\n    x: 1,\n    y: 2\n  }\n}\n");
        let StmtKind::Let { init, .. } = &body(&program)[0].kind else {
            panic!("expected a let");
        };
        assert_eq!(sexpr(init), "(Titik x=1 y=2)");
    }

    #[test]
    fn statements() {
        let program = parse_ok(
            "gawe utama() {
                ayo x: wilangan = 1
                tetep y = x
                x += 2; x = x * 3
                sajrone x < 10 { x = x + 1 }
                kanggo i ing 0..10 { terusna }
                kanggo j ing 1..=3 { mandheg }
                kanggo jeneng ing jeneng_jeneng { tulis(jeneng) }
                bali
            }",
        );
        let kinds: Vec<_> = body(&program)
            .iter()
            .map(|s| match &s.kind {
                StmtKind::Let { mutable: true, .. } => "ayo",
                StmtKind::Let { mutable: false, .. } => "tetep",
                StmtKind::Assign { op: Some(_), .. } => "op=",
                StmtKind::Assign { op: None, .. } => "=",
                StmtKind::While { .. } => "sajrone",
                StmtKind::For {
                    iter:
                        ForIter::Range {
                            inclusive: false, ..
                        },
                    ..
                } => "kanggo..",
                StmtKind::For {
                    iter:
                        ForIter::Range {
                            inclusive: true, ..
                        },
                    ..
                } => "kanggo..=",
                StmtKind::For {
                    iter: ForIter::Each(_),
                    ..
                } => "kanggo-ing",
                StmtKind::Return(None) => "bali",
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(
            kinds,
            [
                "ayo",
                "tetep",
                "op=",
                "=",
                "sajrone",
                "kanggo..",
                "kanggo..=",
                "kanggo-ing",
                "bali"
            ]
        );
    }

    #[test]
    fn else_if_chain_with_liyane_on_next_line() {
        let program = parse_ok(
            "gawe utama() {
                yen a { tulis(1) }
                liyane yen b { tulis(2) }
                liyane { tulis(3) }
            }",
        );
        let StmtKind::If {
            else_branch: Some(else_branch),
            ..
        } = &body(&program)[0].kind
        else {
            panic!("expected yen");
        };
        let ElseBranch::If(inner) = else_branch.as_ref() else {
            panic!("expected liyane yen");
        };
        assert!(matches!(
            &inner.kind,
            StmtKind::If {
                else_branch: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn functions_and_structs() {
        let program = parse_ok(
            "wujud Titik {
                x: pecahan
                y: pecahan
            }
            wujud Simpul { regane: wilangan, sabanjure: Simpul }
            gawe dawa(t: Titik, larik: [[wilangan]]) -> pecahan { bali t.x }",
        );
        assert_eq!(program.items.len(), 3);
        let Item::Struct(titik) = &program.items[0] else {
            panic!()
        };
        assert_eq!(titik.fields.len(), 2);
        let Item::Function(dawa) = &program.items[2] else {
            panic!()
        };
        assert_eq!(dawa.params.len(), 2);
        assert!(matches!(
            &dawa.params[1].ty.kind,
            TypeExprKind::Array(inner) if matches!(inner.kind, TypeExprKind::Array(_))
        ));
        assert_eq!(
            dawa.ret.as_ref().map(|t| &t.kind),
            Some(&TypeExprKind::Pecahan)
        );
    }

    #[test]
    fn reports_missing_paren_and_keeps_going() {
        let messages = errors(
            "gawe utama() {
                tulis(1
                ayo x = )
                tulis(2)
            }",
        );
        assert_eq!(
            messages,
            [
                "expected `)` to close the argument list, found end of line",
                "expected an expression, found `)`",
            ]
        );
    }

    #[test]
    fn statements_outside_functions() {
        let parsed = parse("tulis(1)\n");
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(
            parsed.diagnostics[0].message,
            "expected `gawe` or `wujud`, found identifier `tulis`"
        );
        let parsed = parse("ayo x = 1\n");
        assert_eq!(
            parsed.diagnostics[0].notes,
            ["statements go inside a function, such as `gawe utama() { … }`"]
        );
    }

    #[test]
    fn rejects_bad_assignment_targets() {
        assert_eq!(
            errors("gawe utama() { f() = 1 }"),
            ["cannot assign to this expression"]
        );
    }

    #[test]
    fn rejects_methods_with_a_hint() {
        let parsed = parse("gawe utama() { larik.tambah(1) }");
        assert_eq!(parsed.diagnostics[0].message, "methods are not supported");
        assert_eq!(
            parsed.diagnostics[0].notes,
            ["call it as a function instead: `tambah(larik, …)`"]
        );
        let parsed = parse("gawe utama() { [1].tambah(2) }");
        assert_eq!(
            parsed.diagnostics[0].notes,
            ["call it as a function instead: `tambah(…)`"]
        );
    }

    #[test]
    fn deep_nesting_is_an_error_not_a_crash() {
        // Debug builds use big stack frames; give the parser the room the
        // compiler driver gives it.
        let messages = std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                let open = "(".repeat(5000);
                let close = ")".repeat(5000);
                errors(&format!("gawe utama() {{ tulis({open}1{close}) }}"))
            })
            .unwrap()
            .join()
            .unwrap();
        assert!(messages.contains(&"this code nests too deeply".to_string()));
    }

    #[test]
    fn brace_on_next_line_gets_a_hint() {
        let parsed = parse("gawe utama()\n{\n}\n");
        assert_eq!(
            parsed.diagnostics[0].notes,
            ["put the `{` on the same line"]
        );
    }

    // ---- JavaScript and TypeScript style -------------------------------

    #[test]
    fn typescript_style_types() {
        let program = parse_ok("gawe f(larik: wilangan[][], n: wilangan): ukara[] { bali [] }");
        let Item::Function(f) = &program.items[0] else {
            panic!()
        };
        let TypeExprKind::Array(inner) = &f.params[0].ty.kind else {
            panic!("expected an array type")
        };
        assert!(matches!(&inner.kind, TypeExprKind::Array(e) if e.kind == TypeExprKind::Wilangan));
        assert!(matches!(
            &f.ret.as_ref().unwrap().kind,
            TypeExprKind::Array(e) if e.kind == TypeExprKind::Ukara
        ));
    }

    #[test]
    fn javascript_operators() {
        assert_eq!(
            expr_of("a && !b || c === d"),
            "(utawa (lan a (ora b)) (== c d))"
        );
        // `!` binds tightly, as in JavaScript; `ora` loosely, as in Python.
        assert_eq!(expr_of("!a == b"), "(== (ora a) b)");
        assert_eq!(expr_of("ora a == b"), "(ora (== a b))");
        assert_eq!(expr_of("a !== b"), "(!= a b)");
    }

    #[test]
    fn template_strings() {
        assert_eq!(
            expr_of("`a ${x + 1} b ${f(y)}`"),
            r#"(template "a " (+ x 1) " b " (call f y))"#
        );
        assert_eq!(expr_of("`plain`"), r#"(template "plain")"#);
    }

    #[test]
    fn template_errors_point_inside_the_string() {
        let parsed = parse("gawe utama() { tulis(`a ${}`) }");
        assert_eq!(
            parsed.diagnostics[0].message,
            "empty `${}` in a template string"
        );
        let parsed = parse("gawe utama() { tulis(`a ${1 2}`) }");
        assert_eq!(
            parsed.diagnostics[0].message,
            "expected `}` after the expression in `${…}`, found number `2`"
        );
        // The error points at the `2`, inside the string.
        assert_eq!(parsed.diagnostics[0].span, Span::new(28, 29));
    }

    #[test]
    fn increment_and_decrement_statements() {
        let program = parse_ok("gawe utama() { i++; --j; larik[0]++ }");
        let ops: Vec<_> = body(&program)
            .iter()
            .map(|stmt| match &stmt.kind {
                StmtKind::Assign {
                    op: Some(op),
                    value,
                    ..
                } if matches!(value.kind, ExprKind::Int(1)) => *op,
                other => panic!("expected an increment, got {other:?}"),
            })
            .collect();
        assert_eq!(ops, [BinaryOp::Add, BinaryOp::Sub, BinaryOp::Add]);
    }

    #[test]
    fn increment_inside_an_expression_has_a_hint() {
        let parsed = parse("gawe utama() { tulis(i++) }");
        assert_eq!(
            parsed.diagnostics[0].message,
            "expected `)` to close the argument list, found `++`"
        );
        assert_eq!(
            parsed.diagnostics[0].notes,
            ["`++` only works as a statement of its own, such as `i++`"]
        );
    }

    #[test]
    fn javascript_style_loops() {
        let program = parse_ok(
            "gawe utama() {
                kanggo (ayo i = 0; i < 10; i++) { }
                kanggo (;;) { mandheg }
                kanggo (tetep x ing larik) { }
                kanggo (i ing 0..3) { }
            }",
        );
        let stmts = body(&program);
        let StmtKind::ForC {
            init: Some(init),
            cond: Some(cond),
            update: Some(update),
            ..
        } = &stmts[0].kind
        else {
            panic!("expected a C-style loop, got {:?}", stmts[0].kind)
        };
        assert!(matches!(init.kind, StmtKind::Let { mutable: true, .. }));
        assert_eq!(sexpr(cond), "(< i 10)");
        assert!(matches!(update.kind, StmtKind::Assign { .. }));
        assert!(matches!(
            stmts[1].kind,
            StmtKind::ForC {
                init: None,
                cond: None,
                update: None,
                ..
            }
        ));
        assert!(matches!(
            &stmts[2].kind,
            StmtKind::For { var, iter: ForIter::Each(_), .. } if var.name == "x"
        ));
        assert!(matches!(
            stmts[3].kind,
            StmtKind::For {
                iter: ForIter::Range { .. },
                ..
            }
        ));
    }

    #[test]
    fn loop_header_mistakes() {
        let parsed = parse("gawe utama() { kanggo (ayo i = 0, i < 3; i++) {} }");
        assert_eq!(
            parsed.diagnostics[0].message,
            "expected `;` after the loop's first part, found `,`"
        );
        assert_eq!(
            parsed.diagnostics[0].notes,
            ["loops read `kanggo (ayo i = 0; i < 10; i++) { … }`"]
        );
    }
}
