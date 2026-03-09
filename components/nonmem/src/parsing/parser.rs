use crate::estimation::EstimationMethod;
use crate::parsing::comments::ParamName;
use crate::parsing::errors::SyntaxError;
use crate::parsing::lexer::ControlRecord;
use crate::parsing::model::{
    BlockStructure, ComparisonOperator, Covariance, Data, DataFilter, DataValueFilter,
    DataValueFilterKind, Estimation, InputColumn, Parameter, ParameterBlock, Parameterization,
    Simulation, Subroutine,
};
use crate::parsing::utils::{Span, Spanned};
use crate::parsing::{Model, Token, lex};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone, PartialEq)]
pub(crate) struct Parser {
    tokens: Vec<Spanned<Token>>,
    index: isize,
    current_span: Span,
    model: Model,
}

// As a macro so we can use the matches! macro properly
macro_rules! expect {
    ($parser:expr, $match:pat, $expectation:expr) => {{
        match $parser.next_non_trivia_or_error()? {
            (token, span) if matches!(token, $match) => Ok((token, span)),
            (token, _) => Err(SyntaxError::new(
                format!("Found {:?} but expected {}.", token, $expectation),
                &$parser.current_span,
            )),
        }
    }};
    ($parser:expr, $match:pat => $target:expr, $expectation:expr) => {{
        match $parser.next_non_trivia_or_error()? {
            ($match, span) => Ok(($target, span)),
            (token, _) => Err(SyntaxError::new(
                format!("Found {:?} but expected {}.", token, $expectation),
                &$parser.current_span,
            )),
        }
    }};
}

impl Parser {
    pub fn new(input: &str) -> Result<Self, SyntaxError> {
        let tokens = lex(input)?;
        Ok(Self {
            tokens,
            index: -1,
            current_span: Span::default(),
            model: Model::default(),
        })
    }

    fn consume_inline_comment(&mut self) -> Option<String> {
        let comment = self.peek_inline_comment_with_line().map(|(x, _)| x);
        if comment.is_some() {
            // Advance past whitespace and comment
            while let Some((token, _)) = self.next() {
                match token {
                    Token::Whitespace(_) => continue,
                    Token::Comment(_) => break,
                    _ => {
                        // We went too far, back up one
                        self.index -= 1;
                        break;
                    }
                }
            }
        }
        comment
    }

    fn peek_inline_comment_with_line(&self) -> Option<(String, usize)> {
        let mut index = self.index as usize;

        // Look ahead for whitespace then comment
        while index + 1 < self.tokens.len() {
            index += 1;

            if let Some(token) = self.tokens.get(index) {
                match token.node() {
                    Token::Whitespace(ws) => {
                        if ws.contains("\n") {
                            break;
                        }
                    } // skip whitespace
                    Token::Comment(comment) => {
                        return Some((comment.trim().to_string(), token.span().start_line));
                    }
                    _ => break, // found non-whitespace, non-comment token
                }
            }
        }

        None
    }

    fn next(&mut self) -> Option<(&Token, &Span)> {
        self.index += 1;

        if let Some(token) = self.tokens.get(self.index as usize) {
            self.current_span = token.span().clone();
            Some((token.node(), token.span()))
        } else {
            None
        }
    }

    fn peek_non_trivia(&self) -> Option<(&Token, &Span)> {
        let mut index = self.index as usize;

        while index + 1 < self.tokens.len() {
            index += 1;

            if let Some(token) = self.tokens.get(index)
                && !token.is_trivia()
            {
                return Some((token.node(), token.span()));
            }
        }

        None
    }

    fn next_non_trivia(&mut self) -> Option<(Token, Span)> {
        loop {
            self.next();

            if self.index as usize >= self.tokens.len() {
                return None;
            }
            let token = &self.tokens[self.index as usize];
            if token.is_trivia() {
                continue;
            } else {
                return Some((token.node().clone(), token.span().clone()));
            }
        }
    }

    fn next_non_trivia_or_error(&mut self) -> Result<(Token, Span), SyntaxError> {
        let mut current_span = self.current_span.clone();
        match self.next_non_trivia() {
            None => {
                // The EOI is after the current span
                current_span.start_col = current_span.end_col;
                current_span.start_line = current_span.end_line;
                Err(SyntaxError::new(
                    "Unexpected end of input".to_string(),
                    &current_span,
                ))
            }
            Some((t, s)) => Ok((t.clone(), s.clone())),
        }
    }

    fn next_or_error(&mut self) -> Result<(&Token, &Span), SyntaxError> {
        let mut current_span = self.current_span.clone();
        match self.next() {
            None => {
                // The EOI is after the current span
                current_span.start_col = current_span.end_col;
                current_span.start_line = current_span.end_line;
                Err(SyntaxError::new(
                    "Unexpected end of input".to_string(),
                    &current_span,
                ))
            }
            Some(c) => Ok(c),
        }
    }

