//! Expression and path parsing for values and bind/connect.

use crate::ast::{Expression, Node};
use crate::parser::lex::{name, qualified_name, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::digit1;
use nom::combinator::{map, opt};
use nom::sequence::{delimited, preceded};
use nom::Parser;
use nom::IResult;

/// Integer literal.
fn literal_integer(input: Input<'_>) -> IResult<Input<'_>, Node<Expression>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, sign) = opt(alt((tag(&b"-"[..]), tag(&b"+"[..])))).parse(input)?;
    let (input, digits) = digit1.parse(input)?;
    let s = String::from_utf8_lossy(digits.fragment());
    let n: i64 = s.parse().unwrap_or(0);
    let n = if sign.map(|s: Input| s.fragment() == b"-").unwrap_or(false) {
        -n
    } else {
        n
    };
    Ok((input, node_from_to(start, input, Expression::LiteralInteger(n))))
}

/// Real literal (simple: digits.digits).
fn literal_real(input: Input<'_>) -> IResult<Input<'_>, Node<Expression>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, whole) = digit1.parse(input)?;
    let (input, _) = tag(&b"."[..]).parse(input)?;
    let (input, frac) = digit1.parse(input)?;
    let s = format!(
        "{}.{}",
        String::from_utf8_lossy(whole.fragment()),
        String::from_utf8_lossy(frac.fragment())
    );
    Ok((input, node_from_to(start, input, Expression::LiteralReal(s))))
}

/// String literal: double-quoted.
fn literal_string(input: Input<'_>) -> IResult<Input<'_>, Node<Expression>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"\""[..]).parse(input)?;
    let frag = input.fragment();
    let mut i = 0;
    while i < frag.len() {
        if frag[i] == b'\\' && i + 1 < frag.len() {
            i += 2;
            continue;
        }
        if frag[i] == b'"' {
            let s = String::from_utf8_lossy(&frag[..i]).replace("\\\"", "\"");
            let (input, _) = nom::bytes::complete::take(i + 1).parse(input)?;
            return Ok((input, node_from_to(start, input, Expression::LiteralString(s))));
        }
        i += 1;
    }
    let s = String::from_utf8_lossy(frag).replace("\\\"", "\"");
    let (input, _) = nom::bytes::complete::take(frag.len()).parse(input)?;
    Ok((input, node_from_to(start, input, Expression::LiteralString(s))))
}

/// Boolean literal: true | false.
fn literal_boolean(input: Input<'_>) -> IResult<Input<'_>, Node<Expression>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, v) = alt((
        map(tag(&b"true"[..]), |_| true),
        map(tag(&b"false"[..]), |_| false),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, Expression::LiteralBoolean(v))))
}

/// Feature reference: single name.
fn feature_ref_primary(input: Input<'_>) -> IResult<Input<'_>, Node<Expression>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, n) = name(input)?;
    Ok((input, node_from_to(start, input, Expression::FeatureRef(n))))
}

/// Literal only (no unit): integer, real, string, boolean.
fn literal_only(input: Input<'_>) -> IResult<Input<'_>, Node<Expression>> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        literal_boolean,
        literal_integer,
        literal_real,
        literal_string,
    ))
    .parse(input)
}

/// Literal with optional [ unit ]: 1750 [kg] -> LiteralWithUnit(...).
fn literal_with_unit(input: Input<'_>) -> IResult<Input<'_>, Node<Expression>> {
    let start = input;
    let (input, value_node) = literal_only(input)?;
    let (input, _) = ws_and_comments(input)?;
    if !input.fragment().starts_with(b"[") {
        return Ok((input, value_node));
    }
    let (input, _) = tag(&b"["[..]).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, unit_name) = qualified_name(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"]"[..]).parse(input)?;
    let unit = Node::new(
        crate::ast::Span::dummy(),
        Expression::Bracket(Box::new(Node::new(
            crate::ast::Span::dummy(),
            Expression::FeatureRef(unit_name),
        ))),
    );
    let expr = Expression::LiteralWithUnit {
        value: Box::new(value_node),
        unit: Box::new(unit),
    };
    Ok((input, node_from_to(start, input, expr)))
}

/// Parenthesized expression: ( expression ).
fn parenthesized(input: Input<'_>) -> IResult<Input<'_>, Node<Expression>> {
    let (input, _) = ws_and_comments(input)?;
    delimited(
        tag(&b"("[..]),
        preceded(ws_and_comments, expression),
        preceded(ws_and_comments, tag(&b")"[..])),
    )
    .parse(input)
}

/// KerML null or empty sequence ().
fn null_expression(input: Input<'_>) -> IResult<Input<'_>, Node<Expression>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = alt((
        map(tag(&b"null"[..]), |_| ()),
        map(
            delimited(tag(&b"("[..]), ws_and_comments, tag(&b")"[..])),
            |_| (),
        ),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, Expression::Null)))
}

/// Primary expression: literal with unit, literal only, feature ref, null, or parenthesized.
fn primary(input: Input<'_>) -> IResult<Input<'_>, Node<Expression>> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        literal_with_unit,
        literal_only,
        null_expression,
        feature_ref_primary,
        parenthesized,
    ))
    .parse(input)
}

