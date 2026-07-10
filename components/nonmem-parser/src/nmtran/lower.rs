use super::{NmtranSpannedToken, NmtranToken};
use crate::ast::{BinaryOp, NmtranExpr, NmtranStatement, UnaryOp};
use crate::cst::{NmtranChild, NmtranNode, NmtranNodeKind};

pub(crate) fn lower_stmts(
    children: &[NmtranChild],
    tokens: &[NmtranSpannedToken],
) -> Vec<NmtranStatement> {
    children
        .iter()
        .filter_map(|c| match c {
            NmtranChild::Node(n) => Some(lower_stmt(n, tokens)),
            NmtranChild::Token(_) => None,
        })
        .collect()
}

fn lower_stmt(node: &NmtranNode, tokens: &[NmtranSpannedToken]) -> NmtranStatement {
    match node.kind {
        NmtranNodeKind::Assignment => lower_assignment(node, tokens),
        NmtranNodeKind::If => lower_if(node, tokens),
        NmtranNodeKind::DoWhile => lower_do_while(node, tokens),
        NmtranNodeKind::Call => lower_call(node, tokens),
        NmtranNodeKind::Exit => lower_exit(node, tokens),
        _ => NmtranStatement::Unknown {
            text: node.text(tokens).trim().to_string(),
        },
    }
}

fn lower_assignment(node: &NmtranNode, tokens: &[NmtranSpannedToken]) -> NmtranStatement {
    let non_trivia = node.non_trivia_children(tokens);

    // First token is target ident
    let target = match &non_trivia[0] {
        NmtranChild::Token(i) => tokens[*i].text.clone(),
        _ => unreachable!("parser guarantees ident token as assignment target"),
    };

    // Check for indices: LeftParen, ArgList, RightParen
    let mut idx = 1;
    let mut indices = Vec::new();

    if matches!(&non_trivia.get(idx), Some(NmtranChild::Token(i)) if tokens[*i].token == NmtranToken::LeftParen)
    {
        idx += 1; // skip (
        // Collect ArgList children if present
        if let Some(NmtranChild::Node(arg_list)) = non_trivia.get(idx)
            && arg_list.kind == NmtranNodeKind::ArgList
        {
            for c in &arg_list.children {
                match c {
                    NmtranChild::Token(i)
                        if matches!(
                            tokens[*i].token,
                            NmtranToken::Ident | NmtranToken::Int | NmtranToken::Float
                        ) =>
                    {
                        indices.push(tokens[*i].text.clone());
                    }
                    NmtranChild::Node(n) => {
                        // For expression arguments, render their text
                        indices.push(n.text(tokens).trim().to_string());
                    }
                    _ => {}
                }
            }
            idx += 1;
        }
        idx += 1; // skip )
    }

    // Skip Equals
    idx += 1;

    // Expression
    let expr = non_trivia
        .get(idx)
        .map(|c| lower_expr(c, tokens))
        .expect("parser guarantees expression in assignment");

    NmtranStatement::Assignment {
        target,
        indices,
        expr,
    }
}

fn flatten_nested_if(
    node: &NmtranNode,
    tokens: &[NmtranSpannedToken],
    elseif_branches: &mut Vec<(NmtranExpr, Vec<NmtranStatement>)>,
    else_body: &mut Option<Vec<NmtranStatement>>,
) {
    let NmtranStatement::If {
        condition,
        body,
        elseif_branches: nested_elseifs,
        else_body: nested_else,
    } = lower_if(node, tokens)
    else {
        unreachable!("lower_if always returns If")
    };
    elseif_branches.push((condition, body));
    elseif_branches.extend(nested_elseifs);
    *else_body = nested_else;
}