    fn parse_input_block(&mut self) -> Result<Vec<InputColumn>, SyntaxError> {
        let mut out = Vec::new();

        while let Some((peeked, _)) = self.peek_non_trivia()
            && !matches!(peeked, Token::ControlRecord { .. })
        {
            let (token, span) = self.next_non_trivia_or_error()?;

            match token {
                Token::Identifier(ident) => {
                    // Check for = (alias or drop)
                    if let Some((Token::Equals, _)) = self.peek_non_trivia() {
                        self.next_non_trivia_or_error()?;
                        let (token, span) = self.next_or_error()?;
                        match token {
                            Token::Identifier(original) => {
                                out.push(InputColumn::Aliased {
                                    from: original.to_string(),
                                    to: ident,
                                });
                            }
                            Token::Keyword(kw)
                                if kw.eq_ignore_ascii_case("DROP")
                                    || kw.eq_ignore_ascii_case("SKIP") =>
                            {
                                out.push(InputColumn::Dropped(ident));
                            }
                            _ => {
                                return Err(SyntaxError::new(
                                    format!(
                                        "Expected an identifier or DROP/SKIP keyword but found {}",
                                        token.name()
                                    ),
                                    span,
                                ));
                            }
                        }
                    } else {
                        out.push(InputColumn::Included(ident));
                    }
                }
                Token::Keyword(kw)
                    if kw.eq_ignore_ascii_case("DROP") || kw.eq_ignore_ascii_case("SKIP") =>
                {
                    // Standalone DROP/SKIP - unnamed dropped column
                    out.push(InputColumn::Dropped(String::new()));
                }
                _ => {
                    return Err(SyntaxError::new(
                        format!(
                            "Expected an identifier or DROP/SKIP keyword but found {}",
                            token.name()
                        ),
                        &span,
                    ));
                }
            }

            // Optionally consume comma separators between parameters
            if let Some((Token::Comma, _)) = self.peek_non_trivia() {
                self.next_non_trivia_or_error()?;
            }
        }

        Ok(out)
    }

