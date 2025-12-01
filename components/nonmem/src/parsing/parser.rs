use crate::estimation::EstimationMethod;
use crate::parsing::comments::ParamName;
use crate::parsing::errors::SyntaxError;
use crate::parsing::lexer::ControlRecord;
use crate::parsing::model::{
    BlockStructure, ComparisonOperator, Data, DataFilter, DataValueFilter, DataValueFilterKind,
    Estimation, InputColumn, Parameter, ParameterBlock, Parameterization, Subroutine,
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
                "IGNORE" => {
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

    fn parse_parameters<T: ParamName>(
        &mut self,
    ) -> Result<Vec<(Parameter<T>, usize)>, SyntaxError> {
        let mut out = Vec::new();
        // (param_index, line_number)
        let mut parameters_with_lines = Vec::new();

        // First pass: Parse all parameters and track their line numbers
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

                    out.push((
                        Parameter {
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

                    let param_index = out.len();
                    parameters_with_lines.push((param_index, param_line));

                    let param = match values.len() {
                        1 => (
                            Parameter {
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
                    out.push(param);

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

            if let Token::Keyword(kw) = token {
                match kw.to_uppercase().as_str() {
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
                    _ => {}
                }
            }
        }

        Ok((estimation, (file_idx, msfo_idx)))
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

        // Parse additional keywords that can come after BLOCK(N)
        let mut advance = false;
        if let Some((token, _)) = self.peek_non_trivia() {
            let kw = match token {
                Token::Keyword(k) => Some(k),
                Token::Identifier(k) => Some(k),
                _ => None,
            };

            if let Some(kw) = kw {
                advance = true;

                if let Some(param) = Parameterization::from_keyword(kw) {
                    parametrization = Some(param);
                } else if kw.eq_ignore_ascii_case("SAME") {
                    same = true;
                } else if kw.eq_ignore_ascii_case("FIX") || kw.eq_ignore_ascii_case("FIXED") {
                    block_fixed = true;
                } else if kw.eq_ignore_ascii_case("VALUES") {
                    // VALUES is handled below in the parameter parsing section
                    advance = false; // Don't consume it here
                } else {
                    advance = false; // Don't advance if we don't recognize the keyword
                }
            }
        }

        if advance {
            self.next_non_trivia_or_error()?;
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
            let mut expanded_params = Vec::new();

            for i in 0..size {
                for j in 0..=i {
                    let value = if i == j {
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

                    expanded_params.push((
                        Parameter {
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
                Token::Keyword(kw) if !kw.eq_ignore_ascii_case("BLOCK") => {
                    self.next_non_trivia_or_error()?;
                    if let Some(param) = Parameterization::from_keyword(&kw) {
                        initial_parametrization = Some(param);
                    } else if kw.eq_ignore_ascii_case("SAME") {
                        initial_same = true;
                    }
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
                }
                Token::Number { .. } | Token::LeftParen => {
                    let params = self.parse_parameters()?;
                    let (parameters, indices): (Vec<_>, Vec<_>) = params.into_iter().unzip();
                    out.push(ParameterBlock {
                        structure: BlockStructure::Diagonal,
                        parametrization: None,
                        parameters,
                    });
                    token_indices.push(indices);
                }
                _ => (),
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
                            } else {
                                self.model.subroutines.push(Subroutine::Builtin(kw));
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
                    ControlRecord::Covariance => {}
                    ControlRecord::Model => {}
                    ControlRecord::Des => {}
                    ControlRecord::Simulation => {
                        while let Some((peeked, _)) = self.peek_non_trivia()
                            && !matches!(peeked, Token::ControlRecord { .. })
                        {
                            if let (Token::Keyword(kw), _) = self.next_non_trivia_or_error()?
                                && kw.eq_ignore_ascii_case("ONLYSIM")
                            {
                                self.model.is_simulation_only = true;
                            }
                        }
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
    use super::*;
    use fs_err as fs;
    use insta::{assert_debug_snapshot, assert_snapshot, glob};
    use std::path::Path;

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

    #[test]
    fn can_do_theta_perturbation() {
        let input = fs::read_to_string("test_data/parser/multiline_table.mod").unwrap();
        let model = Model::parse(&input).unwrap();
        let retries = model.theta_perturbation(0.1, 3, Some(42)).unwrap();
        let params = retries
            .iter()
            // TODO change this relative path?
            .map(|x| x.with_modified_paths(Path::new("/home/vincent/dataset.csv")))
            .collect::<Vec<_>>();
        assert_debug_snapshot!(params);
    }
}