/// Lower an `If` node into `NmtranStatement::If`.
///
/// The parser can produce two shapes for `ELSE IF`: either a nested
/// `NmtranNodeKind::If` node directly in the body, or an `ElseKw` token
/// followed by a nested `If` node in the else-body loop.
fn lower_if(node: &NmtranNode, tokens: &[NmtranSpannedToken]) -> NmtranStatement {
    let non_trivia = node.non_trivia_children(tokens);

    // Structure: IF/ELSEIF, LeftParen, condition_expr, RightParen, [THEN], body_stmts..., [nested_If | ELSE + else_body + ENDIF]
    let mut idx = 0;

    // Skip IF/ELSEIF keyword
    idx += 1;
    // Skip LeftParen
    idx += 1;

    // Extract condition
    let condition = non_trivia
        .get(idx)
        .map(|c| lower_expr(c, tokens))
        .expect("parser guarantees condition in if");
    idx += 1;

    // Skip RightParen
    idx += 1;

    // Check for THEN
    let has_then = matches!(
        non_trivia.get(idx),
        Some(NmtranChild::Token(i)) if tokens[*i].token == NmtranToken::ThenKw
    );

    if !has_then {
        // Inline form `IF (cond) stmt`
        let body = match non_trivia.get(idx) {
            Some(NmtranChild::Node(n)) => vec![lower_stmt(n, tokens)],
            _ => Vec::new(),
        };
        return NmtranStatement::If {
            condition,
            body,
            elseif_branches: Vec::new(),
            else_body: None,
        };
    }
    idx += 1; // THEN

    // Collect body statements and look for termination.
    // The last non trivia token is an `ELSEIF` or `ELSE IF` if present
    let last = non_trivia.len().saturating_sub(1);
    let mut body = Vec::new();
    let mut elseif_branches = Vec::new();
    let mut else_body = None;

    while idx < non_trivia.len() {
        match &non_trivia[idx] {
            NmtranChild::Node(n) if n.kind == NmtranNodeKind::If && idx == last => {
                // Trailing ELSEIF chain.
                flatten_nested_if(n, tokens, &mut elseif_branches, &mut else_body);
                break;
            }
            NmtranChild::Token(i)
                if tokens[*i].token == NmtranToken::EndIfKw
                    || tokens[*i].token == NmtranToken::EndKw =>
            {
                break;
            }
            NmtranChild::Token(i) if tokens[*i].token == NmtranToken::ElseKw => {
                idx += 1;
                if let Some(NmtranChild::Node(n)) = non_trivia.get(idx)
                    && n.kind == NmtranNodeKind::If
                    && idx == last
                {
                    flatten_nested_if(n, tokens, &mut elseif_branches, &mut else_body);
                    break;
                }
                // Otherwise collect the else body (which may contain nested IFs).
                let mut else_stmts = Vec::new();
                while idx < non_trivia.len() {
                    match &non_trivia[idx] {
                        NmtranChild::Token(j)
                            if tokens[*j].token == NmtranToken::EndIfKw
                                || tokens[*j].token == NmtranToken::EndKw =>
                        {
                            break;
                        }
                        NmtranChild::Node(n) => {
                            else_stmts.push(lower_stmt(n, tokens));
                        }
                        _ => {}
                    }
                    idx += 1;
                }
                if !else_stmts.is_empty() {
                    else_body = Some(else_stmts);
                }
                break;
            }
            NmtranChild::Node(n) => {
                body.push(lower_stmt(n, tokens));
            }
            _ => {}
        }
        idx += 1;
    }

    NmtranStatement::If {
        condition,
        body,
        elseif_branches,
        else_body,
    }
}

fn lower_do_while(node: &NmtranNode, tokens: &[NmtranSpannedToken]) -> NmtranStatement {
    let non_trivia = node.non_trivia_children(tokens);

    let mut idx = 1; // DO / DOWHILE
    if matches!(non_trivia.get(idx), Some(NmtranChild::Token(i)) if tokens[*i].token == NmtranToken::WhileKw)
    {
        idx += 1; // WHILE
    }
    idx += 1; // (

    let condition = non_trivia
        .get(idx)
        .map(|c| lower_expr(c, tokens))
        .expect("parser guarantees condition in do-while");
    idx += 1; // condition
    idx += 1; // )

    let mut body = Vec::new();
    while idx < non_trivia.len() {
        match &non_trivia[idx] {
            NmtranChild::Token(i)
                if tokens[*i].token == NmtranToken::EndDoKw
                    || tokens[*i].token == NmtranToken::EndKw =>
            {
                break;
            }
            NmtranChild::Node(n) => {
                body.push(lower_stmt(n, tokens));
            }
            _ => {}
        }
        idx += 1;
    }

    NmtranStatement::DoWhile { condition, body }
}

fn lower_call(node: &NmtranNode, tokens: &[NmtranSpannedToken]) -> NmtranStatement {
    let non_trivia = node.non_trivia_children(tokens);

    let mut idx = 1; // skip CALL

    let subroutine = match non_trivia.get(idx) {
        Some(NmtranChild::Token(i)) if tokens[*i].token == NmtranToken::Ident => {
            let name = tokens[*i].text.clone();
            idx += 1;
            name
        }
        _ => {
            return NmtranStatement::Unknown {
                text: node.text(tokens).trim().to_string(),
            };
        }
    };

    let mut args = Vec::new();

    // Check for LeftParen
    if matches!(non_trivia.get(idx), Some(NmtranChild::Token(i)) if tokens[*i].token == NmtranToken::LeftParen)
    {
        idx += 1; // skip (
        while idx < non_trivia.len() {
            match &non_trivia[idx] {
                NmtranChild::Token(i) if tokens[*i].token == NmtranToken::RightParen => break,
                NmtranChild::Token(i) if tokens[*i].token == NmtranToken::Comma => {}
                _ => {
                    args.push(lower_expr(non_trivia[idx], tokens));
                }
            }
            idx += 1;
        }
    }

    NmtranStatement::Call { subroutine, args }
}

