//! Attribute definition and usage parsing.

use crate::ast::{AttributeBody, AttributeDef, AttributeUsage, Node};
use crate::parser::expr::expression;
use crate::parser::lex::{
    identification, name, qualified_name, skip_until_brace_end, take_until_terminator, ws1,
    ws_and_comments,
};
use crate::parser::node_from_to;
use crate::parser::with_span;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::combinator::map;
use nom::sequence::{delimited, preceded};
use nom::Parser;
use nom::IResult;

fn is_reserved_shorthand_starter(name: &str) -> bool {
    matches!(
        name,
        "interface"
            | "part"
            | "connect"
            | "bind"
            | "perform"
            | "allocate"
            | "port"
            | "state"
            | "satisfy"
            | "action"
            | "attribute"
            | "ref"
            | "doc"
            | "metadata"
            | "filter"
            | "use"
            | "view"
            | "viewpoint"
            | "render"
            | "rendering"
            | "requirement"
            | "require"
            | "concern"
            | "actor"
            | "item"
            | "individual"
            | "constraint"
            | "calc"
            | "enum"
            | "occurrence"
    )
}

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

fn skip_value_part(input: Input<'_>) -> IResult<Input<'_>, ()> {
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
    let (input, _) = take_until_terminator(input, b";{")?;
    Ok((input, ()))
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
    let (input, _) = nom::combinator::opt(preceded(
        alt((
            tag(&b"private"[..]),
            tag(&b"protected"[..]),
            tag(&b"public"[..]),
        )),
        ws1,
    ))
    .parse(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"attribute"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"def"[..]), ws1)).parse(input)?;
    let (input, ident) = identification(input)?;
    let name_str = ident.name.clone().unwrap_or_default();
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
    // Quantities/Units declarations often contain rich suffixes before body (`:>>`, lists, defaults,
    // multiple typed references). Consume the remainder up to `;`/`{` to keep dedicated mapping.
    let (input, _) = nom::combinator::opt(skip_value_part).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = attribute_body(input)?;
    Ok((
        input,
        node_from_to(start, input, AttributeDef {
            name: name_str,
            typing,
            body,
            name_span: None,
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

/// Shorthand attribute usage (no `attribute` keyword) commonly used inside part bodies.
///
/// Supports:
/// - `name : Type ;`
/// - `name : Type = expr ;`
/// - `:>> name : Type = expr ;` (leading `:>>` ignored; treated as a usage)
pub(crate) fn attribute_usage_shorthand(input: Input<'_>) -> IResult<Input<'_>, Node<AttributeUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(ws_and_comments, tag(&b":>>"[..]))).parse(input)?;
    let (input, (name_span, name_str)) = with_span(name).parse(input)?;
    if is_reserved_shorthand_starter(&name_str) {
        return Err(nom::Err::Error(nom::error::Error::new(
            start,
            nom::error::ErrorKind::Tag,
        )));
    }
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    // Parse (and ignore) the declared type.
    let (input, _) = preceded(ws_and_comments, qualified_name).parse(input)?;
    // Optional value assignment. For now, we accept the syntax but do not fully parse the expression
    // (some validation fixtures use complex function calls like `sum(( ... ))`).
    let (input, _) = nom::combinator::opt(preceded(
        ws_and_comments,
        alt((
            preceded(tag(&b"="[..]), ws_and_comments),
            preceded(tag(&b":="[..]), ws_and_comments),
            preceded(
                preceded(tag(&b"default"[..]), ws1),
                preceded(alt((tag(&b"="[..]), tag(&b":="[..]))), ws_and_comments),
            ),
        )),
    ))
    .parse(input)?;
    let (input, _) = nom::combinator::opt(preceded(ws_and_comments, take_until(&b";"[..]))).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((
        input,
        node_from_to(start, input, AttributeUsage {
            name: name_str,
            redefines: None,
            value: None,
            body: AttributeBody::Semicolon,
            name_span: Some(name_span),
            redefines_span: None,
        }),
    ))
}
