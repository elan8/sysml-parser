//! Interface definition and usage parsing.
#![allow(dead_code, unused_imports)]

use crate::ast::{
    ConnectBody, ConnectStmt, EndDecl, InterfaceDef, InterfaceDefBody, InterfaceDefBodyElement,
    Node, RefBody, RefDecl,
};
use crate::parser::expr::path_expression;
use crate::parser::lex::{
    identification, name, qualified_name, skip_until_brace_end, take_until_terminator, ws1,
    ws_and_comments,
};
use crate::parser::node_from_to;
use crate::parser::requirement::doc_comment;
use crate::parser::with_span;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::preceded;
use nom::IResult;
use nom::Parser;

/// End declaration: `end` name `:` type `;`
fn end_decl(input: Input<'_>) -> IResult<Input<'_>, Node<EndDecl>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"end"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) =
        nom::combinator::opt(preceded(ws_and_comments, tag(&b"port"[..]))).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, (name_span, name_str)) = with_span(name).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, (tilde, (type_ref_span, type_name))) = preceded(
        ws_and_comments,
        (
            nom::combinator::opt(tag(&b"~"[..])),
            with_span(qualified_name),
        ),
    )
    .parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            EndDecl {
                name: name_str,
                type_name: if tilde.is_some() {
                    format!("~{}", type_name)
                } else {
                    type_name
                },
                uses_derived_syntax: false,
                name_span: Some(name_span),
                type_ref_span: Some(type_ref_span),
            },
        ),
    ))
}

/// Ref body: `;` or `{` ... `}`
fn ref_body(input: Input<'_>) -> IResult<Input<'_>, RefBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| RefBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| RefBody::Brace,
        ),
    ))
    .parse(input)
}

/// Ref declaration: `ref` name `:` type body
fn ref_decl(input: Input<'_>) -> IResult<Input<'_>, Node<RefDecl>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"ref"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, (name_span, name_str)) = with_span(name).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, (type_ref_span, type_name)) =
        preceded(ws_and_comments, with_span(qualified_name)).parse(input)?;
    let (input, body) = ref_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            RefDecl {
                name: name_str,
                type_name,
                value: None,
                body,
                name_span: Some(name_span),
                type_ref_span: Some(type_ref_span),
            },
        ),
    ))
}

/// Connect body: `;` or `{` ... `}`
pub(crate) fn connect_body(input: Input<'_>) -> IResult<Input<'_>, ConnectBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| ConnectBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| ConnectBody::Brace,
        ),
    ))
    .parse(input)
}

/// Connect statement: `connect` from `to` to body
fn connect_stmt(input: Input<'_>) -> IResult<Input<'_>, Node<ConnectStmt>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"connect"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, from_expr) = path_expression(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"to"[..])).parse(input)?;
    let (input, to_expr) = preceded(ws_and_comments, path_expression).parse(input)?;
    let (input, body) = connect_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            ConnectStmt {
                from: from_expr,
                to: to_expr,
                body,
            },
        ),
    ))
}

fn interface_def_body_element(
    input: Input<'_>,
) -> IResult<Input<'_>, Node<InterfaceDefBodyElement>> {
    let (input, _) = ws_and_comments(input)?;
    let start = input;
    let (input, elem) = alt((
        map(doc_comment, InterfaceDefBodyElement::Doc),
        map(end_decl, InterfaceDefBodyElement::EndDecl),
        map(ref_decl, InterfaceDefBodyElement::RefDecl),
        map(connect_stmt, InterfaceDefBodyElement::ConnectStmt),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

/// Interface def body: `;` or `{` InterfaceDefBodyElement* `}`
fn interface_def_body(input: Input<'_>) -> IResult<Input<'_>, InterfaceDefBody> {
    let (input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b";") {
        let (input, _) = tag(&b";"[..]).parse(input)?;
        return Ok((input, InterfaceDefBody::Semicolon));
    }
    let (input, _) = tag(&b"{"[..]).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, elements) =
        many0(preceded(ws_and_comments, interface_def_body_element)).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = if input.fragment().starts_with(b"}") {
        (input, ())
    } else {
        skip_until_brace_end(input)?
    };
    let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
    Ok((input, InterfaceDefBody::Brace { elements }))
}

/// Interface definition: `interface` `def` Identification body
pub(crate) fn interface_def(input: Input<'_>) -> IResult<Input<'_>, Node<InterfaceDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"interface"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"def"[..]), ws1)).parse(input)?;
    let (input, identification) = identification(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = interface_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            InterfaceDef {
                identification,
                body,
            },
        ),
    ))
}