fn lower_exit(node: &NmtranNode, tokens: &[NmtranSpannedToken]) -> NmtranStatement {
    let non_trivia = node.non_trivia_children(tokens);
    let args: Vec<String> = non_trivia
        .iter()
        .filter_map(|c| match c {
            NmtranChild::Token(i) if tokens[*i].token != NmtranToken::ExitKw => {
                Some(tokens[*i].text.clone())
            }
            _ => None,
        })
        .collect();

    NmtranStatement::Exit { args }
}

fn lower_expr(child: &NmtranChild, tokens: &[NmtranSpannedToken]) -> NmtranExpr {
    match child {
        NmtranChild::Token(idx) => {
            let tok = &tokens[*idx];
            match tok.token {
                NmtranToken::Int | NmtranToken::Float => {
                    let text = tok.text.replace(['D', 'd'], "E");
                    let value = text.parse::<f64>().unwrap_or_else(|_| {
                        unreachable!("lexer guarantees Int/Float parses as f64: {text:?}")
                    });
                    NmtranExpr::Number(value)
                }
                _ => NmtranExpr::Ident(tok.text.clone()),
            }
        }
        NmtranChild::Node(n) => match n.kind {
            NmtranNodeKind::BinaryExpr => {
                let non_trivia = n.non_trivia_children(tokens);

                if non_trivia.len() < 3 {
                    unreachable!("parser guarantees lhs, op, rhs in binary expr");
                }

                let lhs = lower_expr(non_trivia[0], tokens);
                let op = match non_trivia[1] {
                    NmtranChild::Token(i) => BinaryOp::from(&tokens[*i].token),
                    _ => unreachable!("parser guarantees token operator in binary expr"),
                };
                let rhs = lower_expr(non_trivia[2], tokens);

                NmtranExpr::BinaryExpr {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            }
            NmtranNodeKind::UnaryExpr => {
                let non_trivia = n.non_trivia_children(tokens);

                if non_trivia.len() < 2 {
                    unreachable!("parser guarantees op and operand in unary expr");
                }

                let op = match non_trivia[0] {
                    NmtranChild::Token(i) => match tokens[*i].token {
                        NmtranToken::Minus => UnaryOp::Neg,
                        NmtranToken::Plus => UnaryOp::Pos,
                        NmtranToken::DotNot => UnaryOp::Not,
                        _ => unreachable!("parser only emits Minus, Plus, DotNot for unary"),
                    },
                    _ => unreachable!("parser guarantees token operator in unary expr"),
                };
                let operand = lower_expr(non_trivia[1], tokens);

                NmtranExpr::UnaryExpr {
                    op,
                    operand: Box::new(operand),
                }
            }
            NmtranNodeKind::ParenExpr => {
                let non_trivia = n.non_trivia_children(tokens);

                // (expr) → skip parens, get inner
                if non_trivia.len() >= 3 {
                    let inner = lower_expr(non_trivia[1], tokens);
                    NmtranExpr::Paren(Box::new(inner))
                } else {
                    unreachable!("parser guarantees lparen, expr, rparen in paren expr")
                }
            }
            NmtranNodeKind::FunctionCall => {
                let non_trivia = n.non_trivia_children(tokens);

                // Ident, LeftParen, [ArgList], RightParen
                let name = match non_trivia.first() {
                    Some(NmtranChild::Token(i)) => tokens[*i].text.clone(),
                    _ => unreachable!("parser guarantees function name"),
                };

                let mut args = Vec::new();
                for c in &non_trivia {
                    if let NmtranChild::Node(arg_list) = c
                        && arg_list.kind == NmtranNodeKind::ArgList
                    {
                        // Lower each non-trivia, non-comma child as expr
                        for ac in &arg_list.children {
                            match ac {
                                NmtranChild::Token(i)
                                    if tokens[*i].token == NmtranToken::Comma
                                        || tokens[*i].token.is_trivia() => {}
                                _ => {
                                    args.push(lower_expr(ac, tokens));
                                }
                            }
                        }
                    }
                }

                NmtranExpr::FunctionCall { name, args }
            }
            NmtranNodeKind::Assignment
            | NmtranNodeKind::If
            | NmtranNodeKind::DoWhile
            | NmtranNodeKind::Call
            | NmtranNodeKind::Exit
            | NmtranNodeKind::ArgList
            | NmtranNodeKind::Unknown => NmtranExpr::Ident(n.text(tokens).trim().to_string()),
        },
    }
}