/// Apply postfix #( expr ) or . name to an expression.
fn postfix<'a>(
    input: Input<'a>,
    start: Input<'a>,
    current: Node<Expression>,
) -> IResult<Input<'a>, Node<Expression>> {
    let (input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b"#") {
        let (input, _) = tag(&b"#"[..]).parse(input)?;
        let (input, _) = preceded(ws_and_comments, tag(&b"("[..])).parse(input)?;
        let (input, index_node) = preceded(ws_and_comments, expression).parse(input)?;
        let (input, _) = preceded(ws_and_comments, tag(&b")"[..])).parse(input)?;
        let expr = Expression::Index {
            base: Box::new(current),
            index: Box::new(index_node),
        };
        return postfix(input, start, node_from_to(start, input, expr));
    }
    if input.fragment().starts_with(b".") {
        let (input, _) = tag(&b"."[..]).parse(input)?;
        let (input, _) = ws_and_comments(input)?;
        let (input, member) = name(input)?;
        let expr = Expression::MemberAccess(Box::new(current), member);
        return postfix(input, start, node_from_to(start, input, expr));
    }
    Ok((input, current))
}

/// Try to consume a binary infix operator token (KerML BinaryOperator).
fn binary_op_token(input: Input<'_>) -> IResult<Input<'_>, String> {
    let (input, _) = ws_and_comments(input)?;
    // Multi-char operators must be tried before single-char subsets; ** before *.
    alt((
        map(tag(&b">="[..]), |_| ">=".to_string()),
        map(tag(&b"<="[..]), |_| "<=".to_string()),
        map(tag(&b"==="[..]), |_| "===".to_string()),
        map(tag(&b"!=="[..]), |_| "!==".to_string()),
        map(tag(&b"=="[..]), |_| "==".to_string()),
        map(tag(&b"!="[..]), |_| "!=".to_string()),
        map(tag(&b"**"[..]), |_| "**".to_string()),
        map(tag(&b"&&"[..]), |_| "&&".to_string()),
        map(tag(&b"||"[..]), |_| "||".to_string()),
        map(tag(&b"+"[..]), |_| "+".to_string()),
        map(tag(&b"-"[..]), |_| "-".to_string()),
        map(tag(&b"*"[..]), |_| "*".to_string()),
        map(tag(&b"/"[..]), |_| "/".to_string()),
        map(tag(&b"%"[..]), |_| "%".to_string()),
        map(tag(&b"^"[..]), |_| "^".to_string()),
        map(tag(&b".."[..]), |_| "..".to_string()),
        map(tag(&b"|"[..]), |_| "|".to_string()),
        map(tag(&b"&"[..]), |_| "&".to_string()),
        map(tag(&b"xor"[..]), |_| "xor".to_string()),
        map(tag(&b">"[..]), |_| ">".to_string()),
        map(tag(&b"<"[..]), |_| "<".to_string()),
    ))
    .parse(input)
}

/// Left-associative binary expression chain.
fn binary_chain<'a>(
    input: Input<'a>,
    start: Input<'a>,
    left: Node<Expression>,
) -> IResult<Input<'a>, Node<Expression>> {
    match binary_op_token(input) {
        Ok((input, op)) => {
            let (input, _) = ws_and_comments(input)?;
            let rhs_start = input;
            let (input, rhs_primary) = primary(input)?;
            let (input, rhs) = postfix(input, rhs_start, rhs_primary)?;
            let expr = Expression::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(rhs),
            };
            let node = node_from_to(start, input, expr);
            binary_chain(input, start, node)
        }
        Err(_) => Ok((input, left)),
    }
}

/// Unary operator token: + - ~ not (KerML UnaryOperator).
fn unary_op_token(input: Input<'_>) -> IResult<Input<'_>, String> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b"not"[..]), |_| "not".to_string()),
        map(tag(&b"~"[..]), |_| "~".to_string()),
        map(tag(&b"+"[..]), |_| "+".to_string()),
        map(tag(&b"-"[..]), |_| "-".to_string()),
    ))
    .parse(input)
}

/// Parse unary prefixes then primary; build nested UnaryOp from the right.
fn unary_and_primary(input: Input<'_>) -> IResult<Input<'_>, Node<Expression>> {
    let start = input;
    let (input, prefixes) = nom::multi::many0(unary_op_token).parse(input)?;
    let primary_start = input;
    let (input, primary_node) = primary(input)?;
    let (input, after_postfix) = postfix(input, primary_start, primary_node)?;
    let mut expr = after_postfix;
    for op in prefixes.into_iter().rev() {
        expr = node_from_to(
            start,
            input,
            Expression::UnaryOp {
                op,
                operand: Box::new(expr),
            },
        );
    }
    Ok((input, expr))
}

/// Full expression: optional unary prefixes + primary, postfix, then optional binary operator chain.
pub(crate) fn expression(input: Input<'_>) -> IResult<Input<'_>, Node<Expression>> {
    let start = input;
    let (input, primary_node) = unary_and_primary(input)?;
    binary_chain(input, start, primary_node)
}

/// Path expression: name or name.name.name... (for bind/connect). Returns Node<Expression>.
pub(crate) fn path_expression(input: Input<'_>) -> IResult<Input<'_>, Node<Expression>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, first) = name(input)?;
    let mut expr = Expression::FeatureRef(first);
    let mut rest = input;
    loop {
        let (next, _) = ws_and_comments(rest)?;
        if !next.fragment().starts_with(b".") {
            break;
        }
        let (next, _) = tag(&b"."[..]).parse(next)?;
        let (next, _) = ws_and_comments(next)?;
        let (next, member) = name(next)?;
        expr = Expression::MemberAccess(
            Box::new(Node::new(crate::ast::Span::dummy(), expr)),
            member,
        );
        rest = next;
    }
    Ok((rest, node_from_to(start, rest, expr)))
}
