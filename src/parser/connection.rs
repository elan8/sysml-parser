//! Connection definition parsing (BNF ConnectionDefinition).
#![allow(dead_code, unused_imports)]

use crate::ast::{
    ConnectStmt, ConnectionDef, ConnectionDefBody, ConnectionDefBodyElement, EndDecl, Node,
    RefBody, RefDecl,
};
use crate::parser::expr::path_expression;
use crate::parser::lex::{
    identification, name, qualified_name, recover_body_element, skip_until_brace_end,
    starts_with_any_keyword, take_until_terminator, ws1, ws_and_comments,
    CONNECTION_DEF_BODY_STARTERS,
};
use crate::parser::node_from_to;
use crate::parser::with_span;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::combinator::{map, opt};
use nom::multi::many0;
use nom::sequence::preceded;
use nom::IResult;
use nom::Parser;

fn derived_end_name(input: Input<'_>) -> IResult<Input<'_>, String> {
    let (input, _) = tag(&b"#"[..]).parse(input)?;
    let (input, value) =
        take_while1(|c: u8| c.is_ascii_alphanumeric() || c == b'_').parse(input)?;
    Ok((
        input,
        format!("#{}", String::from_utf8_lossy(value.fragment())),
    ))
}

fn end_decl(input: Input<'_>) -> IResult<Input<'_>, Node<EndDecl>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"end"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, (name_span, name_str)) =
        with_span(|input| alt((derived_end_name, name)).parse(input)).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, uses_derived_syntax) = if let Ok((input, _)) =
        tag::<_, _, nom::error::Error<Input<'_>>>(&b"::>"[..]).parse(input)
    {
        (input, true)
    } else {
        let (input, _) = tag(&b":"[..]).parse(input)?;
        (input, false)
    };
    let (input, (type_ref_span, type_name)) = if uses_derived_syntax {
        let (input, _) = ws_and_comments(input)?;
        let start_type = input;
        let (input, value) =
            take_while1(|c: u8| c != b';' && c != b'\n' && c != b'\r').parse(input)?;
        let type_name = String::from_utf8_lossy(value.fragment()).trim().to_string();
        let span = crate::ast::Span {
            offset: start_type.location_offset(),
            line: start_type.location_line(),
            column: start_type.get_column(),
            len: value.fragment().len(),
        };
        (input, (span, type_name))
    } else {
        preceded(ws_and_comments, with_span(qualified_name)).parse(input)?
    };
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            EndDecl {
                name: name_str,
                type_name,
                uses_derived_syntax,
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
                value: None,
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
    let (mut input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b";") {
        let (input, _) = tag(&b";"[..]).parse(input)?;
        return Ok((input, ConnectionDefBody::Semicolon));
    }
    let (next, _) = tag(&b"{"[..]).parse(input)?;
    input = next;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().starts_with(b"}") {
            let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
            return Ok((input, ConnectionDefBody::Brace { elements }));
        }
        match connection_def_body_element(input) {
            Ok((next, element)) => {
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                elements.push(element);
                input = next;
            }
            Err(_) if starts_with_any_keyword(input.fragment(), CONNECTION_DEF_BODY_STARTERS) => {
                let (next, _) = recover_body_element(input, CONNECTION_DEF_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                input = next;
            }
            Err(_) => {
                let (input, _) = skip_until_brace_end(input)?;
                let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
                return Ok((input, ConnectionDefBody::Brace { elements }));
            }
        }
    }
}

/// Connection definition: `connection def` Identification body.
pub(crate) fn connection_def(input: Input<'_>) -> IResult<Input<'_>, Node<ConnectionDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, annotation) = opt(preceded(
        tag(&b"#"[..]),
        take_while1(|c: u8| c.is_ascii_alphanumeric() || c == b'_'),
    ))
    .parse(input)?;
    let annotation = annotation.map(|raw| String::from_utf8_lossy(raw.fragment()).to_string());
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"connection"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"def"[..]), ws1)).parse(input)?;
    let (input, identification) = identification(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = connection_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            ConnectionDef {
                annotation,
                identification,
                body,
            },
        ),
    ))
}
