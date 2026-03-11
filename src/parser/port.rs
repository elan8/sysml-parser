//! Port definition and port usage parsing.

use crate::ast::{Node, PortBody, PortDef, PortDefBody, PortDefBodyElement, PortUsage};
use crate::parser::expr::expression;
use crate::parser::lex::{identification, name, qualified_name, skip_until_brace_end, ws1, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::combinator::{map, opt};
use nom::multi::many0;
use nom::sequence::{preceded, tuple};
use nom::IResult;

/// Port body: ';' or '{' PortUsage* '}' (nested ports) or '{' skip '}' for Brace (e.g. in/out ends).
fn port_body(input: Input<'_>) -> IResult<Input<'_>, PortBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(b";"), |_| PortBody::Semicolon),
        port_body_brace,
    ))(input)
}

/// Brace port body: '{' ( PortUsage* | skip to '}' ) '}'.
fn port_body_brace(input: Input<'_>) -> IResult<Input<'_>, PortBody> {
    let (input, _) = tag(b"{")(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, elements) = many0(preceded(ws_and_comments, port_usage))(input)?;
    let (input, _) = if elements.is_empty() {
        skip_until_brace_end(input)?
    } else {
        (input, ())
    };
    let (input, _) = preceded(ws_and_comments, tag(b"}"))(input)?;
    Ok((
        input,
        if elements.is_empty() {
            PortBody::Brace
        } else {
            PortBody::BraceWithPorts { elements }
        },
    ))
}

/// Port usage: 'port' ( ':>>' name | name ) ( ':' type )? multiplicity? ( ':>' ... )? ( 'redefines' ... )? body
pub(crate) fn port_usage(input: Input<'_>) -> IResult<Input<'_>, Node<PortUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(b"port")(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = alt((
        preceded(
            preceded(ws_and_comments, tag(b":>>")),
            preceded(ws_and_comments, name),
        ),
        preceded(ws_and_comments, name),
    ))(input)?;
    let (input, type_name) = opt(preceded(
        preceded(ws_and_comments, tag(b":")),
        preceded(ws_and_comments, qualified_name),
    ))(input)?;
    let (input, multiplicity) = opt(multiplicity)(input)?;
    let (input, subsets) = opt(preceded(
        preceded(ws_and_comments, tag(b":>")),
        preceded(ws_and_comments, tuple((
            name,
            opt(preceded(
                preceded(ws_and_comments, tag(b"=")),
                preceded(ws_and_comments, expression),
            )),
        ))),
    ))(input)?;
    let (input, redefines) = opt(preceded(
        preceded(ws_and_comments, tag(b"redefines")),
        preceded(ws1, qualified_name),
    ))(input)?;
    let (input, body) = port_body(input)?;
    Ok((
        input,
        node_from_to(start, input, PortUsage {
            name: name_str,
            type_name,
            multiplicity,
            subsets,
            redefines,
            body,
        }),
    ))
}

fn multiplicity(input: Input<'_>) -> IResult<Input<'_>, String> {
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(b"[")(input)?;
    let (input, content) = take_until(&b"]"[..])(input)?;
    let (input, _) = tag(b"]")(input)?;
    let s = format!("[{}]", String::from_utf8_lossy(content.fragment()).trim());
    Ok((input, s))
}

/// Port def body: ';' or '{' PortDefBodyElement* '}'
fn port_def_body(input: Input<'_>) -> IResult<Input<'_>, PortDefBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(b";"), |_| PortDefBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(b"{"),
                preceded(
                    ws_and_comments,
                    many0(preceded(ws_and_comments, port_def_body_element)),
                ),
                preceded(ws_and_comments, tag(b"}")),
            ),
            |elements| PortDefBody::Brace { elements },
        ),
    ))(input)
}

fn port_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<PortDefBodyElement>> {
    let start = input;
    let (input, p) = port_usage(input)?;
    Ok((input, node_from_to(start, input, PortDefBodyElement::PortUsage(p))))
}

/// Port definition: 'port' 'def' Identification body
pub(crate) fn port_def(input: Input<'_>) -> IResult<Input<'_>, Node<PortDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(b"port")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(b"def")(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, body) = port_def_body(input)?;
    Ok((
        input,
        node_from_to(start, input, PortDef {
            identification,
            body,
        }),
    ))
}
