use std::ops::Range;

use crate::cst::{CstChild, CstNode, NodeKind};
use crate::errors::{Diagnostic, ParseErrorKind};
use crate::lexer;
use crate::lexer::{SpannedToken, Token};
use crate::nmtran::NmtranParser;
use crate::nmtran::lex_nmtran;

/// Each data item label consists of letters (A-Z) and numerals (0-9), but it must begin with a letter.
/// Starting with NONMEM 7.1, the underscore character _ may be used in a data item
fn is_valid_label(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[derive(Debug)]
pub(crate) struct Parser {
    idx: usize,
    tokens: Vec<SpannedToken>,
    source: String,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let source = input.replace("\r\n", "\n");
        let tokens = lexer::lex(&source);
        Self {
            idx: 0,
            tokens,
            source,
        }
    }

    pub fn parse(mut self) -> Result<(CstNode, Vec<SpannedToken>, String), Diagnostic> {
        let mut root = CstNode::new(NodeKind::Root);

        while self.idx < self.tokens.len() {
            self.collect_trivia(&mut root);

            match self.peek() {
                Some(t) if t.token == Token::ControlRecord => {
                    let record_name = t.text.to_uppercase();

                    let node = match record_name.as_str() {
                        "$PROBLEM" | "$PROB" => self.parse_problem()?,
                        "$INPUT" | "$INPT" => self.parse_input()?,
                        "$DATA" => self.parse_data()?,
                        "$SUBROUTINES" | "$SUB" | "$SUBROUTINE" => self.parse_subroutines()?,
                        "$THETA" | "$THTA" => self.parse_theta()?,
                        "$OMEGA" | "$OMEG" => self.parse_omega()?,
                        "$SIGMA" | "$SIGM" => self.parse_sigma()?,
                        "$ESTIMATION" | "$EST" => self.parse_estimation()?,
                        "$TABLE" | "$TABL" => self.parse_table()?,
                        "$SIMULATION" | "$SIM" => self.parse_simulation()?,
                        "$COVARIANCE" | "$COV" => self.parse_covariance()?,
                        "$ABBREVIATED" | "$ABBR" => self.parse_abbreviated()?,
                        "$PK" => self.parse_code_block(NodeKind::Pk)?,
                        "$ERROR" | "$ERR" | "$ERRO" => {
                            self.parse_code_block(NodeKind::ErrorBlock)?
                        }
                        "$DES" => self.parse_code_block(NodeKind::Des)?,
                        "$PRED" | "$PRE" => self.parse_code_block(NodeKind::Pred)?,
                        _ => {
                            let mut unknown = CstNode::new(NodeKind::UnknownRecord);
                            self.eat(&mut unknown);
                            while !self.at_end_of_record() {
                                self.eat(&mut unknown);
                            }
                            root.children.push(CstChild::Node(unknown));
                            continue;
                        }
                    };

                    root.children.push(CstChild::Node(node));
                }
                None => break,
                Some(t) => {
                    return Err(Diagnostic::parse(
                        ParseErrorKind::Message(format!(
                            "expected a control record (e.g. $PROBLEM, $DATA), found '{}'",
                            t.text
                        )),
                        t.span.clone(),
                    ));
                }
            }
        }

        Ok((root, self.tokens, self.source))
    }

    fn peek(&self) -> Option<&SpannedToken> {
        self.tokens.get(self.idx)
    }

    fn peek_or_eof(&self, expected: &[Token]) -> Result<&SpannedToken, Diagnostic> {
        if let Some(t) = self.peek() {
            Ok(t)
        } else {
            Err(Diagnostic::parse(
                ParseErrorKind::UnexpectedEof {
                    expected: expected.to_vec(),
                },
                self.eof_span(),
            ))
        }
    }

    fn eof_span(&self) -> Range<usize> {
        self.tokens
            .last()
            .map(|t| t.span.end..t.span.end)
            .unwrap_or(0..0)
    }

    fn peek_non_trivia(&self) -> Option<&SpannedToken> {
        let mut i = self.idx;
        while i < self.tokens.len() {
            match self.tokens[i].token {
                Token::Whitespace | Token::Newline | Token::Comment => i += 1,
                _ => return Some(&self.tokens[i]),
            }
        }
        None
    }

    fn expect(&mut self, expected: Token, node: &mut CstNode) -> Result<usize, Diagnostic> {
        self.collect_trivia(node);
        match self.peek() {
            Some(tok) if tok.token == expected => {
                let idx = self.idx;
                self.eat(node);
                Ok(idx)
            }
            Some(tok) => Err(Diagnostic::parse(
                ParseErrorKind::UnexpectedToken {
                    expected: vec![expected],
                    found: tok.token.clone(),
                },
                tok.span.clone(),
            )),
            None => Err(Diagnostic::parse(
                ParseErrorKind::UnexpectedEof {
                    expected: vec![expected],
                },
                self.eof_span(),
            )),
        }
    }

    fn eat(&mut self, node: &mut CstNode) {
        node.children.push(CstChild::Token(self.idx));
        self.idx += 1;
    }

    fn collect_trivia(&mut self, node: &mut CstNode) {
        while let Some(tok) = self.peek() {
            match tok.token {
                Token::Whitespace | Token::Newline | Token::Comment => {
                    node.children.push(CstChild::Token(self.idx));
                    self.idx += 1;
                }
                _ => break,
            }
        }
    }

    /// Look ahead to see if current Symbol is followed by = (possibly with trivia in between)
    fn peek_is_named_param(&self) -> bool {
        let mut i = self.idx;
        if !matches!(self.tokens.get(i), Some(t) if t.token == Token::Symbol) {
            return false;
        }
        i += 1;
        while matches!(self.tokens.get(i), Some(t) if matches!(t.token, Token::Whitespace | Token::Newline))
        {
            i += 1;
        }
        matches!(self.tokens.get(i), Some(t) if t.token == Token::Equals)
    }

    fn at_end_of_record(&self) -> bool {
        match self.peek_non_trivia() {
            None => true,
            Some(tok) => tok.token == Token::ControlRecord,
        }
    }

    fn parse_code_block(&mut self, kind: NodeKind) -> Result<CstNode, Diagnostic> {
        let mut node = CstNode::new(kind);

        // Eat the control record token (e.g. $PK)
        let body_start = self.tokens[self.idx].span.end;
        self.eat(&mut node);

        // Skip all main-lexer tokens until the next record or EOF.
        // These tokens are incorrectly tokenized by the main lexer,
        // so we discard them and re-lex with the NMTRAN tokenizer.
        while !self.at_end_of_record() {
            self.idx += 1;
        }

        let body_end = if self.idx < self.tokens.len() {
            self.tokens[self.idx].span.start
        } else {
            self.source.len()
        };

        let body_text = &self.source[body_start..body_end];
        let nmtran_tokens = lex_nmtran(body_text, body_start);
        let code_block = NmtranParser::parse(nmtran_tokens)?;
        node.children.push(CstChild::CodeBlock(code_block));

        Ok(node)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.1.%20$PROBLEM%20Record
    fn parse_problem(&mut self) -> Result<CstNode, Diagnostic> {
        let mut node = CstNode::new(NodeKind::Problem);
        self.eat(&mut node);
        while !self.at_end_of_record() {
            self.eat(&mut node);
        }
        Ok(node)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.2.%20$INPUT%20Record
    //
    // $INPUT @item sub 1 ~ item sub 2 ~ item sub 3 ~...@
    fn parse_input(&mut self) -> Result<CstNode, Diagnostic> {
        let mut node = CstNode::new(NodeKind::Input);
        self.eat(&mut node);

        while !self.at_end_of_record() {
            self.collect_trivia(&mut node);

            let tok = self.peek_or_eof(&[Token::Symbol, Token::Comma])?;
            // Commas can be used to separate items
            if tok.token == Token::Comma {
                self.eat(&mut node);
                continue;
            }

            if tok.token != Token::Symbol {
                return Err(Diagnostic::parse(
                    ParseErrorKind::UnexpectedToken {
                        expected: vec![Token::Symbol],
                        found: tok.token.clone(),
                    },
                    tok.span.clone(),
                ));
            }
            if !is_valid_label(&tok.text) {
                return Err(Diagnostic::parse(
                    ParseErrorKind::InvalidLabel {
                        text: tok.text.clone(),
                    },
                    tok.span.clone(),
                ));
            }

            let mut col_node = CstNode::new(NodeKind::InputColumn);
            self.eat(&mut col_node);
            self.collect_trivia(&mut col_node);
            // Now we check for `= alias`
            if matches!(self.peek(), Some(t) if t.token == Token::Equals) {
                self.eat(&mut col_node);
                self.collect_trivia(&mut col_node);
                match self.peek() {
                    Some(t) if t.token == Token::Symbol => {
                        if !is_valid_label(&t.text) {
                            return Err(Diagnostic::parse(
                                ParseErrorKind::InvalidLabel {
                                    text: t.text.clone(),
                                },
                                t.span.clone(),
                            ));
                        }
                        self.eat(&mut col_node);
                    }
                    other => {
                        return Err(match other {
                            Some(t) => Diagnostic::parse(
                                ParseErrorKind::UnexpectedToken {
                                    expected: vec![Token::Symbol],
                                    found: t.token.clone(),
                                },
                                t.span.clone(),
                            ),
                            None => Diagnostic::parse(
                                ParseErrorKind::UnexpectedEof {
                                    expected: vec![Token::Symbol],
                                },
                                self.eof_span(),
                            ),
                        });
                    }
                }
            }
            node.children.push(CstChild::Node(col_node));
        }

        Ok(node)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.5.%20$DATA%20Record
    //
    // $DATA [filename|*] [(format)] [IGNORE=@c sub 1@] [NULL=@c sub 2@]
    // [IGNORE=(list)...|ACCEPT=(list)...]
    // [PRED_IGNORE_DATA]
    // [NOWIDE|WIDE] [CHECKOUT]
    // [RECORDS=@n sub 1@|RECORDS=label]
    // [LRECL=@n sub 2@] [NOREWIND|REWIND]
    // [NOOPEN] [LAST20=@n sub 3@] [TRANSLATE=(list)]
    // [BLANKOK]
    // [MISDAT=@r@...]
    // [REPL=@n@...]
    // Notes:
    // = is optional for all A=B options. Commas are optional separators.
    // TRANSLATE not supported
    fn parse_data(&mut self) -> Result<CstNode, Diagnostic> {
        let mut node = CstNode::new(NodeKind::Data);
        self.eat(&mut node);
        self.collect_trivia(&mut node);

        // First non-trivia is the filename/path which can be quoted or not
        match self.peek().map(|t| &t.token) {
            Some(Token::QuotedString | Token::Symbol) => self.eat(&mut node),
            _ => {
                return Err(Diagnostic::parse(
                    ParseErrorKind::Message("expected a filename after $DATA".into()),
                    self.eof_span(),
                ));
            }
        }

        // Then we have a bunch of options, we don't care about all of them tbh
        while !self.at_end_of_record() {
            self.collect_trivia(&mut node);

            match self.peek_or_eof(&[Token::Symbol, Token::Comma])? {
                // optional , between flags
                tok if tok.token == Token::Comma => {
                    self.eat(&mut node);
                    continue;
                }
                tok if tok.token == Token::Symbol => {
                    let keyword = tok.text.to_uppercase();

                    match keyword.as_str() {
                        // key-value with filters as value
                        "IGNORE" | "IGN" | "ACCEPT" => {
                            let mut kv = CstNode::new(NodeKind::KeyValue);
                            self.eat(&mut kv);
                            self.collect_trivia(&mut kv);

                            // Optional = as something like IGN(ID.EQ.3.14) is valid
                            if matches!(self.peek(), Some(t) if t.token == Token::Equals) {
                                self.eat(&mut kv);
                                self.collect_trivia(&mut kv);
                            }

                            match self.peek() {
                                Some(t) if t.token == Token::LeftParen => {
                                    // Inline filter list: (DVID.EQ.3) or (AGE.GT.3,SEX.EQ.1)
                                    let mut parens = CstNode::new(NodeKind::Parens);
                                    self.eat(&mut parens);

                                    loop {
                                        self.collect_trivia(&mut parens);

                                        match self.peek().map(|t| &t.token) {
                                            Some(Token::RightParen) => {
                                                self.eat(&mut parens);
                                                break;
                                            }
                                            Some(Token::Comma) => {
                                                self.eat(&mut parens);
                                                continue;
                                            }
                                            None => {
                                                return Err(Diagnostic::parse(
                                                    ParseErrorKind::UnexpectedEof {
                                                        expected: vec![Token::RightParen],
                                                    },
                                                    self.eof_span(),
                                                ));
                                            }
                                            _ => {
                                                // we will actually parse it later when lowering
                                                let mut filter = CstNode::new(NodeKind::Filter);
                                                while let Some(tok) = self.peek() {
                                                    match tok.token {
                                                        Token::Comma | Token::RightParen => break,
                                                        _ => self.eat(&mut filter),
                                                    }
                                                }
                                                parens.children.push(CstChild::Node(filter));
                                            }
                                        }
                                    }

                                    kv.children.push(CstChild::Node(parens));
                                }
                                Some(t)
                                    if !matches!(t.token, Token::ControlRecord | Token::Comma) =>
                                {
                                    // Something like IGNORE=#
                                    self.eat(&mut kv);
                                }
                                Some(t) => {
                                    return Err(Diagnostic::parse(
                                        ParseErrorKind::UnexpectedToken {
                                            expected: vec![Token::LeftParen],
                                            found: t.token.clone(),
                                        },
                                        t.span.clone(),
                                    ));
                                }
                                _ => {
                                    return Err(Diagnostic::parse(
                                        ParseErrorKind::UnexpectedEof {
                                            expected: vec![Token::LeftParen, Token::Symbol],
                                        },
                                        self.eof_span(),
                                    ));
                                }
                            }

                            node.children.push(CstChild::Node(kv));
                        }
                        // other key values with int/floats/symbol
                        "NULL" | "RECORDS" | "REC" | "LRECL" | "LAST20" | "MISDAT" | "REPL" => {
                            let mut kv = CstNode::new(NodeKind::KeyValue);
                            self.eat(&mut kv);
                            self.collect_trivia(&mut kv);

                            // Optional =
                            if matches!(self.peek(), Some(t) if t.token == Token::Equals) {
                                self.eat(&mut kv); // =
                                self.collect_trivia(&mut kv);
                            }

                            match self.peek() {
                                Some(t)
                                    if matches!(
                                        t.token,
                                        Token::Int | Token::Float | Token::Symbol
                                    ) =>
                                {
                                    self.eat(&mut kv); // value
                                }
                                Some(t) => {
                                    return Err(Diagnostic::parse(
                                        ParseErrorKind::UnexpectedToken {
                                            expected: vec![Token::Int, Token::Float, Token::Symbol],
                                            found: t.token.clone(),
                                        },
                                        t.span.clone(),
                                    ));
                                }
                                _ => {
                                    return Err(Diagnostic::parse(
                                        ParseErrorKind::UnexpectedEof {
                                            expected: vec![Token::Int, Token::Float, Token::Symbol],
                                        },
                                        self.eof_span(),
                                    ));
                                }
                            }

                            node.children.push(CstChild::Node(kv));
                        }
                        // just a flag
                        "NOWIDE" | "WIDE" | "CHECKOUT" | "NOREWIND" | "REWIND" | "NOOPEN"
                        | "BLANKOK" | "PRED_IGNORE_DATA" => {
                            let mut flag = CstNode::new(NodeKind::Flag);
                            self.eat(&mut flag);
                            node.children.push(CstChild::Node(flag));
                        }

                        _ => {
                            return Err(Diagnostic::parse(
                                ParseErrorKind::Message(format!(
                                    "unknown $DATA option '{}'",
                                    tok.text
                                )),
                                tok.span.clone(),
                            ));
                        }
                    }
                }
                tok => {
                    return Err(Diagnostic::parse(
                        ParseErrorKind::UnexpectedToken {
                            expected: vec![Token::Symbol],
                            found: tok.token.clone(),
                        },
                        tok.span.clone(),
                    ));
                }
            }
        }

        Ok(node)
    }

    /// Parse NAMES(...)
    fn parse_names(&mut self) -> Result<CstNode, Diagnostic> {
        let mut node = CstNode::new(NodeKind::ParamNames);
        self.eat(&mut node);
        self.collect_trivia(&mut node);
        self.expect(Token::LeftParen, &mut node)?;

        loop {
            self.collect_trivia(&mut node);
            match self.peek().map(|t| &t.token) {
                Some(Token::RightParen) => {
                    self.eat(&mut node);
                    break;
                }
                Some(Token::Comma) => {
                    self.eat(&mut node);
                }
                Some(Token::Symbol) => {
                    self.eat(&mut node);
                }
                Some(_) => {
                    let tok = self.peek().unwrap();
                    return Err(Diagnostic::parse(
                        ParseErrorKind::UnexpectedToken {
                            expected: vec![Token::Symbol, Token::Comma, Token::RightParen],
                            found: tok.token.clone(),
                        },
                        tok.span.clone(),
                    ));
                }
                None => {
                    return Err(Diagnostic::parse(
                        ParseErrorKind::UnexpectedEof {
                            expected: vec![Token::Symbol, Token::Comma, Token::RightParen],
                        },
                        self.eof_span(),
                    ));
                }
            }
        }

        Ok(node)
    }

    fn parse_values(&mut self) -> Result<CstNode, Diagnostic> {
        let mut node = CstNode::new(NodeKind::ParamValues);
        self.eat(&mut node);
        self.collect_trivia(&mut node);
        self.expect(Token::LeftParen, &mut node)?;

        loop {
            self.collect_trivia(&mut node);
            let tok =
                self.peek_or_eof(&[Token::RightParen, Token::Comma, Token::Int, Token::Float])?;
            match tok.token {
                Token::RightParen => {
                    self.eat(&mut node);
                    break;
                }
                Token::Comma => {
                    self.eat(&mut node);
                }
                Token::Int | Token::Float => {
                    self.eat(&mut node);
                }
                _ => {
                    return Err(Diagnostic::parse(
                        ParseErrorKind::UnexpectedToken {
                            expected: vec![
                                Token::Int,
                                Token::Float,
                                Token::Comma,
                                Token::RightParen,
                            ],
                            found: tok.token.clone(),
                        },
                        tok.span.clone(),
                    ));
                }
            }
        }

        Ok(node)
    }

    fn parse_params_parens(&mut self) -> Result<CstNode, Diagnostic> {
        let mut node = CstNode::new(NodeKind::Parens);
        self.eat(&mut node);

        loop {
            self.collect_trivia(&mut node);
            let tok = self.peek_or_eof(&[
                Token::RightParen,
                Token::Comma,
                Token::Int,
                Token::Float,
                Token::Infinity,
            ])?;
            match tok.token {
                Token::RightParen => {
                    self.eat(&mut node);
                    break;
                }
                Token::Comma | Token::Int | Token::Float | Token::Infinity => {
                    self.eat(&mut node);
                }
                Token::Symbol
                    if tok.text.eq_ignore_ascii_case("FIX")
                        || tok.text.eq_ignore_ascii_case("FIXED") =>
                {
                    let mut flag = CstNode::new(NodeKind::Flag);
                    self.eat(&mut flag);
                    node.children.push(CstChild::Node(flag));
                }
                _ => {
                    return Err(Diagnostic::parse(
                        ParseErrorKind::UnexpectedToken {
                            expected: vec![
                                Token::RightParen,
                                Token::Int,
                                Token::Float,
                                Token::Infinity,
                                Token::Comma,
                            ],
                            found: tok.token.clone(),
                        },
                        tok.span.clone(),
                    ));
                }
            }
        }

        Ok(node)
    }

    /// Parses simple key and key-val, eg $subroutines or $tables or $est
    fn parse_simple_options(&mut self, kind: NodeKind) -> Result<CstNode, Diagnostic> {
        let mut node = CstNode::new(kind);
        self.eat(&mut node);

        while !self.at_end_of_record() {
            self.collect_trivia(&mut node);
            let tok = self.peek_or_eof(&[Token::Symbol])?;
            match tok.token {
                Token::Comment => {
                    self.eat(&mut node);
                }
                Token::Symbol => {
                    // Start as Flag; upgrade to KeyValue if followed by = or bare value
                    let mut child = CstNode::new(NodeKind::Flag);
                    self.eat(&mut child);
                    self.collect_trivia(&mut child);

                    match self.peek() {
                        Some(t) if t.token == Token::Equals => {
                            // KEY=VALUE
                            child.kind = NodeKind::KeyValue;
                            self.eat(&mut child);
                            self.collect_trivia(&mut child);
                            if matches!(
                                self.peek_or_eof(&[
                                    Token::Int,
                                    Token::Float,
                                    Token::Infinity,
                                    Token::Symbol,
                                    Token::QuotedString
                                ])?
                                .token,
                                Token::Int
                                    | Token::Float
                                    | Token::Infinity
                                    | Token::Symbol
                                    | Token::QuotedString
                            ) {
                                self.eat(&mut child);
                            }
                        }
                        Some(t)
                            if matches!(
                                t.token,
                                Token::Int | Token::Float | Token::Infinity | Token::QuotedString
                            ) =>
                        {
                            // KEY VALUE (no =)
                            child.kind = NodeKind::KeyValue;
                            self.eat(&mut child); // value
                        }
                        Some(t) if t.token == Token::LeftParen => {
                            // SYMBOL(...) — e.g., ETAS(1:LAST)
                            self.eat(&mut child); // (
                            while let Some(t) = self.peek() {
                                match t.token {
                                    Token::RightParen => {
                                        self.eat(&mut child); // )
                                        break;
                                    }
                                    Token::ControlRecord | Token::Newline => break,
                                    _ => self.eat(&mut child),
                                }
                            }
                        }
                        _ => (),
                    }
                    node.children.push(CstChild::Node(child));
                }
                _ => break,
            }
        }

        Ok(node)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.6.%20$SUBROUTINES%20Record
    // $SUBROUTINES [subname1 = name1] [subname2 = name2]
    fn parse_subroutines(&mut self) -> Result<CstNode, Diagnostic> {
        self.parse_simple_options(NodeKind::Subroutines)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.14.%20$ESTIMATION%20Record
    fn parse_estimation(&mut self) -> Result<CstNode, Diagnostic> {
        self.parse_simple_options(NodeKind::Estimation)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.16.%20$TABLE%20Record
    fn parse_table(&mut self) -> Result<CstNode, Diagnostic> {
        self.parse_simple_options(NodeKind::Table)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.13.%20$SIMULATION%20Record
    fn parse_simulation(&mut self) -> Result<CstNode, Diagnostic> {
        self.parse_simple_options(NodeKind::Simulation)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.15.%20$COVARIANCE%20Record
    fn parse_covariance(&mut self) -> Result<CstNode, Diagnostic> {
        self.parse_simple_options(NodeKind::Covariance)
    }

    // https://nmhelp.tingjieguo.com/IV/III#$ABBREVIATED
    fn parse_abbreviated(&mut self) -> Result<CstNode, Diagnostic> {
        let mut node = CstNode::new(NodeKind::Abbreviated);
        self.eat(&mut node);

        while !self.at_end_of_record() {
            self.collect_trivia(&mut node);

            let tok = match self.peek() {
                Some(t) => t,
                None => break,
            };

            if tok.token != Token::Symbol {
                break;
            }

            let keyword = tok.text.to_uppercase();
            match keyword.as_str() {
                "REPLACE" => {
                    let mut replace = CstNode::new(NodeKind::Replace);
                    self.eat(&mut replace); // REPLACE
                    self.collect_trivia(&mut replace);

                    // left-hand side: eat tokens until =
                    while let Some(t) = self.peek() {
                        match t.token {
                            Token::Equals => break,
                            Token::ControlRecord | Token::Newline | Token::Comment => break,
                            _ => self.eat(&mut replace),
                        }
                    }

                    // expect =
                    self.expect(Token::Equals, &mut replace)?;
                    self.collect_trivia(&mut replace);

                    // right-hand side: eat tokens until next keyword, newline, or end-of-record
                    while let Some(t) = self.peek() {
                        match t.token {
                            Token::ControlRecord | Token::Newline | Token::Comment => break,
                            Token::Whitespace => {
                                // Check if next non-trivia is a keyword (REPLACE, DECLARE, etc.)
                                // or another symbol that starts a new option — peek ahead
                                if self.peek_is_named_param() {
                                    break;
                                }
                                // check if after whitespace we have a keyword
                                let mut j = self.idx + 1;
                                while j < self.tokens.len()
                                    && self.tokens[j].token == Token::Whitespace
                                {
                                    j += 1;
                                }
                                if j < self.tokens.len() && self.tokens[j].token == Token::Symbol {
                                    let next_upper = self.tokens[j].text.to_uppercase();
                                    if matches!(
                                        next_upper.as_str(),
                                        "REPLACE"
                                            | "DECLARE"
                                            | "FUNCTION"
                                            | "VECTOR"
                                            | "COMRES"
                                            | "DERIV2"
                                            | "TRANS"
                                    ) {
                                        break;
                                    }
                                }
                                self.eat(&mut replace);
                            }
                            _ => {
                                self.eat(&mut replace);
                            }
                        }
                    }

                    node.children.push(CstChild::Node(replace));
                }
                "DECLARE" => {
                    let mut declare = CstNode::new(NodeKind::Declare);
                    self.eat(&mut declare);
                    // collect tokens until next keyword or end of record
                    while !self.at_end_of_record() {
                        match self.peek() {
                            Some(t) if t.token == Token::Newline || t.token == Token::Comment => {
                                self.eat(&mut declare);
                                break;
                            }
                            Some(_) => self.eat(&mut declare),
                            None => break,
                        }
                    }
                    node.children.push(CstChild::Node(declare));
                }
                // Not parsed, just eaten as loose tokens
                "FUNCTION" | "VECTOR" => {
                    self.eat(&mut node);
                    while !self.at_end_of_record() {
                        match self.peek() {
                            Some(t) if t.token == Token::Newline || t.token == Token::Comment => {
                                self.eat(&mut node);
                                break;
                            }
                            Some(_) => self.eat(&mut node),
                            None => break,
                        }
                    }
                }
                _ => {
                    let mut child = CstNode::new(NodeKind::Flag);
                    self.eat(&mut child);
                    self.collect_trivia(&mut child);

                    match self.peek() {
                        Some(t) if t.token == Token::Equals => {
                            child.kind = NodeKind::KeyValue;
                            self.eat(&mut child);
                            self.collect_trivia(&mut child);
                            if matches!(
                                self.peek().map(|t| &t.token),
                                Some(
                                    Token::Int
                                        | Token::Float
                                        | Token::Infinity
                                        | Token::Symbol
                                        | Token::QuotedString
                                )
                            ) {
                                self.eat(&mut child);
                            }
                        }
                        Some(t)
                            if matches!(
                                t.token,
                                Token::Int | Token::Float | Token::Infinity | Token::QuotedString
                            ) =>
                        {
                            child.kind = NodeKind::KeyValue;
                            self.eat(&mut child);
                        }
                        _ => (),
                    }
                    node.children.push(CstChild::Node(child));
                }
            }
        }

        Ok(node)
    }

    fn maybe_parse_fix(&mut self, node: &mut CstNode) {
        if let Some(t) = self.peek_non_trivia()
            && t.token == Token::Symbol
            && (t.text.eq_ignore_ascii_case("FIX") || t.text.eq_ignore_ascii_case("FIXED"))
        {
            self.collect_trivia(node);
            let mut flag = CstNode::new(NodeKind::Flag);
            self.eat(&mut flag);
            node.children.push(CstChild::Node(flag));
        }
    }

    fn maybe_parse_repeat(&mut self, node: &mut CstNode) {
        if let Some(t) = self.peek_non_trivia()
            && t.token == Token::Symbol
            && (t.text.starts_with('x') || t.text.starts_with('X'))
        {
            self.collect_trivia(node);
            let mut rep = CstNode::new(NodeKind::Repeat);
            self.eat(&mut rep);
            node.children.push(CstChild::Node(rep));
        }
    }

    fn parse_bounds(&mut self) -> Result<CstNode, Diagnostic> {
        let mut param = CstNode::new(NodeKind::Param);
        param
            .children
            .push(CstChild::Node(self.parse_params_parens()?));

        // maybe there's xN syntax
        self.maybe_parse_repeat(&mut param);
        // maybe there's FIX/FIXED
        self.maybe_parse_fix(&mut param);
        Ok(param)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.9.%20$THETA%20Record
    //
    // $THETA value1 [ value2 ] [ value3 ] ...
    // [( value_k ) x n ]
    // [label= value ... FIXED]
    // [NAMES (label ...)value ...]
    // [NUMBERPOINTS=n]
    // [ABORT|NOABORT|NOABORTFIRST]
    fn parse_theta(&mut self) -> Result<CstNode, Diagnostic> {
        let mut node = CstNode::new(NodeKind::Theta);
        self.eat(&mut node);
        self.collect_trivia(&mut node);

        // Look for NAMES syntax first
        if let Some(t) = self.peek()
            && t.token == Token::Symbol
            && t.text.eq_ignore_ascii_case("NAMES")
        {
            node.children.push(CstChild::Node(self.parse_names()?));
        }

        while !self.at_end_of_record() {
            self.collect_trivia(&mut node);

            let tok = self.peek_or_eof(&[
                Token::Int,
                Token::Float,
                Token::Infinity,
                Token::LeftParen,
                Token::Symbol,
            ])?;
            match tok.token {
                Token::ControlRecord => break,
                // Bare number: $THETA 1.5 or $THETA 2.3 FIX
                Token::Int | Token::Float | Token::Infinity => {
                    let mut param = CstNode::new(NodeKind::Param);
                    self.eat(&mut param);

                    // Check for FIX/FIXED after
                    self.maybe_parse_fix(&mut param);
                    node.children.push(CstChild::Node(param));
                }

                // Parenthesized: (low, init, high) or (init FIX)
                Token::LeftParen => {
                    node.children.push(CstChild::Node(self.parse_bounds()?));
                }

                // Named: CL=(0, 1.5, 10) or NAME=value
                Token::Symbol => {
                    let mut param = CstNode::new(NodeKind::Param);
                    self.eat(&mut param);
                    self.collect_trivia(&mut param);
                    self.expect(Token::Equals, &mut param)?;
                    self.collect_trivia(&mut param);

                    let next = self.peek_or_eof(&[
                        Token::LeftParen,
                        Token::Int,
                        Token::Float,
                        Token::Infinity,
                    ])?;
                    match next.token {
                        Token::LeftParen => {
                            param
                                .children
                                .push(CstChild::Node(self.parse_params_parens()?));
                        }
                        Token::Int | Token::Float | Token::Infinity => {
                            self.eat(&mut param);
                        }
                        _ => {
                            return Err(Diagnostic::parse(
                                ParseErrorKind::UnexpectedToken {
                                    expected: vec![
                                        Token::LeftParen,
                                        Token::Int,
                                        Token::Float,
                                        Token::Infinity,
                                    ],
                                    found: next.token.clone(),
                                },
                                next.span.clone(),
                            ));
                        }
                    }

                    // maybe there's xN syntax
                    self.maybe_parse_repeat(&mut param);
                    // maybe there's FIX/FIXED
                    self.maybe_parse_fix(&mut param);
                    node.children.push(CstChild::Node(param));
                }

                Token::Comment => {
                    self.eat(&mut node);
                }
                _ => {
                    return Err(Diagnostic::parse(
                        ParseErrorKind::Message(format!(
                            "unexpected token '{}' in $THETA",
                            tok.text
                        )),
                        tok.span.clone(),
                    ));
                }
            }
        }

        self.collect_trivia(&mut node);
        Ok(node)
    }

    fn parse_omega_sigma(&mut self, kind: NodeKind) -> Result<CstNode, Diagnostic> {
        let mut node = CstNode::new(kind);
        self.eat(&mut node);
        self.collect_trivia(&mut node);

        // BLOCK must come first if present
        let tok = self.peek_or_eof(&[Token::Symbol, Token::Int, Token::Float, Token::Infinity])?;
        if tok.token == Token::Symbol && tok.text.eq_ignore_ascii_case("BLOCK") {
            let mut block = CstNode::new(NodeKind::Block);
            self.eat(&mut block);
            self.collect_trivia(&mut block);
            // (n)
            self.expect(Token::LeftParen, &mut block)?;
            self.collect_trivia(&mut block);
            self.eat(&mut block);
            self.collect_trivia(&mut block);
            self.expect(Token::RightParen, &mut block)?;
            node.children.push(CstChild::Node(block));
        }

        // then parse everything else
        while !self.at_end_of_record() {
            self.collect_trivia(&mut node);
            let tok = self.peek_or_eof(&[
                Token::Int,
                Token::Float,
                Token::Infinity,
                Token::LeftParen,
                Token::Symbol,
            ])?;

            match &tok.token {
                Token::Int | Token::Float | Token::Infinity => {
                    let mut param = CstNode::new(NodeKind::Param);
                    self.eat(&mut param);
                    self.maybe_parse_fix(&mut param);
                    node.children.push(CstChild::Node(param));
                }
                Token::LeftParen => {
                    // OMEGA/SIGMA do not have bounds so it's just a number
                    // with optionally FIX/FIXED and potentially xN syntax after
                    let mut param = CstNode::new(NodeKind::Param);
                    self.eat(&mut param); // (
                    self.collect_trivia(&mut param);
                    let tok = self.peek_or_eof(&[Token::Int, Token::Float, Token::Infinity])?;
                    if matches!(tok.token, Token::Int | Token::Float | Token::Infinity) {
                        self.eat(&mut param);
                    } else {
                        return Err(Diagnostic::parse(
                            ParseErrorKind::UnexpectedToken {
                                expected: vec![Token::Int, Token::Float, Token::Infinity],
                                found: tok.token.clone(),
                            },
                            tok.span.clone(),
                        ));
                    }

                    // Handle optional FIX/FIXED inside parens
                    if let Some(t) = self.peek_non_trivia()
                        && t.token == Token::Symbol
                        && (t.text.eq_ignore_ascii_case("FIX")
                            || t.text.eq_ignore_ascii_case("FIXED"))
                    {
                        self.collect_trivia(&mut param);
                        let mut flag = CstNode::new(NodeKind::Flag);
                        self.eat(&mut flag);
                        param.children.push(CstChild::Node(flag));
                    }

                    self.collect_trivia(&mut param);
                    self.expect(Token::RightParen, &mut param)?;
                    self.maybe_parse_repeat(&mut param);
                    node.children.push(CstChild::Node(param));
                }
                Token::Symbol => {
                    let upper = tok.text.to_uppercase();
                    match upper.as_str() {
                        // Flags can appear before, after, or interleaved with values
                        "FIX" | "FIXED" | "CORR" | "CORRELATION" | "SD" | "CHOLESKY"
                        | "VARIANCE" | "STANDARD" | "COVARIANCE" | "UNINT" => {
                            let mut flag = CstNode::new(NodeKind::Flag);
                            self.eat(&mut flag);
                            node.children.push(CstChild::Node(flag));
                        }
                        // SAME or SAME(m)
                        "SAME" => {
                            let mut same = CstNode::new(NodeKind::Same);
                            self.eat(&mut same);
                            // SAME(m) permitted since NONMEM 7.3
                            if matches!(self.peek_non_trivia(), Some(t) if t.token == Token::LeftParen)
                            {
                                self.collect_trivia(&mut same);
                                self.eat(&mut same); // (
                                self.collect_trivia(&mut same);
                                self.expect(Token::Int, &mut same)?;
                                self.collect_trivia(&mut same);
                                self.expect(Token::RightParen, &mut same)?;
                            }
                            node.children.push(CstChild::Node(same));
                        }
                        "NAMES" => {
                            node.children.push(CstChild::Node(self.parse_names()?));
                        }
                        "VALUES" => {
                            node.children.push(CstChild::Node(self.parse_values()?));
                        }
                        _ => {
                            // label=value(s)
                            let mut param = CstNode::new(NodeKind::Param);
                            self.eat(&mut param);
                            self.collect_trivia(&mut param);
                            self.expect(Token::Equals, &mut param)?;
                            self.collect_trivia(&mut param);
                            // Then one or more numbers
                            while matches!(self.peek_non_trivia(), Some(t) if matches!(t.token, Token::Int | Token::Float | Token::Infinity))
                            {
                                self.collect_trivia(&mut param);
                                self.eat(&mut param);
                            }

                            node.children.push(CstChild::Node(param));
                        }
                    }
                }
                _ => {
                    return Err(Diagnostic::parse(
                        ParseErrorKind::Message(format!(
                            "unexpected token '{}' in $OMEGA/$SIGMA",
                            tok.text
                        )),
                        tok.span.clone(),
                    ));
                }
            }
        }

        self.collect_trivia(&mut node);

        Ok(node)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.10.%20$OMEGA%20Record
    fn parse_omega(&mut self) -> Result<CstNode, Diagnostic> {
        self.parse_omega_sigma(NodeKind::Omega)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.11.%20$SIGMA%20Record
    fn parse_sigma(&mut self) -> Result<CstNode, Diagnostic> {
        self.parse_omega_sigma(NodeKind::Sigma)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::Lowerer;
    use insta::{assert_snapshot, glob};

    #[test]
    fn can_parse_mod_files() {
        glob!("../test_data/", "*.mod", |path| {
            let input = fs_err::read_to_string(path).unwrap();
            let parser = Parser::new(&input);
            let (cst, tokens, _source) = parser.parse().unwrap();
            assert_snapshot!(cst.debug_tree(&tokens));
        });
    }

    #[test]
    fn parse_errors() {
        glob!("../test_data/errors/", "*.mod", |path| {
            let input = fs_err::read_to_string(path).unwrap();
            let display_path = std::path::Path::new(path.file_name().unwrap());
            let parser = Parser::new(&input);
            match parser.parse() {
                Err(diag) => {
                    let source = input.replace("\r\n", "\n");
                    assert_snapshot!(diag.render(display_path, &source));
                }
                Ok((cst, tokens, source)) => {
                    let lowerer = Lowerer::new(tokens.as_slice());
                    let (_model, diagnostics) = lowerer.lower(&cst);
                    assert!(!diagnostics.is_empty(), "expected errors but got none");
                    let rendered: Vec<String> = diagnostics
                        .iter()
                        .map(|d| d.render(display_path, &source))
                        .collect();
                    assert_snapshot!(rendered.join("\n"));
                }
            }
        });
    }
}
