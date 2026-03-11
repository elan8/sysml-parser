use crate::ast::{
    Node, StateDef, StateDefBody, StateDefBodyElement, StateUsage, Transition
};
use crate::parser::expr::expression;
use crate::parser::lex::{identification, name, ws1, ws_and_comments, qualified_name};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};

fn keyword_state_def(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = tag(&b"state"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    Ok((input, ()))
}

pub(crate) fn state_def(input: Input<'_>) -> IResult<Input<'_>, Node<StateDef>> {
    let start = input;
    let (input, _) = keyword_state_def(input)?;
    let (input, ident) = identification(input)?;
    let (input, body) = state_def_body(input)?;
    Ok((input, node_from_to(start, input, StateDef { identification: ident, body })))
}

fn state_def_body(input: Input<'_>) -> IResult<Input<'_>, StateDefBody> {
    alt((
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| StateDefBody::Semicolon),
        map(
            delimited(
                preceded(ws_and_comments, tag(&b"{"[..])),
                many0(preceded(ws_and_comments, state_def_body_element)),
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |elements| StateDefBody::Brace { elements },
        ),
    ))
    .parse(input)
}

fn state_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<StateDefBodyElement>> {
    let start = input;
    let mut parser = alt((
        map(state_usage, |n| node_from_to(start, input, StateDefBodyElement::StateUsage(n))),
        map(transition, |n| node_from_to(start, input, StateDefBodyElement::Transition(n))),
    ));
    parser.parse(input)
}

pub(crate) fn state_usage(input: Input<'_>) -> IResult<Input<'_>, Node<StateUsage>> {
    let start = input;
    let (input, _) = tag(&b"state"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, n) = name(input)?;
    let (input, typ) = opt(preceded(preceded(ws_and_comments, tag(&b":"[..])), preceded(ws_and_comments, qualified_name))).parse(input)?;
    let (input, body) = state_def_body(input)?;
    Ok((input, node_from_to(start, input, StateUsage { name: n, type_name: typ, body })))
}

pub(crate) fn transition(input: Input<'_>) -> IResult<Input<'_>, Node<Transition>> {
    let start = input;
    let (input, _) = tag(&b"transition"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, n) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"first"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, source) = expression(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"then"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, target) = expression(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((input, node_from_to(start, input, Transition { name: n, source, target, body: crate::ast::ConnectBody::Semicolon })))
}
