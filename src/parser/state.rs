#![allow(dead_code, unused_imports)]

use crate::ast::{
    EntryAction, Node, ParseErrorNode, RefBody, RefDecl, StateDef, StateDefBody,
    StateDefBodyElement, StateUsage, ThenStmt, Transition,
};
use crate::parser::requirement::doc_comment;
use crate::parser::expr::expression;
use crate::parser::lex::{
    identification, name, qualified_name, recover_body_element, skip_until_brace_end,
    starts_with_any_keyword, take_until_terminator, ws1, ws_and_comments, STATE_BODY_STARTERS,
};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::sequence::{delimited, preceded};
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

fn keyword_state_def(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
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
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = state_def_body(input)?;
    Ok((input, node_from_to(start, input, StateDef { identification: ident, body })))
}

fn state_def_body(input: Input<'_>) -> IResult<Input<'_>, StateDefBody> {
    alt((
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| StateDefBody::Semicolon),
        state_def_body_brace,
    ))
    .parse(input)
}

fn state_def_body_brace(input: Input<'_>) -> IResult<Input<'_>, StateDefBody> {
    let (input, _) = preceded(ws_and_comments, tag(&b"{"[..])).parse(input)?;
    let (input, _) = skip_until_brace_end(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
    Ok((input, StateDefBody::Brace { elements: vec![] }))
}

/// Entry action: `entry` (`;` or body)  or  `entry action` name body
fn entry_action(input: Input<'_>) -> IResult<Input<'_>, Node<EntryAction>> {
    let start = input;
    let (input, _) = tag(&b"entry"[..]).parse(input)?;
    let (input, action_name) = opt((
        preceded(ws_and_comments, tag(&b"action"[..])),
        preceded(ws1, name),
    ))
    .parse(input)?;
    let action_name = action_name.map(|(_, n)| n);
    let (input, body) = state_def_body(input)?;
    Ok((
        input,
        node_from_to(start, input, EntryAction { action_name, body }),
    ))
}

/// Ref in state body: `ref` name `:` type body
fn state_ref(input: Input<'_>) -> IResult<Input<'_>, Node<RefDecl>> {
    let start = input;
    let (input, _) = tag(&b"ref"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
    let (input, value) = opt(preceded(
        preceded(ws_and_comments, tag(&b"="[..])),
        preceded(ws_and_comments, expression),
    ))
    .parse(input)?;
    let (input, body) = preceded(
        ws_and_comments,
        alt((
            map(tag(&b";"[..]), |_| RefBody::Semicolon),
            map(
                delimited(
                    tag(&b"{"[..]),
                    skip_until_brace_end,
                    preceded(ws_and_comments, tag(&b"}"[..])),
                ),
                |_| RefBody::Brace,
            ),
        )),
    )
    .parse(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            RefDecl {
                name: name_str,
                type_name,
                value,
                body,
                name_span: None,
                type_ref_span: None,
            },
        ),
    ))
}

/// Then (initial state): `then` name `;`
fn then_stmt(input: Input<'_>) -> IResult<Input<'_>, Node<ThenStmt>> {
    let start = input;
    let (input, _) = tag(&b"then"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, state_name) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((input, node_from_to(start, input, ThenStmt { state_name })))
}

fn state_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<StateDefBodyElement>> {
    let start = input;
    let mut parser = alt((
        map(doc_comment, |n| node_from_to(start, input, StateDefBodyElement::Doc(n))),
        map(entry_action, |n| node_from_to(start, input, StateDefBodyElement::Entry(n))),
        map(then_stmt, |n| node_from_to(start, input, StateDefBodyElement::Then(n))),
        map(state_ref, |n| node_from_to(start, input, StateDefBodyElement::Ref(n))),
        map(state_usage, |n| node_from_to(start, input, StateDefBodyElement::StateUsage(n))),
        map(transition, |n| node_from_to(start, input, StateDefBodyElement::Transition(n))),
    ));
    parser.parse(input)
}

pub(crate) fn state_usage(input: Input<'_>) -> IResult<Input<'_>, Node<StateUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"state"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, n) = name(input)?;
    let (input, typ) = opt(preceded(preceded(ws_and_comments, tag(&b":"[..])), preceded(ws_and_comments, qualified_name))).parse(input)?;
    // Optional modifier before body: `parallel` or `initial` (SysML state usage)
    let (input, _) = opt(alt((
        preceded(preceded(ws_and_comments, tag(&b"parallel"[..])), ws1),
        preceded(preceded(ws_and_comments, tag(&b"initial"[..])), ws1),
    )))
    .parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = state_def_body(input)?;
    Ok((input, node_from_to(start, input, StateUsage { name: n, type_name: typ, body })))
}

pub(crate) fn transition(input: Input<'_>) -> IResult<Input<'_>, Node<Transition>> {
    let start = input;
    let (input, _) = tag(&b"transition"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, n) = name(input)?;
    // Optional: `first` source (simplified form is `transition name then target;`)
    let (input, source) = opt((
        preceded(ws_and_comments, tag(&b"first"[..])),
        ws1,
        expression,
        // Optional: `accept` trigger expression (e.g. `accept PhaseTimerElapsed`)
        opt((
            preceded(ws_and_comments, tag(&b"accept"[..])),
            preceded(ws1, expression),
        )),
    ))
    .parse(input)?;
    let source = source.map(|(_, _, expr, _)| expr);
    // Optional: `if` guard and `do` effect before `then`
    let (input, guard) = opt((
        preceded(ws_and_comments, tag(&b"if"[..])),
        preceded(ws1, expression),
    ))
    .parse(input)?;
    let guard = guard.map(|(_, expr)| expr);
    let (input, effect) = opt((
        preceded(ws_and_comments, tag(&b"do"[..])),
        preceded(ws1, expression),
    ))
    .parse(input)?;
    let effect = effect.map(|(_, expr)| expr);
    let (input, _) = preceded(ws_and_comments, tag(&b"then"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, target) = expression(input)?;
    let (input, body) = preceded(
        ws_and_comments,
        alt((
            map(tag(&b";"[..]), |_| crate::ast::ConnectBody::Semicolon),
            map(
                delimited(
                    tag(&b"{"[..]),
                    skip_until_brace_end,
                    preceded(ws_and_comments, tag(&b"}"[..])),
                ),
                |_| crate::ast::ConnectBody::Brace,
            ),
        )),
    )
    .parse(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            Transition {
                name: n,
                source,
                guard,
                effect,
                target,
                body,
            },
        ),
    ))
}
