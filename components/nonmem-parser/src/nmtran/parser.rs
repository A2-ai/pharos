use std::fmt::Display;
use std::ops::Range;

use super::lexer::{NmtranSpannedToken, NmtranToken};
use crate::cst::{NmtranChild, NmtranCodeBlock, NmtranNode, NmtranNodeKind};
use crate::errors::{Diagnostic, ParseErrorKind};

/// Blocks we can expect to close something
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockTerm {
    Else,
    ElseIf,
    EndIf,
    EndDo,
}

impl Display for BlockTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            BlockTerm::Else => "ELSE",
            BlockTerm::ElseIf => "ELSEIF",
            BlockTerm::EndIf => "ENDIF",
            BlockTerm::EndDo => "ENDDO",
        };
        write!(f, "{s}")
    }
}

const IF_TERMINATORS: &[BlockTerm] = &[BlockTerm::Else, BlockTerm::ElseIf, BlockTerm::EndIf];
const ELSE_TERMINATORS: &[BlockTerm] = &[BlockTerm::EndIf];
const DO_TERMINATORS: &[BlockTerm] = &[BlockTerm::EndDo];

const PREFIX_NOT_BP: u8 = 5;
const PREFIX_SIGN_BP: u8 = 11;

fn infix_bp(tok: &NmtranToken) -> Option<(u8, u8)> {
    match tok {
        NmtranToken::DotOr => Some((1, 2)),
        NmtranToken::DotAnd => Some((3, 4)),
        NmtranToken::DotEq
        | NmtranToken::DotNe
        | NmtranToken::DotLt
        | NmtranToken::DotLe
        | NmtranToken::DotGt
        | NmtranToken::DotGe
        | NmtranToken::EqEq
        | NmtranToken::SlashEq
        | NmtranToken::Lt
        | NmtranToken::LtEq
        | NmtranToken::Gt
        | NmtranToken::GtEq => Some((5, 6)),
        NmtranToken::Plus | NmtranToken::Minus => Some((7, 8)),
        NmtranToken::Star | NmtranToken::Slash => Some((9, 10)),
        NmtranToken::StarStar => Some((12, 11)), // right-associative
        _ => None,
    }
}

pub(crate) struct NmtranParser {
    idx: usize,
    tokens: Vec<NmtranSpannedToken>,
}

impl NmtranParser {
    pub(crate) fn parse(tokens: Vec<NmtranSpannedToken>) -> Result<NmtranCodeBlock, Diagnostic> {
        let mut parser = Self { tokens, idx: 0 };
        let children = parser.parse_stmts_until(&[])?;
        parser.expect_eof()?;
        Ok(NmtranCodeBlock {
            tokens: parser.tokens,
            children,
        })
    }

    fn eof_span(&self) -> Range<usize> {
        self.tokens
            .last()
            .map(|t| t.span.end..t.span.end)
            .unwrap_or(0..0)
    }

    fn current_span(&self) -> Range<usize> {
        self.tokens
            .get(self.idx)
            .map(|t| t.span.clone())
            .unwrap_or_else(|| self.eof_span())
    }

    fn peek(&self) -> Option<&NmtranToken> {
        self.tokens.get(self.idx).map(|t| &t.token)
    }

    // Used for statement boundaries and block terminators: skip spaces/comments but
    // preserve physical newlines because they can end headers and statements.
    fn skip_ws_comments_from(&self, mut i: usize) -> usize {
        while i < self.tokens.len() && self.tokens[i].token.is_trivia() {
            i += 1;
        }
        i
    }

    // Used for expression-local lookahead: treat `&\n` as transparent so a continued
    // line behaves like a single logical line.
    fn skip_inline_trivia_from(&self, mut i: usize) -> usize {
        loop {
            while i < self.tokens.len() && self.tokens[i].token.is_trivia() {
                i += 1;
            }

            if i < self.tokens.len() && self.tokens[i].token == NmtranToken::Ampersand {
                i += 1;
                if i < self.tokens.len() && self.tokens[i].token == NmtranToken::Newline {
                    i += 1;
                }
                continue;
            }

            return i;
        }
    }

    fn peek_non_trivia(&self) -> Option<(usize, &NmtranSpannedToken)> {
        let i = self.skip_ws_comments_from(self.idx);
        if i < self.tokens.len() {
            Some((i, &self.tokens[i]))
        } else {
            None
        }
    }

