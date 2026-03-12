//! Connection definition parsing (BNF ConnectionDefinition).

use crate::ast::{
    ConnectionDef, ConnectionDefBody, ConnectionDefBodyElement, ConnectStmt, EndDecl, Node,
    RefBody, RefDecl,
};
use crate::parser::expr::path_expression;
use crate::parser::lex::{identification, name, qualified_name, skip_until_brace_end, ws1, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::with_span;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::preceded;
use nom::Parser;
use nom::IResult;

fn end_decl(input: Input<'_>) -> IResult<Input<'_>, Node<EndDecl>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"end"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, (name_span, name_str)) = with_span(name).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, (type_ref_span, type_name)) =
        preceded(ws_and_comments, with_span(qualified_name)).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            EndDecl {
                name: name_str,
                type_name,
                name_span: Some(name_span),
                type_ref_span: Some(type_ref_span),
            },
        ),
    ))
}

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
                body,
                name_span: Some(name_span),
                type_ref_span: Some(type_ref_span),
            },
        ),
    ))
}

fn connect_body(input: Input<'_>) -> IResult<Input<'_>, crate::ast::ConnectBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| crate::ast::ConnectBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| crate::ast::ConnectBody::Brace,
        ),
    ))
    .parse(input)
}

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

fn connection_def_body_element(
    input: Input<'_>,
) -> IResult<Input<'_>, Node<ConnectionDefBodyElement>> {
    let (input, _) = ws_and_comments(input)?;
    let start = input;
    let (input, elem) = alt((
        map(end_decl, ConnectionDefBodyElement::EndDecl),
        map(ref_decl, ConnectionDefBodyElement::RefDecl),
        map(connect_stmt, ConnectionDefBodyElement::ConnectStmt),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

fn connection_def_body(input: Input<'_>) -> IResult<Input<'_>, ConnectionDefBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| ConnectionDefBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                preceded(
                    ws_and_comments,
                    many0(preceded(ws_and_comments, connection_def_body_element)),
                ),
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |elements| ConnectionDefBody::Brace { elements },
        ),
    ))
    .parse(input)
}

/// Connection definition: `connection def` Identification body.
pub(crate) fn connection_def(input: Input<'_>) -> IResult<Input<'_>, Node<ConnectionDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"connection"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, body) = connection_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            ConnectionDef {
                identification,
                body,
            },
        ),
    ))
}
