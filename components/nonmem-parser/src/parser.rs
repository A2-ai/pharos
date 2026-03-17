use std::fmt::Display;
use std::ops::Range;

use crate::cst::{CstChild, CstNode, NodeKind};
use crate::lexer;
use crate::lexer::{SpannedToken, Token};
use anyhow::Result;

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

#[derive(Debug, Clone)]
pub enum ParseErrorKind {
    UnexpectedToken { expected: Token, found: Token },
    UnexpectedEof,
    InvalidLabel { found: Token },
    Message(String),
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Option<Range<usize>>,
}
impl std::error::Error for ParseError {}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO
        f.write_fmt(format_args!("{:?}", self))
    }
}
impl ParseError {
    pub fn unexpected(found: &SpannedToken, expected: Token) -> Self {
        Self {
            kind: ParseErrorKind::UnexpectedToken {
                expected,
                found: found.token.clone(),
            },
            span: Some(found.span.clone()),
        }
    }

    pub fn invalid_label(found: &SpannedToken) -> Self {
        Self {
            kind: ParseErrorKind::InvalidLabel {
                found: found.token.clone(),
            },
            span: Some(found.span.clone()),
        }
    }

    pub fn message(s: &str, span: Option<Range<usize>>) -> Self {
        Self {
            kind: ParseErrorKind::Message(s.to_string()),
            span,
        }
    }

    pub fn eof() -> Self {
        Self {
            kind: ParseErrorKind::UnexpectedEof,
            span: None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Parser<'a> {
    idx: usize,
    input: &'a str,
    tokens: Vec<SpannedToken>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let tokens = lexer::lex(input);
        Self {
            idx: 0,
            input,
            tokens,
        }
    }

    pub fn parse(mut self) -> Result<(CstNode, Vec<SpannedToken>)> {
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
                        _ => {
                            let mut unknown = CstNode::new(NodeKind::UnknownRecord);
                            println!("{record_name} not handled");
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
                _ => todo!("{:?}", self.peek()),
            }
        }

        Ok((root, self.tokens))
    }

    fn peek(&self) -> Option<&SpannedToken> {
        self.tokens.get(self.idx)
    }

    fn peek_or_eof(&self) -> Result<&SpannedToken> {
        if let Some(t) = self.peek() {
            Ok(t)
        } else {
            Err(ParseError::eof().into())
        }
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

    fn expect(&mut self, expected: Token, node: &mut CstNode) -> Result<usize, ParseError> {
        self.collect_trivia(node);
        match self.peek() {
            Some(tok) if tok.token == expected => {
                let idx = self.idx;
                self.eat(node);
                Ok(idx)
            }
            Some(tok) => Err(ParseError::unexpected(tok, expected)),
            None => Err(ParseError::eof()),
        }
    }

    fn advance(&mut self) -> &SpannedToken {
        let tok = &self.tokens[self.idx];
        self.idx += 1;
        tok
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

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.1.%20$PROBLEM%20Record
    fn parse_problem(&mut self) -> Result<CstNode> {
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
    fn parse_input(&mut self) -> Result<CstNode> {
        let mut node = CstNode::new(NodeKind::Input);
        self.eat(&mut node);

        while !self.at_end_of_record() {
            self.collect_trivia(&mut node);

            let tok = self.peek_or_eof()?;
            // Commas can be used to separate items
            if tok.token == Token::Comma {
                self.eat(&mut node);
                continue;
            }

            if tok.token != Token::Symbol {
                return Err(ParseError::unexpected(tok, Token::Symbol).into());
            }
            if !is_valid_label(&tok.text) {
                return Err(ParseError::invalid_label(tok).into());
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
                            return Err(ParseError::invalid_label(t).into());
                        }
                        self.eat(&mut col_node);
                    }
                    other => {
                        return Err(match other {
                            Some(t) => ParseError::unexpected(t, Token::Symbol).into(),
                            None => ParseError::eof().into(),
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
    fn parse_data(&mut self) -> Result<CstNode> {
        let mut node = CstNode::new(NodeKind::Data);
        self.eat(&mut node);
        self.collect_trivia(&mut node);

        // First non-trivia is the filename/path which can be quoted or not
        match self.peek().map(|t| &t.token) {
            Some(Token::QuotedString | Token::Symbol) => self.eat(&mut node),
            // TODO: better error message here, write it ourselves
            _ => return Err(ParseError::eof().into()),
        }

        // Then we have a bunch of options, we don't care about all of them tbh
        while !self.at_end_of_record() {
            self.collect_trivia(&mut node);

            match self.peek_or_eof()? {
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
                                                return Err(ParseError::eof().into());
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
                                    return Err(ParseError::unexpected(t, Token::LeftParen).into());
                                }
                                _ => {
                                    return Err(ParseError::eof().into());
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
                                    return Err(ParseError::message(
                                        "Unexpected token, expected an integer, a float or a name",
                                        Some(t.span.clone()),
                                    )
                                    .into());
                                }
                                _ => {
                                    return Err(ParseError::eof().into());
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
                            return Err(ParseError::message(
                                "unknown keyword",
                                Some(tok.span.clone()),
                            )
                            .into());
                        }
                    }
                }
                tok => {
                    return Err(ParseError::unexpected(tok, Token::Symbol).into());
                }
            }
        }

        Ok(node)
    }

    /// Parse NAMES(...)
    fn parse_names(&mut self) -> Result<CstNode> {
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
                _ => return Err(ParseError::message("expected a name, comma, or )", None).into()),
            }
        }

        Ok(node)
    }

    fn parse_values(&mut self) -> Result<CstNode> {
        let mut node = CstNode::new(NodeKind::ParamValues);
        self.eat(&mut node);
        self.collect_trivia(&mut node);
        self.expect(Token::LeftParen, &mut node)?;

        loop {
            self.collect_trivia(&mut node);
            let tok = self.peek_or_eof()?;
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
                    return Err(
                        ParseError::message("number, comma, or )", Some(tok.span.clone())).into(),
                    );
                }
            }
        }

        Ok(node)
    }