    fn eat(&mut self, children: &mut Vec<NmtranChild>) {
        children.push(NmtranChild::Token(self.idx));
        self.idx += 1;
    }

    fn expect(
        &mut self,
        tok: NmtranToken,
        children: &mut Vec<NmtranChild>,
    ) -> Result<(), Diagnostic> {
        match self.peek() {
            Some(t) if *t == tok => {
                self.eat(children);
                Ok(())
            }
            Some(t) => Err(Diagnostic::parse(
                ParseErrorKind::Message(format!("expected {tok}, found {t}")),
                self.current_span(),
            )),
            None => Err(Diagnostic::parse(
                ParseErrorKind::Message(format!("expected {tok}, found end of input")),
                self.eof_span(),
            )),
        }
    }

    fn error_here(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::parse(ParseErrorKind::Message(message.into()), self.current_span())
    }

    /// Skip whitespace/comments and treat `&\n` as line continuation.
    /// When `eat_newlines` is true, consume physical newlines too.
    fn collect_trivia(&mut self, children: &mut Vec<NmtranChild>, eat_newlines: bool) {
        while let Some(tok) = self.peek() {
            match tok {
                NmtranToken::Whitespace | NmtranToken::Comment => self.eat(children),
                NmtranToken::Newline if eat_newlines => self.eat(children),
                NmtranToken::Ampersand => {
                    self.eat(children);
                    if matches!(self.peek(), Some(NmtranToken::Newline)) {
                        self.eat(children);
                    }
                }
                _ => break,
            }
        }
    }

    fn peek_is(&self, tok: NmtranToken) -> bool {
        matches!(self.peek_non_trivia(), Some((_, t)) if t.token == tok)
    }

    /// Is this identifier the start of `name = ...` or `name(...) = ...`?
    fn is_assignment_start(&self, idx: usize) -> bool {
        let mut i = self.skip_inline_trivia_from(idx + 1);

        match self.tokens.get(i).map(|t| &t.token) {
            None => false,
            Some(NmtranToken::Equals) => true,
            Some(NmtranToken::LeftParen) => {
                i += 1;
                let mut depth = 1;
                while i < self.tokens.len() && depth > 0 {
                    match self.tokens[i].token {
                        NmtranToken::LeftParen => depth += 1,
                        NmtranToken::RightParen => depth -= 1,
                        _ => {}
                    }
                    i += 1;
                }

                if depth != 0 {
                    return false;
                }

                i = self.skip_inline_trivia_from(i);
                matches!(self.tokens.get(i), Some(t) if t.token == NmtranToken::Equals)
            }
            Some(_) => false,
        }
    }