    fn parse_data_block(&mut self) -> Result<Data, SyntaxError> {
        let mut data = Data::default();

        let (token, span) = self.next_non_trivia_or_error()?;
        data.path = match token {
            Token::Identifier(s) => s,
            Token::QuotedString(s) => s,
            _ => {
                return Err(SyntaxError::new(
                    format!("Expected dataset path, found {}", token.name()),
                    &span,
                ));
            }
        };

        macro_rules! parse_filters {
            ($container:expr) => {{
                let (token, span) = self.next_non_trivia_or_error()?;
                match token {
                    Token::Identifier(ident) => {
                        $container.push(DataFilter::Marker(ident.clone()));
                    }
                    Token::LeftParen => {
                        loop {
                            let (token, span) = self.next_non_trivia_or_error()?;
                            match token {
                                Token::Identifier(ident) => {
                                    // Handle all filter formats (compact, spaced, mixed) in one place
                                    $container.push(self.parse_any_filter_format(ident.clone())?);
                                }
                                Token::Comma => continue,
                                Token::RightParen => break,
                                _ => {
                                    return Err(SyntaxError::new(
                                        format!(
                                            "Unexpected token {:?}, expected a name, a comma or a right parenthesis",
                                            token.name()
                                        ),
                                        &span,
                                    ));
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(SyntaxError::new(
                            format!(
                                "Expected an identifier or a left parenthesis, found {}",
                                token.name()
                            ),
                            &span,
                        ));
                    }
                }
            }};
        }

        while let Some((peeked, _)) = self.peek_non_trivia()
            && !matches!(peeked, Token::ControlRecord { .. })
        {
            // then we should have a keyword
            let keyword = expect!(self, Token::Keyword(s) => s.to_string(), "keyword")?.0;
            // then maybe an equal sign
            if let Some((Token::Equals, _)) = self.peek_non_trivia() {
                expect!(self, Token::Equals, "an equal sign")?;
            }

            match keyword.to_ascii_uppercase().as_str() {
                "IGNORE" | "IGN" => {
                    parse_filters!(data.ignore);
                }
                "ACCEPT" => {
                    parse_filters!(data.accept);
                }
                "RECORDS" => {
                    let value = expect!(self, Token::Number {value, ..} => value, "a number")?.0;
                    data.num_records = Some(value as usize);
                }
                "NULL" => {
                    let (token, span) = self.next_non_trivia_or_error()?;
                    match token {
                        Token::Identifier(s) | Token::Keyword(s) => {
                            data.null_value = Some(s);
                        }
                        _ => {
                            return Err(SyntaxError::new(
                                format!("Expected a character for NULL, found {}", token.name()),
                                &span,
                            ));
                        }
                    }
                }
                _ => {
                    // we ignore other keywords and just consume whatever is after
                    self.next_non_trivia_or_error()?;
                }
            }
        }

        Ok(data)
    }

    /// Parse NAMES(...) syntax if present, returning the list of names.
    /// Consumes the NAMES keyword and parenthesized list if found.
    fn parse_names_list(&mut self) -> Result<Vec<String>, SyntaxError> {
        let mut names = Vec::new();

        if let Some((Token::Keyword(kw), _)) = self.peek_non_trivia() {
            if kw.eq_ignore_ascii_case("NAMES") {
                self.next_non_trivia_or_error()?; // consume NAMES
                expect!(self, Token::LeftParen, "(")?;
                // Parse comma-separated names until )
                loop {
                    let (token, _) = self.next_non_trivia_or_error()?;
                    match token {
                        Token::Identifier(name) | Token::Keyword(name) => {
                            names.push(name);
                        }
                        Token::Comma => continue,
                        Token::RightParen => break,
                        _ => {
                            return Err(SyntaxError::new(
                                format!(
                                    "Expected identifier, comma, or ) in NAMES list, got {}",
                                    token.name()
                                ),
                                &self.current_span,
                            ));
                        }
                    }
                }
            }
        }

        Ok(names)
    }

    fn parse_parameters<T: ParamName>(
        &mut self,
    ) -> Result<Vec<(Parameter<T>, usize)>, SyntaxError> {
        let mut out = Vec::new();
        // (param_index, line_number)
        let mut parameters_with_lines = Vec::new();

        // Check for NAMES(...) syntax at the start
        let names = self.parse_names_list()?;
        let mut name_index = 0;

        while let Some((peeked, _)) = self.peek_non_trivia()
            && !matches!(peeked, Token::ControlRecord { .. })
        {
            let (token, span) = self.next_non_trivia_or_error()?;
            let param_line = span.start_line;

            macro_rules! add_comment {
                () => {{
                    // Check if there's a comment after this parameter on the same line
                    if let Some((comment_text, comment_line)) = self.peek_inline_comment_with_line()
                    {
                        // Assign comment to all parameters on the same line or the comment line
                        // since we might define multiple param in a single line
                        for &(idx, line) in &parameters_with_lines {
                            if line == param_line || line == comment_line {
                                out[idx].0.comment = Some(comment_text.clone());
                            }
                        }
                        self.consume_inline_comment();
                    }
                }};
            }

            // Check for named parameter syntax: NAME=(...)
            let (param_name, token) = if let Token::Identifier(ident) = &token
                && let Some((Token::Equals, _)) = self.peek_non_trivia()
            {
                let name = ident.clone();
                self.next_non_trivia_or_error()?; // consume =
                name_index += 1; // consume NAMES slot even when individual name used
                let token = self.next_non_trivia_or_error()?.0;
                (Some(name), token)
            } else {
                // Don't assign name here - let each branch handle it appropriately
                // Token::Number handles its own name lookup
                // Token::LeftParen defers to the repeat loop for xN support
                (None, token)
            };

            match token {
                Token::Number { value, .. } => {
                    let is_fixed = if let Some((Token::Keyword(kw), _)) = self.peek_non_trivia() {
                        kw.eq_ignore_ascii_case("FIX") || kw.eq_ignore_ascii_case("FIXED")
                    } else {
                        false
                    };

                    if is_fixed {
                        self.next_non_trivia_or_error()?;
                    }

                    let param_index = out.len();
                    parameters_with_lines.push((param_index, param_line));

                    let final_name = if param_name.is_some() {
                        param_name
                    } else {
                        let n = names.get(name_index).cloned();
                        if n.is_some() {
                            name_index += 1;
                        }
                        n
                    };

                    out.push((
                        Parameter {
                            name: final_name,
                            lower_bound: None,
                            initial_value: value,
                            upper_bound: None,
                            is_fixed,
                            comment: None,
                            parsed_comment: None,
                        },
                        self.index as usize,
                    ));
                    add_comment!();
                }
                Token::LeftParen => {
                    let mut values = Vec::new();
                    let mut values_indices = Vec::new();
                    let mut is_fixed = false;

                    loop {
                        let (token, span) = self.next_non_trivia_or_error()?;
                        match token {
                            Token::Number { value, .. } => {
                                values.push(value);
                                values_indices.push(self.index as usize);
                            }
                            Token::Keyword(kw)
                                if kw.eq_ignore_ascii_case("INF")
                                    || kw.eq_ignore_ascii_case("INFINITY") =>
                            {
                                values.push(1_000_000.0);
                                values_indices.push(self.index as usize);
                            }
                            Token::Identifier(ident)
                                if ident.eq_ignore_ascii_case("-INF")
                                    || ident.eq_ignore_ascii_case("-INFINITY") =>
                            {
                                values.push(-1_000_000.0);
                                values_indices.push(self.index as usize);
                            }
                            Token::Keyword(kw)
                                if kw.eq_ignore_ascii_case("FIX")
                                    || kw.eq_ignore_ascii_case("FIXED") =>
                            {
                                is_fixed = true;
                            }
                            Token::Comma => (),
                            Token::RightParen => break,
                            _ => {
                                return Err(SyntaxError::new(
                                    format!(
                                        "Expected a number, FIX, a comma or a right parenthesis but got {}",
                                        token.name()
                                    ),
                                    &span,
                                ));
                            }
                        }
                    }

                    let param = match values.len() {
                        1 => (
                            Parameter {
                                name: param_name.clone(),
                                lower_bound: None,
                                initial_value: values[0],
                                upper_bound: None,
                                is_fixed,
                                comment: None,
                                parsed_comment: None,
                            },
                            values_indices[0],
                        ),
                        2 => (
                            Parameter {
                                name: param_name.clone(),
                                lower_bound: Some(values[0]),
                                initial_value: values[1],
                                upper_bound: None,
                                is_fixed,
                                comment: None,
                                parsed_comment: None,
                            },
                            values_indices[1],
                        ),
                        3 => (
                            Parameter {
                                name: param_name.clone(),
                                lower_bound: Some(values[0]),
                                initial_value: values[1],
                                upper_bound: Some(values[2]),
                                is_fixed,
                                comment: None,
                                parsed_comment: None,
                            },
                            values_indices[1],
                        ),
                        _ => {
                            return Err(SyntaxError::new(
                                format!(
                                    "Invalid parameter, got too many values: {values:?}. Expected 1, 2 or 3 numbers."
                                ),
                                &self.current_span,
                            ));
                        }
                    };

                    // Check for xN repeat syntax (e.g., (0.1)x5)
                    let repeat_count = if let Some((Token::Identifier(ident), span)) =
                        self.peek_non_trivia()
                    {
                        let ident_lower = ident.to_lowercase();
                        if ident_lower.starts_with('x') {
                            if let Ok(n) = ident_lower[1..].parse::<usize>()
                                && n > 0
                            {
                                self.next_non_trivia_or_error()?; // consume the xN token
                                n
                            } else {
                                return Err(SyntaxError::new(
                                        "Repeat count in xN syntax must be an integer greater than zero."
                                            .to_string(),
                                        span,
                                    ));
                            }
                        } else {
                            1
                        }
                    } else {
                        1
                    };

                    for i in 0..repeat_count {
                        let current_name = if i == 0 && param_name.is_some() {
                            // First param gets the explicit name (e.g., CL from CL=(0,1)x2)
                            // Note: name_index already incremented before
                            param_name.clone()
                        } else if let Some(name) = names.get(name_index).cloned() {
                            // Subsequent params (or first if no explicit name) get NAMES
                            name_index += 1;
                            Some(name)
                        } else {
                            None
                        };
                        let idx = out.len();
                        let mut new_param = param.0.clone();
                        new_param.name = current_name;
                        parameters_with_lines.push((idx, param_line));
                        out.push((new_param, param.1));
                    }

                    add_comment!();
                }
                _ => {
                    return Err(SyntaxError::new(
                        format!(
                            "Expected a number of a left parenthesis, got a {} instead",
                            token.name()
                        ),
                        &span,
                    ));
                }
            }
        }

        Ok(out)
    }

    fn parse_any_filter_format(&mut self, first_token: String) -> Result<DataFilter, SyntaxError> {
        // Track spans for the 3 essential parts: field, operator, value
        let field_span = self.current_span.clone();
        let mut op_span = None;
        let mut value_span = None;

        // Collect tokens for reconstruction
        let mut collected_tokens = vec![first_token];

        loop {
            match self.peek_non_trivia() {
                Some((Token::Comma, _)) | Some((Token::RightParen, _)) | None => break,
                _ => {
                    let (token, span) = self.next_non_trivia_or_error()?;
                    match token {
                        Token::Identifier(s) | Token::Keyword(s) => {
                            // This could be an operator or part of a combined token
                            if op_span.is_none() && s != "." {
                                op_span = Some(span);
                            }
                            collected_tokens.push(s);
                        }
                        Token::Number { original, .. } => {
                            // This is likely the value
                            if value_span.is_none() {
                                value_span = Some(span);
                            }
                            collected_tokens.push(original);
                        }
                        _ => {
                            // Skip dots and other tokens, but still reconstruct them
                            collected_tokens.push(format!("{}", token));
                        }
                    }
                }
            }
        }

        // Reconstruct as single identifier without spaces
        let reconstructed = collected_tokens.join("");

        // Parse using compact format logic
        let parts = reconstructed.splitn(3, '.').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(SyntaxError::new(
                format!("Invalid data filter: {reconstructed}"),
                &field_span,
            ));
        }

        let field = parts[0].to_string();

        let op = match ComparisonOperator::from_str(parts[1]) {
            Ok(op) => op,
            Err(e) => {
                let error_span = if let Some(span) = op_span {
                    // Multi-token case: use the operator token's span
                    span
                } else {
                    // Single token case: calculate span within the identifier
                    let mut span_op = field_span.clone();
                    span_op.start_col += parts[0].len() + 1; // Skip "FIELD."
                    span_op.end_col = span_op.start_col + parts[1].len(); // Point to "OP"
                    span_op
                };
                return Err(SyntaxError::new(e, &error_span));
            }
        };

        let value = {
            let value_str = parts[2];
            if value_str.starts_with('"') && value_str.ends_with('"') && value_str.len() >= 2 {
                // Handle quoted string value like "C"
                let string_content = &value_str[1..value_str.len() - 1]; // Strip quotes
                DataValueFilterKind::String(string_content.to_string())
            } else {
                // Try to parse as number
                match value_str.parse::<f64>() {
                    Ok(num) => DataValueFilterKind::Number(num),
                    Err(_) => {
                        let error_span = if let Some(span) = value_span {
                            // Multi-token case: use the value token's span
                            span
                        } else {
                            // Single token case: calculate span within the identifier
                            let mut span_val = field_span.clone();
                            span_val.start_col += reconstructed.len() - parts[2].len(); // Point to value part
                            span_val.end_col = span_val.start_col + parts[2].len();
                            span_val
                        };
                        return Err(SyntaxError::new(
                            format!(
                                "Invalid value in data filter: '{}' is neither a number nor a quoted string",
                                parts[2]
                            ),
                            &error_span,
                        ));
                    }
                }
            }
        };

        Ok(DataFilter::ValueFilter(DataValueFilter {
            field,
            op,
            value,
        }))
    }

    /// Parse a single option: either KEY=VALUE or just KEY (flag)
    /// Returns Some((key, value)) where value is None for flags, or None if token isn't a keyword/identifier
    fn parse_option(
        &mut self,
        token: &Token,
    ) -> Result<Option<(String, Option<String>)>, SyntaxError> {
        let key = match token {
            Token::Keyword(kw) => kw.to_uppercase(),
            Token::Identifier(id) => id.to_uppercase(),
            _ => return Ok(None),
        };

        if matches!(self.peek_non_trivia(), Some((Token::Equals, _))) {
            self.next_non_trivia_or_error()?; // consume '='
            let (value_token, _) = self.next_non_trivia_or_error()?;
            let value = match value_token {
                Token::Number { original, .. } => original,
                Token::Keyword(v) | Token::Identifier(v) => v,
                Token::QuotedString(v) => v,
                _ => return Ok(Some((key, None))), // treat as flag if value is weird
            };
            Ok(Some((key, Some(value))))
        } else {
            Ok(Some((key, None)))
        }
    }

    #[allow(clippy::type_complexity)]
    fn parse_estimation(
        &mut self,
    ) -> Result<(Estimation, (Option<usize>, Option<usize>)), SyntaxError> {
        let mut estimation = Estimation::default();
        let mut file_idx = None;
        let mut msfo_idx = None;

        while let Some((peeked, _)) = self.peek_non_trivia()
            && !matches!(peeked, Token::ControlRecord { .. })
        {
            let (token, _) = self.next_non_trivia_or_error()?;

            let key = match &token {
                Token::Keyword(kw) => Some(kw.to_uppercase()),
                Token::Identifier(id) => Some(id.to_uppercase()),
                _ => None,
            };

            if let Some(key) = key {
                match key.as_str() {
                    "METHOD" => {
                        expect!(self, Token::Equals, "equal sign")?;
                        let (token2, span) = self.next_non_trivia_or_error()?;
                        match token2 {
                            Token::Number { value, .. } => {
                                estimation.method =
                                    EstimationMethod::from_str(&(value as usize).to_string())
                                        .map_err(|e| SyntaxError::new(e, &span))?;
                            }
                            Token::Keyword(kw2) => {
                                estimation.method = EstimationMethod::from_str(&kw2)
                                    .map_err(|e| SyntaxError::new(e, &span))?;
                            }
                            _ => {
                                return Err(SyntaxError::new(
                                    "Invalid ESTIMATION METHOD value.".to_string(),
                                    &span,
                                ));
                            }
                        }
                    }
                    "MSFO" => {
                        expect!(self, Token::Equals, "equal sign")?;
                        let (path, _) = expect!(self, Token::Identifier(s) => s, "file path")?;
                        msfo_idx = Some(self.index as usize);
                        estimation.msfo = Some(PathBuf::from(path));
                    }
                    "FILE" => {
                        expect!(self, Token::Equals, "equal sign")?;
                        let (path, _) = expect!(self, Token::Identifier(s) => s, "file path")?;
                        file_idx = Some(self.index as usize);
                        estimation.file = Some(PathBuf::from(path));
                    }
                    _ => {
                        if let Some((k, v)) = self.parse_option(&token)? {
                            estimation.options.insert(k, v);
                        }
                    }
                }
            }
        }

        Ok((estimation, (file_idx, msfo_idx)))
    }

    fn parse_simulation(&mut self) -> Result<Simulation, SyntaxError> {
        let mut simulation = Simulation::default();

        while let Some((peeked, _)) = self.peek_non_trivia()
            && !matches!(peeked, Token::ControlRecord { .. })
        {
            let (token, _) = self.next_non_trivia_or_error()?;
            if let Some((key, value)) = self.parse_option(&token)? {
                simulation.options.insert(key, value);
            }
        }

        Ok(simulation)
    }

    fn parse_covariance(&mut self) -> Result<Covariance, SyntaxError> {
        let mut covariance = Covariance::default();

        while let Some((peeked, _)) = self.peek_non_trivia()
            && !matches!(peeked, Token::ControlRecord { .. })
        {
            let (token, _) = self.next_non_trivia_or_error()?;
            if let Some((key, value)) = self.parse_option(&token)? {
                covariance.options.insert(key, value);
            }
        }

        Ok(covariance)
    }

    /// Shared helper method to parse block content after BLOCK(N) syntax
    /// Handles post-BLOCK keywords (CORR, SD, CHOLESKY, SAME, FIX, VALUES) and parameter parsing
    /// Returns (parameters, token_indices, final_parametrization, final_same_flag)
    #[allow(clippy::type_complexity)]
    fn parse_block_content<T: ParamName>(
        &mut self,
        size: usize,
        initial_parametrization: Option<Parameterization>,
        initial_same: bool,
    ) -> Result<
        (
            Vec<Parameter<T>>,
            Vec<usize>,
            Option<Parameterization>,
            bool,
        ),
        SyntaxError,
    > {
        let mut parametrization = initial_parametrization;
        let mut same = initial_same;
        let mut block_fixed = false;
        let mut names: Vec<String> = Vec::new();

        // Parse additional keywords that can come after BLOCK(N)
        // These can appear in any order: CORR/SD/CHOLESKY, SAME, FIX, NAMES(...)
        loop {
            let Some((token, _)) = self.peek_non_trivia() else {
                break;
            };

            let kw = match token {
                Token::Keyword(k) => Some(k.clone()),
                Token::Identifier(k) => Some(k.clone()),
                _ => None,
            };

            let Some(kw) = kw else {
                break;
            };

            if let Some(param) = Parameterization::from_keyword(&kw) {
                self.next_non_trivia_or_error()?;
                parametrization = Some(param);
            } else if kw.eq_ignore_ascii_case("SAME") {
                self.next_non_trivia_or_error()?;
                same = true;
            } else if kw.eq_ignore_ascii_case("FIX") || kw.eq_ignore_ascii_case("FIXED") {
                self.next_non_trivia_or_error()?;
                block_fixed = true;
            } else if kw.eq_ignore_ascii_case("NAMES") {
                names = self.parse_names_list()?;
            } else if kw.eq_ignore_ascii_case("VALUES") {
                // VALUES is handled below in the parameter parsing section
                break;
            } else {
                // Unknown keyword, stop processing block keywords
                break;
            }
        }

        // If SAME was encountered, return empty parameters (BlockSame doesn't have its own parameters)
        if same {
            return Ok((Vec::new(), Vec::new(), parametrization, same));
        }

        let expected_count = size * (size + 1) / 2; // Lower triangular count

        // Parse parameters - either VALUES syntax or regular parameters
        let parameters = if let Some((token, _)) = self.peek_non_trivia()
            && matches!(token, Token::Keyword(kw) | Token::Identifier(kw) if kw.eq_ignore_ascii_case("VALUES"))
        {
            // Consume VALUES keyword
            self.next_non_trivia_or_error()?;

            // Parse VALUES (value1, value2, ...)
            expect!(self, Token::LeftParen, "left parenthesis after VALUES")?;
            let mut values = Vec::new();
            loop {
                let (token, span) = self.next_non_trivia_or_error()?;
                match token {
                    Token::Number { value, .. } => {
                        values.push(value);
                    }
                    Token::Comma => continue,
                    Token::RightParen => break,
                    _ => {
                        return Err(SyntaxError::new(
                            format!(
                                "Expected number, comma, or right parenthesis in VALUES, got {}",
                                token.name()
                            ),
                            &span,
                        ));
                    }
                }
            }

            // Expand values to full lower triangular matrix
            // NAMES correspond to diagonal elements (the ETAs)
            let mut expanded_params = Vec::new();
            let mut name_index = 0;

            for i in 0..size {
                for j in 0..=i {
                    let is_diagonal = i == j;
                    let value = if is_diagonal {
                        // Diagonal element - always use first value
                        values[0]
                    } else {
                        // Off-diagonal element - use second value if available, otherwise first
                        if values.len() > 1 {
                            values[1]
                        } else {
                            values[0]
                        }
                    };

                    // Only diagonal elements get names
                    let name = if is_diagonal {
                        let n = names.get(name_index).cloned();
                        name_index += 1;
                        n
                    } else {
                        None
                    };

                    expanded_params.push((
                        Parameter {
                            name,
                            lower_bound: None,
                            initial_value: value,
                            upper_bound: None,
                            is_fixed: block_fixed,
                            comment: None,
                            parsed_comment: None,
                        },
                        self.index as usize,
                    ));
                }
            }

            expanded_params
        } else {
            // Regular parameter parsing
            self.parse_parameters()?
        };

        if parameters.len() != expected_count {
            return Err(SyntaxError::new(
                format!(
                    "Expected {} parameters for BLOCK({}), got {}",
                    expected_count,
                    size,
                    parameters.len()
                ),
                &self.current_span,
            ));
        }

        let (mut params, indices): (Vec<_>, Vec<_>) = parameters.into_iter().unzip();

        if block_fixed && !params.iter().any(|p| p.is_fixed) {
            // Only set fixed if not already set by VALUES parsing
            for param in &mut params {
                param.is_fixed = true;
            }
        }

        Ok((params, indices, parametrization, same))
    }

    #[allow(clippy::type_complexity)]
    fn parse_omega_sigma<T: ParamName>(
        &mut self,
    ) -> Result<(Vec<ParameterBlock<T>>, Vec<Vec<usize>>), SyntaxError> {
        let mut out = Vec::new();
        let mut token_indices = Vec::new();

        let mut initial_parametrization = None;
        let mut initial_same = false;

        while let Some((peeked, _)) = self.peek_non_trivia()
            && !matches!(peeked, Token::ControlRecord { .. })
        {
            let token = peeked.clone();
            match token {
                // Handle parameterization keywords that come BEFORE BLOCK (e.g., $OMEGA CORRELATION BLOCK(2))
                // Only consume recognized keywords: parameterization (CORR, SD, etc.), SAME, or BLOCK
                Token::Keyword(kw) if Parameterization::from_keyword(&kw).is_some() => {
                    self.next_non_trivia_or_error()?;
                    initial_parametrization = Parameterization::from_keyword(&kw);
                }
                Token::Keyword(kw) if kw.eq_ignore_ascii_case("SAME") => {
                    self.next_non_trivia_or_error()?;
                    initial_same = true;
                }
                Token::Keyword(kw) if kw.eq_ignore_ascii_case("BLOCK") => {
                    self.next_non_trivia_or_error()?;
                    expect!(self, Token::LeftParen, "left parenthesis")?;
                    let (size, _) =
                        expect!(self, Token::Number {value, ..} => value as usize, "number")?;
                    expect!(self, Token::RightParen, "right parenthesis")?;

                    let (final_parameters, block_token_indices, final_parametrization, final_same) =
                        self.parse_block_content(size, initial_parametrization, initial_same)?;

                    // Determine structure based on whether SAME was encountered during parsing
                    let structure = if final_same {
                        BlockStructure::BlockSame { size }
                    } else {
                        BlockStructure::Block { size }
                    };

                    out.push(ParameterBlock {
                        structure,
                        parametrization: final_parametrization,
                        parameters: final_parameters,
                    });
                    token_indices.push(block_token_indices);

                    // Reset for next block
                    initial_parametrization = None;
                    initial_same = false;
                }
                // Diagonal parameters: numbers, parentheses, NAMES keyword, or identifiers (for NAME=... syntax)
                Token::Number { .. }
                | Token::LeftParen
                | Token::Keyword(_)
                | Token::Identifier(_) => {
                    let params = self.parse_parameters()?;
                    let (parameters, indices): (Vec<_>, Vec<_>) = params.into_iter().unzip();
                    out.push(ParameterBlock {
                        structure: BlockStructure::Diagonal,
                        parametrization: None,
                        parameters,
                    });
                    token_indices.push(indices);
                }
                _ => {
                    // Skip unknown tokens to avoid infinite loop
                    self.next_non_trivia_or_error()?;
                }
            }
        }

        Ok((out, token_indices))
    }

    pub fn parse(&mut self) -> Result<Model, SyntaxError> {
        // at the top level we should have Control Records only
        while let Some((token, _)) = self.next_non_trivia() {
            match token {
                Token::ControlRecord { kind, .. } => match kind {
                    ControlRecord::Problem => {
                        let (problem, _) = expect!(self, Token::Ignored(s) => s, "problem name")?;
                        self.model.problem = problem.trim().to_string();
                        self.model.token_ranges.problem_content = Some(self.index as usize);
                    }
                    ControlRecord::Input => {
                        let input = self.parse_input_block()?;
                        self.model.input_columns = input;
                    }
                    ControlRecord::Data => {
                        let data = self.parse_data_block()?;
                        self.model.data = data;
                    }
                    ControlRecord::Subroutine => {
                        while let Some((peeked, _)) = self.peek_non_trivia()
                            && matches!(peeked, Token::Keyword(_))
                        {
                            let kw = expect!(self, Token::Keyword(kw) => kw, "keyword")?.0;
                            if kw.eq_ignore_ascii_case("other") {
                                expect!(self, Token::Equals, "equal sign")?;
                                let (ident, _) =
                                    expect!(self, Token::Identifier(ident) => ident, "path")?;
                                self.model
                                    .subroutines
                                    .push(Subroutine::Other(PathBuf::from(ident)));
                            } else if kw.eq_ignore_ascii_case("tol") {
                                expect!(self, Token::Equals, "equal sign")?;
                                let (value, _) = expect!(self, Token::Number { value, .. } => value, "tolerance value")?;
                                // Attach tolerance to the last builtin subroutine
                                if let Some(Subroutine::Builtin { tolerance, .. }) =
                                    self.model.subroutines.last_mut()
                                {
                                    *tolerance = Some(value as u32);
                                }
                            } else {
                                self.model.subroutines.push(Subroutine::Builtin {
                                    name: kw,
                                    tolerance: None,
                                });
                            }
                        }
                    }
                    ControlRecord::Pk => {
                        expect!(self, Token::Ignored(_), "PK block")?;
                    }
                    ControlRecord::Pred => {}
                    ControlRecord::Theta => {
                        let params = self.parse_parameters()?;
                        for (param, idx) in params {
                            self.model.theta_parameters.push(param);
                            self.model.token_ranges.theta_initial_values.push(idx);
                        }
                    }
                    ControlRecord::Omega => {
                        let (omega, omega_indices) = self.parse_omega_sigma()?;
                        self.model.omega_blocks.extend(omega);
                        self.model
                            .token_ranges
                            .omega_initial_values
                            .extend(omega_indices);
                    }
                    ControlRecord::Sigma => {
                        let (sigma, sigma_indices) = self.parse_omega_sigma()?;
                        self.model.sigma_blocks.extend(sigma);
                        self.model
                            .token_ranges
                            .sigma_initial_values
                            .extend(sigma_indices);
                    }
                    ControlRecord::Error => {}
                    ControlRecord::Estimation => {
                        let (estimation, indices) = self.parse_estimation()?;
                        self.model.estimations.push(estimation);
                        self.model.token_ranges.estimations.push(indices);
                    }
                    ControlRecord::Covariance => {
                        self.model.covariance = Some(self.parse_covariance()?);
                    }
                    ControlRecord::Model => {}
                    ControlRecord::Des => {}
                    ControlRecord::Simulation => {
                        self.model.simulation = Some(self.parse_simulation()?);
                    }
                    ControlRecord::Table => {
                        while let Some((peeked, _)) = self.peek_non_trivia()
                            && !matches!(peeked, Token::ControlRecord { .. })
                        {
                            if let (Token::Keyword(kw), _) = self.next_non_trivia_or_error()?
                                && kw.eq_ignore_ascii_case("FILE")
                            {
                                expect!(self, Token::Equals, "equal sign")?;
                                let path = expect!(self, Token::Identifier(id) => id, "path")?.0;
                                self.model.tables.push(PathBuf::from(path));
                                self.model
                                    .token_ranges
                                    .table_files
                                    .push(self.index as usize);
                            }
                        }
                    }
                    ControlRecord::Other(_) => {}
                },
                Token::Whitespace(_) | Token::Comment(_) => {
                    unreachable!("impossible to get trivia tokens, got {:?}", token)
                }
                _ => (),
            }
        }

        self.model.tokens = self.tokens.iter().map(|s| s.node().clone()).collect();

        Ok(std::mem::take(&mut self.model))
    }
}

#[cfg(test)]
mod tests {
    use fs_err as fs;
    use insta::{assert_debug_snapshot, assert_snapshot, glob};
    use std::path::PathBuf;

    use super::Model;

    #[test]
    fn can_parse_mod_files() {
        let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/parser");
        glob!(&test_dir, "*.mod", |path| {
            let input = fs::read_to_string(path).unwrap();
            let model = Model::parse(&input).unwrap();
            assert_debug_snapshot!(model);
        });
    }

    #[test]
    fn can_change_relative_paths() {
        glob!("../../test_data/model_paths", "*.mod", |path| {
            let input = fs::read_to_string(path).unwrap();
            let model = Model::parse(&input).unwrap();
            // TODO Should this relative path change?
            assert_snapshot!(model.with_modified_paths(Path::new("/home/vincent/dataset.csv")));
        });
    }
}
