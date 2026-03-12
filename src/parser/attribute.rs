//! Attribute definition and usage parsing.

use crate::ast::{AttributeBody, AttributeDef, AttributeUsage, Node};
use crate::parser::expr::expression;
use crate::parser::lex::{
    name, qualified_name, skip_until_brace_end, ws1, ws_and_comments,
};
use crate::parser::node_from_to;
use crate::parser::with_span;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::sequence::{delimited, preceded};
use nom::Parser;
use nom::IResult;

/// Value part: `= expr` | `:= expr` | `default = expr` | `default := expr` (BNF FeatureValue).
fn value_part(input: Input<'_>) -> IResult<Input<'_>, Node<crate::ast::Expression>> {
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = alt((
        preceded(tag(&b"="[..]), ws_and_comments),
        preceded(tag(&b":="[..]), ws_and_comments),
        preceded(
            preceded(tag(&b"default"[..]), ws1),
            preceded(alt((tag(&b"="[..]), tag(&b":="[..]))), ws_and_comments),
        ),
    ))
    .parse(input)?;
    expression(input)
}

/// Attribute body: ';' or '{' ... '}' (skip content inside braces)
pub(crate) fn attribute_body(input: Input<'_>) -> IResult<Input<'_>, AttributeBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| AttributeBody::Semicolon),
        map(
            delimited(
                tag(&b"{"[..]),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| AttributeBody::Brace,
        ),
    ))
    .parse(input)
}

/// Attribute definition: 'attribute' name ( ':>' | ':' )? qualified_name? body
pub(crate) fn attribute_def(input: Input<'_>) -> IResult<Input<'_>, Node<AttributeDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"attribute"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"def"[..]), ws1)).parse(input)?;
    let (input, (name_span, name_str)) = with_span(name).parse(input)?;
    let (input, typing_result) = nom::combinator::opt(alt((
        preceded(
            preceded(ws_and_comments, tag(&b":>"[..])),
            preceded(ws_and_comments, with_span(qualified_name)),
        ),
        preceded(
            preceded(ws_and_comments, tag(&b":"[..])),
            preceded(ws_and_comments, with_span(qualified_name)),
        ),
    )))
    .parse(input)?;
    let (typing_span, typing) = typing_result
        .map(|(span, s)| (Some(span), Some(s)))
        .unwrap_or((None, None));
    let (input, body) = attribute_body(input)?;
    Ok((
        input,
        node_from_to(start, input, AttributeDef {
            name: name_str,
            typing,
            body,
            name_span: Some(name_span),
            typing_span,
        }),
    ))
}

/// Attribute usage: 'attribute' name ( 'redefines' qualified_name )? ( '=' value )? body
pub(crate) fn attribute_usage(input: Input<'_>) -> IResult<Input<'_>, Node<AttributeUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"attribute"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, (name_span, name_str)) = with_span(name).parse(input)?;
    let (input, redefines_result) = nom::combinator::opt(alt((
        preceded(
            preceded(ws_and_comments, tag(&b"redefines"[..])),
            preceded(ws1, with_span(qualified_name)),
        ),
        preceded(
            preceded(ws_and_comments, tag(&b":>>"[..])),
            preceded(ws_and_comments, with_span(qualified_name)),
        ),
    )))
    .parse(input)?;
    let (redefines_span, redefines) = redefines_result
        .map(|(span, s)| (Some(span), Some(s)))
        .unwrap_or((None, None));
    let (input, value) = nom::combinator::opt(preceded(ws_and_comments, value_part)).parse(input)?;
    let (input, body) = attribute_body(input)?;
    Ok((
        input,
        node_from_to(start, input, AttributeUsage {
            name: name_str,
            redefines,
            value,
            body,
            name_span: Some(name_span),
            redefines_span,
        }),
    ))
}
