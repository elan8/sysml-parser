use crate::ast::{
    Node, ActorDecl, ActorUsage, Objective, UseCaseDef, UseCaseDefBody, UseCaseDefBodyElement
};
use crate::parser::lex::{identification, name, ws1, ws_and_comments, qualified_name};
use crate::parser::requirement::{doc_comment, subject_decl};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};

pub(crate) fn actor_decl(input: Input<'_>) -> IResult<Input<'_>, Node<ActorDecl>> {
    let start = input;
    let (input, _) = tag(&b"actor"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, ident) = identification(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((input, node_from_to(start, input, ActorDecl { identification: ident })))
}

fn keyword_use_case_def(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = tag(&b"use"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"case"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    Ok((input, ()))
}

pub(crate) fn use_case_def(input: Input<'_>) -> IResult<Input<'_>, Node<UseCaseDef>> {
    let start = input;
    let (input, _) = keyword_use_case_def(input)?;
    let (input, ident) = identification(input)?;
    let (input, body) = use_case_def_body(input)?;
    Ok((input, node_from_to(start, input, UseCaseDef { identification: ident, body })))
}

fn use_case_def_body(input: Input<'_>) -> IResult<Input<'_>, UseCaseDefBody> {
    alt((
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| UseCaseDefBody::Semicolon),
        map(
            delimited(
                preceded(ws_and_comments, tag(&b"{"[..])),
                many0(preceded(ws_and_comments, use_case_def_body_element)),
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |elements| UseCaseDefBody::Brace { elements },
        ),
    ))
    .parse(input)
}

fn use_case_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<UseCaseDefBodyElement>> {
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