    fn parse_params_parens(&mut self) -> Result<CstNode> {
        let mut node = CstNode::new(NodeKind::Parens);
        self.eat(&mut node);

        loop {
            self.collect_trivia(&mut node);
            let tok = self.peek_or_eof()?;
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
                // numbers, commas, FIX, INF, etc.
                _ => {
                    return Err(ParseError::message(
                        "expected ), a number, a comma or FIX/FIXED",
                        Some(tok.span.clone()),
                    )
                    .into());
                }
            }
        }

        Ok(node)
    }

    /// Parses simple key and key-val, eg $subroutines or $tables or $est
    fn parse_simple_options(&mut self, kind: NodeKind) -> Result<CstNode> {
        let mut node = CstNode::new(kind);
        self.eat(&mut node);

        while !self.at_end_of_record() {
            self.collect_trivia(&mut node);
            let tok = self.peek_or_eof()?;
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
                                self.peek_or_eof()?.token,
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
    fn parse_subroutines(&mut self) -> Result<CstNode> {
        self.parse_simple_options(NodeKind::Subroutines)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.14.%20$ESTIMATION%20Record
    fn parse_estimation(&mut self) -> Result<CstNode> {
        self.parse_simple_options(NodeKind::Estimation)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.16.%20$TABLE%20Record
    fn parse_table(&mut self) -> Result<CstNode> {
        self.parse_simple_options(NodeKind::Table)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.13.%20$SIMULATION%20Record
    fn parse_simulation(&mut self) -> Result<CstNode> {
        self.parse_simple_options(NodeKind::Simulation)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.15.%20$COVARIANCE%20Record
    fn parse_covariance(&mut self) -> Result<CstNode> {
        self.parse_simple_options(NodeKind::Covariance)
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

    fn parse_bounds(&mut self) -> Result<CstNode> {
        let mut param = CstNode::new(NodeKind::Param);
        self.eat(&mut param);
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
    fn parse_theta(&mut self) -> Result<CstNode> {
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

            let tok = self.peek_or_eof()?;
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

                    let next = self.peek_or_eof()?;
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
                            return Err(ParseError::message(
                                "expected ( or a number",
                                Some(next.span.clone()),
                            )
                            .into());
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
                    return Err(
                        ParseError::message("unexpected token", Some(tok.span.clone())).into(),
                    );
                }
            }
        }

        self.collect_trivia(&mut node);
        Ok(node)
    }

    fn parse_omega_sigma(&mut self, kind: NodeKind) -> Result<CstNode> {
        let mut node = CstNode::new(kind);
        self.eat(&mut node);
        self.collect_trivia(&mut node);

        // BLOCK must come first if present
        let tok = self.peek_or_eof()?;
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
            let tok = self.peek_or_eof()?;

            match &tok.token {
                Token::Int | Token::Float | Token::Infinity => {
                    let mut param = CstNode::new(NodeKind::Param);
                    self.eat(&mut param);
                    self.maybe_parse_fix(&mut param);
                    node.children.push(CstChild::Node(param));
                }
                Token::LeftParen => {
                    // OMEGA/SIGMA do not have bounds so it's just a number with potentially xN syntax after
                    let mut param = CstNode::new(NodeKind::Param);
                    self.eat(&mut param);
                    self.collect_trivia(&mut param);
                    let tok = self.peek_or_eof()?;
                    if matches!(tok.token, Token::Int | Token::Float | Token::Infinity) {
                        self.eat(&mut param);
                    } else {
                        return Err(ParseError::message(
                            "number inside ()",
                            Some(tok.span.clone()),
                        )
                        .into());
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
                _ => unreachable!("unexpected token {tok:?}"),
            }
        }

        self.collect_trivia(&mut node);

        Ok(node)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.10.%20$OMEGA%20Record
    fn parse_omega(&mut self) -> Result<CstNode> {
        self.parse_omega_sigma(NodeKind::Omega)
    }

    // https://nmhelp.tingjieguo.com/IV/III#III.III.III.B.11.%20$SIGMA%20Record
    fn parse_sigma(&mut self) -> Result<CstNode> {
        self.parse_omega_sigma(NodeKind::Sigma)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::{assert_snapshot, glob};

    #[test]
    fn can_parse_mod_files() {
        glob!("../test_data/", "*.mod", |path| {
            let input = fs_err::read_to_string(path).unwrap();
            let parser = Parser::new(&input);
            let (cst, tokens) = parser.parse().unwrap();
            assert_snapshot!(cst.debug_tree(&tokens));
        });
    }
}