    fn peek_block_term(&self) -> Option<BlockTerm> {
        let (i, tok) = self.peek_non_trivia()?;
        match tok.token {
            NmtranToken::ElseKw => Some(BlockTerm::Else),
            NmtranToken::ElseIfKw => Some(BlockTerm::ElseIf),
            NmtranToken::EndIfKw => Some(BlockTerm::EndIf),
            NmtranToken::EndDoKw => Some(BlockTerm::EndDo),
            NmtranToken::EndKw => {
                let j = self.skip_ws_comments_from(i + 1);
                match self.tokens.get(j) {
                    Some(t) if t.token == NmtranToken::IfKw => Some(BlockTerm::EndIf),
                    Some(t) if t.token == NmtranToken::DoKw => Some(BlockTerm::EndDo),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn expect_eof(&self) -> Result<(), Diagnostic> {
        if self.idx == self.tokens.len() {
            Ok(())
        } else if let Some(term) = self.peek_block_term() {
            Err(self.error_here(format!("unexpected {term}")))
        } else {
            Err(self.error_here("unexpected trailing input"))
        }
    }

    fn at_stmt_end(&self) -> bool {
        self.peek_non_trivia()
            .is_none_or(|(_, t)| t.token == NmtranToken::Newline)
    }

    fn eat_to_eol(&mut self, children: &mut Vec<NmtranChild>) {
        while !self.at_stmt_end() {
            self.collect_trivia(children, false);
            if self.at_stmt_end() {
                break;
            }
            self.eat(children);
        }
    }

    fn parse_stmts_until(
        &mut self,
        terminators: &[BlockTerm],
    ) -> Result<Vec<NmtranChild>, Diagnostic> {
        let mut children = Vec::new();

        loop {
            self.collect_trivia(&mut children, true);

            if self.peek().is_none() {
                break;
            }

            if let Some(term) = self.peek_block_term() {
                if terminators.contains(&term) {
                    break;
                }
                return Err(self.error_here(format!("unexpected {term}")));
            }

            children.push(NmtranChild::Node(self.parse_stmt()?));
        }

        Ok(children)
    }

    fn parse_stmt(&mut self) -> Result<NmtranNode, Diagnostic> {
        let (idx, tok) = self.peek_non_trivia().ok_or_else(|| {
            Diagnostic::parse(
                ParseErrorKind::Message("unexpected end of input".into()),
                self.eof_span(),
            )
        })?;

        match tok.token {
            NmtranToken::IfKw | NmtranToken::ElseIfKw => return self.parse_if(),
            NmtranToken::CallKw => {
                return Ok(self.parse_keyword_stmt(NmtranNodeKind::Call));
            }
            NmtranToken::ExitKw => {
                return Ok(self.parse_keyword_stmt(NmtranNodeKind::Exit));
            }
            NmtranToken::DoKw => {
                let next = self
                    .peek_non_trivia()
                    .map(|(i, _)| self.skip_inline_trivia_from(i + 1));
                if matches!(next.and_then(|i| self.tokens.get(i)), Some(t) if t.token == NmtranToken::WhileKw)
                {
                    return self.parse_do_while();
                }
            }
            NmtranToken::DoWhileKw => {
                return self.parse_do_while();
            }
            NmtranToken::Ident if self.is_assignment_start(idx) => {
                return self.parse_assignment();
            }
            _ => {}
        }

        Ok(self.parse_unknown_stmt())
    }

    fn parse_unknown_stmt(&mut self) -> NmtranNode {
        let mut children = vec![];
        self.eat_to_eol(&mut children);
        NmtranNode {
            kind: NmtranNodeKind::Unknown,
            children,
        }
    }

    fn parse_keyword_stmt(&mut self, kind: NmtranNodeKind) -> NmtranNode {
        let mut children = vec![];
        self.collect_trivia(&mut children, false);
        self.eat(&mut children);
        self.eat_to_eol(&mut children);
        NmtranNode { kind, children }
    }

    fn parse_required_paren_expr(
        &mut self,
        children: &mut Vec<NmtranChild>,
        empty_message: &str,
    ) -> Result<(), Diagnostic> {
        self.expect(NmtranToken::LeftParen, children)?;
        self.collect_trivia(children, false);

        if matches!(self.peek(), Some(NmtranToken::RightParen)) {
            return Err(self.error_here(empty_message));
        }

        children.push(self.parse_expr(0)?);
        self.collect_trivia(children, false);
        self.expect(NmtranToken::RightParen, children)?;
        self.collect_trivia(children, false);
        Ok(())
    }

    fn parse_assignment(&mut self) -> Result<NmtranNode, Diagnostic> {
        let mut children = vec![];

        self.collect_trivia(&mut children, false);
        self.eat(&mut children); // ident
        self.collect_trivia(&mut children, false);

        if matches!(self.peek(), Some(NmtranToken::LeftParen)) {
            self.eat(&mut children); // (
            self.parse_arg_list(&mut children)?;
            self.collect_trivia(&mut children, false);
            self.expect(NmtranToken::RightParen, &mut children)?;
        }

        self.collect_trivia(&mut children, false);
        self.expect(NmtranToken::Equals, &mut children)?;
        self.collect_trivia(&mut children, false);

        if self.at_stmt_end() {
            return Err(self.error_here("missing assignment right-hand side"));
        }

        children.push(self.parse_expr(0)?);
        self.collect_trivia(&mut children, false);

        if !self.at_stmt_end() {
            return Err(self.error_here("expected end of assignment"));
        }

        Ok(NmtranNode {
            kind: NmtranNodeKind::Assignment,
            children,
        })
    }

    fn parse_if(&mut self) -> Result<NmtranNode, Diagnostic> {
        let mut children = vec![];

        self.collect_trivia(&mut children, false);
        self.eat(&mut children); // IF or ELSEIF
        self.collect_trivia(&mut children, false);
        self.parse_required_paren_expr(&mut children, "missing IF condition")?;

        if self.peek_is(NmtranToken::ThenKw) {
            self.eat(&mut children);
            self.collect_trivia(&mut children, false);

            if !self.at_stmt_end() {
                return Err(self.error_here("expected end of IF header after THEN"));
            }

            children.extend(self.parse_stmts_until(IF_TERMINATORS)?);
            self.collect_trivia(&mut children, true);

            if self.peek_is(NmtranToken::ElseIfKw) {
                children.push(NmtranChild::Node(self.parse_if()?));
            } else if self.peek_is(NmtranToken::ElseKw) {
                self.eat(&mut children);
                self.collect_trivia(&mut children, false);

                if self.peek_is(NmtranToken::IfKw) {
                    children.push(NmtranChild::Node(self.parse_if()?));
                } else {
                    if !self.at_stmt_end() {
                        return Err(self.error_here("expected end of ELSE header"));
                    }

                    children.extend(self.parse_stmts_until(ELSE_TERMINATORS)?);
                    self.collect_trivia(&mut children, true);
                    self.expect_end_keyword(
                        &mut children,
                        NmtranToken::EndIfKw,
                        NmtranToken::IfKw,
                        "ENDIF",
                    )?;
                }
            } else {
                self.expect_end_keyword(
                    &mut children,
                    NmtranToken::EndIfKw,
                    NmtranToken::IfKw,
                    "ENDIF",
                )?;
            }
        } else {
            if self.at_stmt_end() {
                return Err(self.error_here("expected THEN or statement after IF condition"));
            }
            children.push(NmtranChild::Node(self.parse_stmt()?));
        }

        Ok(NmtranNode {
            kind: NmtranNodeKind::If,
            children,
        })
    }

    fn expect_end_keyword(
        &mut self,
        children: &mut Vec<NmtranChild>,
        compact_kw: NmtranToken,
        split_kw: NmtranToken,
        label: &str,
    ) -> Result<(), Diagnostic> {
        if self.peek_is(compact_kw) {
            self.eat(children);
            return Ok(());
        }

        if self.peek_is(NmtranToken::EndKw) {
            self.eat(children);
            self.collect_trivia(children, false);
            if self.peek_is(split_kw) {
                self.eat(children);
                return Ok(());
            }
        }

        Err(self.error_here(format!("missing {label}")))
    }

    fn parse_do_while(&mut self) -> Result<NmtranNode, Diagnostic> {
        let mut children = vec![];

        self.collect_trivia(&mut children, false);
        self.eat(&mut children); // DO or DOWHILE
        self.collect_trivia(&mut children, false);
        if matches!(self.peek(), Some(t) if *t == NmtranToken::WhileKw) {
            self.eat(&mut children); // WHILE (split form only)
            self.collect_trivia(&mut children, false);
        }
        self.parse_required_paren_expr(&mut children, "missing DO WHILE condition")?;

        if !self.at_stmt_end() {
            return Err(self.error_here("expected end of DO WHILE header"));
        }

        children.extend(self.parse_stmts_until(DO_TERMINATORS)?);
        self.collect_trivia(&mut children, true);
        self.expect_end_keyword(
            &mut children,
            NmtranToken::EndDoKw,
            NmtranToken::DoKw,
            "ENDDO",
        )?;

        Ok(NmtranNode {
            kind: NmtranNodeKind::DoWhile,
            children,
        })
    }

    fn parse_expr(&mut self, min_bp: u8) -> Result<NmtranChild, Diagnostic> {
        let mut lhs = self.parse_prefix()?;

        loop {
            let saved = self.idx;
            let mut trivia = vec![];
            self.collect_trivia(&mut trivia, false);

            let Some(tok) = self.peek() else {
                self.idx = saved;
                break;
            };
            let Some((l_bp, r_bp)) = infix_bp(tok) else {
                self.idx = saved;
                break;
            };
            if l_bp < min_bp {
                self.idx = saved;
                break;
            }

            let mut children = vec![lhs];
            children.extend(trivia);
            self.eat(&mut children); // operator
            self.collect_trivia(&mut children, false);
            children.push(self.parse_expr(r_bp)?);

            lhs = NmtranChild::Node(NmtranNode {
                kind: NmtranNodeKind::BinaryExpr,
                children,
            });
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<NmtranChild, Diagnostic> {
        match self.peek() {
            Some(NmtranToken::DotNot) => {
                let mut children = vec![];
                self.eat(&mut children);
                self.collect_trivia(&mut children, false);
                children.push(self.parse_expr(PREFIX_NOT_BP)?);
                Ok(NmtranChild::Node(NmtranNode {
                    kind: NmtranNodeKind::UnaryExpr,
                    children,
                }))
            }
            Some(NmtranToken::Minus | NmtranToken::Plus) => {
                let mut children = vec![];
                self.eat(&mut children);
                self.collect_trivia(&mut children, false);
                children.push(self.parse_expr(PREFIX_SIGN_BP)?);
                Ok(NmtranChild::Node(NmtranNode {
                    kind: NmtranNodeKind::UnaryExpr,
                    children,
                }))
            }
            Some(NmtranToken::LeftParen) => {
                let mut children = vec![];
                self.eat(&mut children); // (
                self.collect_trivia(&mut children, false);
                children.push(self.parse_expr(0)?);
                self.collect_trivia(&mut children, false);
                self.expect(NmtranToken::RightParen, &mut children)?;
                Ok(NmtranChild::Node(NmtranNode {
                    kind: NmtranNodeKind::ParenExpr,
                    children,
                }))
            }
            Some(NmtranToken::Ident)
                if matches!(
                    self.tokens.get(self.skip_inline_trivia_from(self.idx + 1)),
                    Some(t) if t.token == NmtranToken::LeftParen
                ) =>
            {
                let mut children = vec![];
                self.eat(&mut children); // ident
                self.collect_trivia(&mut children, false);
                self.expect(NmtranToken::LeftParen, &mut children)?;
                self.parse_arg_list(&mut children)?;
                self.collect_trivia(&mut children, false);
                self.expect(NmtranToken::RightParen, &mut children)?;
                Ok(NmtranChild::Node(NmtranNode {
                    kind: NmtranNodeKind::FunctionCall,
                    children,
                }))
            }
            Some(NmtranToken::Ident | NmtranToken::Int | NmtranToken::Float) => {
                let i = self.idx;
                self.idx += 1;
                Ok(NmtranChild::Token(i))
            }
            Some(t) => Err(Diagnostic::parse(
                ParseErrorKind::Message(format!("unexpected {t} in expression")),
                self.current_span(),
            )),
            None => Err(Diagnostic::parse(
                ParseErrorKind::Message("unexpected end of expression".into()),
                self.eof_span(),
            )),
        }
    }

    fn parse_arg_list(&mut self, parent: &mut Vec<NmtranChild>) -> Result<(), Diagnostic> {
        let mut args = vec![];

        loop {
            self.collect_trivia(&mut args, false);
            if matches!(self.peek(), Some(NmtranToken::RightParen) | None) {
                break;
            }

            args.push(self.parse_expr(0)?);
            self.collect_trivia(&mut args, false);

            if matches!(self.peek(), Some(NmtranToken::Comma)) {
                self.eat(&mut args);
            } else {
                break;
            }
        }

        if !args.is_empty() {
            parent.push(NmtranChild::Node(NmtranNode {
                kind: NmtranNodeKind::ArgList,
                children: args,
            }));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nmtran::lex_nmtran;

    fn parse_ok(input: &str) -> NmtranCodeBlock {
        NmtranParser::parse(lex_nmtran(input, 0)).expect("parse failed")
    }

    fn first_node(cb: &NmtranCodeBlock) -> &NmtranNode {
        cb.children
            .iter()
            .find_map(|child| match child {
                NmtranChild::Node(node) => Some(node),
                NmtranChild::Token(_) => None,
            })
            .expect("expected a node")
    }

    fn last_child_node(node: &NmtranNode) -> &NmtranNode {
        node.children
            .iter()
            .rev()
            .find_map(|child| match child {
                NmtranChild::Node(node) => Some(node),
                NmtranChild::Token(_) => None,
            })
            .expect("expected a child node")
    }

    fn first_child_node(node: &NmtranNode) -> &NmtranNode {
        node.children
            .iter()
            .find_map(|child| match child {
                NmtranChild::Node(node) => Some(node),
                NmtranChild::Token(_) => None,
            })
            .expect("expected a child node")
    }

    fn count_kind_in_node(node: &NmtranNode, kind: NmtranNodeKind) -> usize {
        let self_count = usize::from(node.kind == kind);
        self_count
            + node
                .children
                .iter()
                .map(|child| match child {
                    NmtranChild::Node(node) => count_kind_in_node(node, kind),
                    NmtranChild::Token(_) => 0,
                })
                .sum::<usize>()
    }

    fn count_kind_in_block(cb: &NmtranCodeBlock, kind: NmtranNodeKind) -> usize {
        cb.children
            .iter()
            .map(|child| match child {
                NmtranChild::Node(node) => count_kind_in_node(node, kind),
                NmtranChild::Token(_) => 0,
            })
            .sum()
    }

    #[test]
    fn parses_assignment_tree() {
        let cb = parse_ok("CL = THETA(1)\n");
        let assignment = first_node(&cb);
        assert_eq!(assignment.kind, NmtranNodeKind::Assignment);
        assert_eq!(
            last_child_node(assignment).kind,
            NmtranNodeKind::FunctionCall
        );
    }

    #[test]
    fn unary_minus_wraps_power_expression() {
        let cb = parse_ok("X = -A**2\n");
        let assignment = first_node(&cb);
        let rhs = last_child_node(assignment);
        assert_eq!(rhs.kind, NmtranNodeKind::UnaryExpr);
        assert_eq!(first_child_node(rhs).kind, NmtranNodeKind::BinaryExpr);
    }

    #[test]
    fn dot_not_wraps_comparison_expression() {
        let cb = parse_ok("IF (.NOT. A .EQ. B) X = 1\n");
        let if_node = first_node(&cb);
        let condition = first_child_node(if_node);
        assert_eq!(condition.kind, NmtranNodeKind::UnaryExpr);
        assert_eq!(first_child_node(condition).kind, NmtranNodeKind::BinaryExpr);
    }

    #[test]
    fn parses_if_else_if_else_tree() {
        let cb = parse_ok(
            "IF (X.GT.0) THEN\n  Y = 1\nELSE IF (X.LT.0) THEN\n  Y = 2\nELSE\n  Y = 3\nENDIF\n",
        );
        assert_eq!(first_node(&cb).kind, NmtranNodeKind::If);
    }

    #[test]
    fn parses_do_while_tree() {
        let cb = parse_ok("DO WHILE (I.LT.10)\n  X = X + 1\nENDDO\n");
        let do_while = first_node(&cb);
        assert_eq!(do_while.kind, NmtranNodeKind::DoWhile);
    }

    #[test]
    fn parses_single_token_dowhile() {
        let cb = parse_ok("DOWHILE (I.LT.10)\n  X = X + 1\nENDDO\n");
        let do_while = first_node(&cb);
        assert_eq!(do_while.kind, NmtranNodeKind::DoWhile);
    }

    #[test]
    fn accepts_end_if_and_end_do_spellings() {
        let cb = parse_ok(
            "IF (X.GT.0) THEN ; keep going\n  DO WHILE (I.LT.10)\n    X = X + 1\n  END DO\nEND IF\n",
        );
        assert_eq!(count_kind_in_block(&cb, NmtranNodeKind::If), 1);
        assert_eq!(count_kind_in_block(&cb, NmtranNodeKind::DoWhile), 1);
    }

    #[test]
    fn condition_line_continuation_parses() {
        let cb = parse_ok("IF (X.GT.0 .AND. &\n  Y.LT.2) Z = 1\n");
        assert_eq!(first_node(&cb).kind, NmtranNodeKind::If);
    }

    #[test]
    fn dotted_operator_against_numeric_literal_parses() {
        // From real ddmore models: a numeric literal jammed against `.AND.` /
        // `.EQ.` with no whitespace. The lexer must not let Float swallow the
        // leading dot of the next operator.
        parse_ok("IF (CMT.EQ.4.AND.EVID.EQ.0) Y = 1\n");
        parse_ok("IF (CENSORING.EQ.1.AND.DV.EQ.-1.AND.TIME.GT.0) CS = 1\n");
    }

    #[test]
    fn else_newline_if_parses_as_nested_if() {
        let cb = parse_ok(
            "IF (X.GT.0) THEN\n  Y = 1\nELSE\n  IF (X.LT.0) THEN\n    Y = 2\n  ENDIF\nENDIF\n",
        );
        assert_eq!(count_kind_in_block(&cb, NmtranNodeKind::If), 2);
        assert_eq!(count_kind_in_block(&cb, NmtranNodeKind::Assignment), 2);
        assert_eq!(count_kind_in_block(&cb, NmtranNodeKind::Unknown), 0);
    }

    #[test]
    fn rejects_stray_top_level_terminator() {
        let result = NmtranParser::parse(lex_nmtran("X = 1\nENDIF\nY = 2\n", 0));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_empty_if_condition() {
        let result = NmtranParser::parse(lex_nmtran("IF () Y = 1\n", 0));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_bare_if_header() {
        let result = NmtranParser::parse(lex_nmtran("IF (X.GT.0)\n", 0));
        assert!(result.is_err());
    }
}
