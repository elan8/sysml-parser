#![allow(dead_code, unused_imports)]

use crate::ast::{
    Node, ActorDecl, ActorUsage, Objective, ParseErrorNode, UseCaseDef, UseCaseDefBody,
    UseCaseDefBodyElement, UseCaseUsage,
};
use crate::parser::lex::{
    identification, looks_like_missing_semicolon, name, qualified_name, recover_body_element,
    starts_with_any_keyword,
    skip_until_brace_end, take_until_terminator, ws1, ws_and_comments, USE_CASE_BODY_STARTERS,
};
use crate::parser::requirement::{doc_comment, subject_decl};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::sequence::preceded;
use nom::{IResult, Parser};

fn recovery_found_snippet(input: Input<'_>) -> Option<String> {
    let frag = input.fragment();
    let take = frag
        .iter()
        .position(|&b| b == b'\n' || b == b'\r')
        .unwrap_or(frag.len())
        .min(60);
    let snippet = String::from_utf8_lossy(&frag[..take]).trim().to_string();
    if snippet.is_empty() { None } else { Some(snippet) }
}

pub(crate) fn actor_decl(input: Input<'_>) -> IResult<Input<'_>, Node<ActorDecl>> {
    let start = input;
    let (input, _) = tag(&b"actor"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, ident) = identification(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((input, node_from_to(start, input, ActorDecl { identification: ident })))
}

fn keyword_use_case_def(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"use"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"case"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    Ok((input, ()))
}

/// use case name ( : type )? CaseBody
pub(crate) fn use_case_usage(input: Input<'_>) -> IResult<Input<'_>, Node<UseCaseUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"use"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"case"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, ident) = name(input)?;
    let (input, type_name) = opt(preceded(
        preceded(ws_and_comments, tag(&b":"[..])),
        preceded(ws_and_comments, qualified_name),
    ))
    .parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = use_case_def_body(input)?;
    Ok((
        input,
        node_from_to(start, input, UseCaseUsage {
            name: ident,
            type_name,
            body,
        }),
    ))
}

pub(crate) fn use_case_def(input: Input<'_>) -> IResult<Input<'_>, Node<UseCaseDef>> {
    let start = input;
    let (input, _) = keyword_use_case_def(input)?;
    let (input, ident) = identification(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = use_case_def_body(input)?;
    Ok((input, node_from_to(start, input, UseCaseDef { identification: ident, body })))
}

pub(crate) fn use_case_def_body(input: Input<'_>) -> IResult<Input<'_>, UseCaseDefBody> {
    alt((
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| UseCaseDefBody::Semicolon),
        use_case_def_body_brace,
    ))
    .parse(input)
}

fn use_case_def_body_brace(input: Input<'_>) -> IResult<Input<'_>, UseCaseDefBody> {
    let (mut input, _) = preceded(ws_and_comments, tag(&b"{"[..])).parse(input)?;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().starts_with(b"}") {
            let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
            return Ok((input, UseCaseDefBody::Brace { elements }));
        }
        match use_case_def_body_element(input) {
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
            Err(_) if starts_with_any_keyword(input.fragment(), USE_CASE_BODY_STARTERS) => {
                let (next, _) = recover_body_element(input, USE_CASE_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                elements.push(node_from_to(
                    input,
                    next,
                    UseCaseDefBodyElement::Error(Node::new(
                        crate::ast::Span::dummy(),
                        if looks_like_missing_semicolon(input, USE_CASE_BODY_STARTERS) {
                            ParseErrorNode {
                                message: "missing semicolon before next declaration".to_string(),
                                code: "missing_semicolon".to_string(),
                                expected: Some("';'".to_string()),
                                found: recovery_found_snippet(input),
                                suggestion: Some("Insert ';' before this declaration.".to_string()),
                            }
                        } else {
                            ParseErrorNode {
                                message: "recovered use case body element".to_string(),
                                code: "recovered_use_case_body_element".to_string(),
                                expected: Some("valid use case body element".to_string()),
                                found: recovery_found_snippet(input),
                                suggestion: Some(
                                    "Fix this use case member and re-run parsing.".to_string(),
                                ),
                            }
                        },
                    )),
                ));
                input = next;
            }
            Err(_) => {
                let (input, _) = skip_until_brace_end(input)?;
                let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
                return Ok((input, UseCaseDefBody::Brace { elements }));
            }
        }
    }
}

pub(crate) fn use_case_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<UseCaseDefBodyElement>> {
    let (input, _) = ws_and_comments(input)?;
    let start = input;
    let (input, elem) = alt((
        map(doc_comment, UseCaseDefBodyElement::Doc),
        map(subject_decl, UseCaseDefBodyElement::SubjectDecl),
        map(actor_usage, UseCaseDefBodyElement::ActorUsage),
        map(objective, UseCaseDefBodyElement::Objective),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

pub(crate) fn actor_usage(input: Input<'_>) -> IResult<Input<'_>, Node<ActorUsage>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"actor"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, n) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((input, node_from_to(start, input, ActorUsage { name: n, type_name })))
}

pub(crate) fn objective(input: Input<'_>) -> IResult<Input<'_>, Node<Objective>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"objective"[..])).parse(input)?;
    let (input, body) = crate::parser::requirement::constraint_body(input)?;
    Ok((input, node_from_to(start, input, Objective { body })))
}
