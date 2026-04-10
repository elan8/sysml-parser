#![allow(dead_code, unused_imports)]

use crate::ast::{
    EntryAction, Node, RefBody, RefDecl, StateDef, StateDefBody, StateDefBodyElement, StateUsage,
    ThenStmt, Transition,
};
use crate::parser::build_recovery_error_node;
use crate::parser::expr::expression;
use crate::parser::lex::{
    identification, name, qualified_name, recover_body_element, skip_statement_or_block,
    skip_until_brace_end, starts_with_any_keyword, take_until_terminator, ws1, ws_and_comments,
    STATE_BODY_STARTERS,
};
use crate::parser::node_from_to;
use crate::parser::metadata_annotation::annotation;
use crate::parser::requirement::{doc_comment, requirement_usage};
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};

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
    Ok((
        input,
        node_from_to(
            start,
            input,
            StateDef {
                identification: ident,
                body,
            },
        ),
    ))
}

pub(crate) fn state_def_body(input: Input<'_>) -> IResult<Input<'_>, StateDefBody> {
    alt((
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| {
            StateDefBody::Semicolon
        }),
        state_def_body_brace,
    ))
    .parse(input)
}

fn state_def_body_brace(input: Input<'_>) -> IResult<Input<'_>, StateDefBody> {
    let (mut input, _) = preceded(ws_and_comments, tag(&b"{"[..])).parse(input)?;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().is_empty() {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }
        if input.fragment().starts_with(b"}") {
            let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
            return Ok((input, StateDefBody::Brace { elements }));
        }
        match state_def_body_element(input) {
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
            Err(_) if starts_with_any_keyword(input.fragment(), STATE_BODY_STARTERS) => {
                let (next, _) = recover_body_element(input, STATE_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                elements.push(node_from_to(
                    input,
                    next,
                    StateDefBodyElement::Error(Node::new(
                        crate::ast::Span::dummy(),
                        build_recovery_error_node(
                            input,
                            STATE_BODY_STARTERS,
                            "state body",
                            "recovered_state_body_element",
                        ),
                    )),
                ));
                input = next;
            }
            Err(_) => {
                let start_unknown = input;
                if let Ok((next, _)) = skip_statement_or_block(input) {
                    if next.location_offset() != start_unknown.location_offset() {
                        let frag = start_unknown.fragment();
                        let take = frag.len().min(80);
                        let preview = String::from_utf8_lossy(&frag[..take]).trim().to_string();
                        elements.push(node_from_to(
                            start_unknown,
                            next,
                            StateDefBodyElement::Other(preview),
                        ));
                        input = next;
                        continue;
                    }
                }
                let recovery = build_recovery_error_node(
                    start_unknown,
                    STATE_BODY_STARTERS,
                    "state body",
                    "recovered_state_body_element",
                );
                let (next, _) = recover_body_element(input, STATE_BODY_STARTERS)?;
                if next.location_offset() == start_unknown.location_offset() {
                    let (input, _) = skip_until_brace_end(input)?;
                    let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
                    return Ok((input, StateDefBody::Brace { elements }));
                }
                if matches!(
                    recovery.code.as_str(),
                    "missing_member_name" | "missing_type_reference"
                ) {
                    elements.push(node_from_to(
                        start_unknown,
                        next,
                        StateDefBodyElement::Error(Node::new(crate::ast::Span::dummy(), recovery)),
                    ));
                } else {
                    let frag = start_unknown.fragment();
                    let take = frag.len().min(80);
                    let preview = String::from_utf8_lossy(&frag[..take]).trim().to_string();
                    elements.push(node_from_to(
                        start_unknown,
                        next,
                        StateDefBodyElement::Other(preview),
                    ));
                }
                input = next;
            }
        }
    }
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
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
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
    let (input, _) = opt(preceded(ws1, tag(&b"state"[..]))).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
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
                value: None,
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
        map(doc_comment, |n| {
            node_from_to(start, input, StateDefBodyElement::Doc(n))
        }),
        map(annotation, |n| {
            node_from_to(start, input, StateDefBodyElement::Annotation(n))
        }),
        map(entry_action, |n| {
            node_from_to(start, input, StateDefBodyElement::Entry(n))
        }),
        map(then_stmt, |n| {
            node_from_to(start, input, StateDefBodyElement::Then(n))
        }),
        map(state_ref, |n| {
            node_from_to(start, input, StateDefBodyElement::Ref(n))
        }),
        map(requirement_usage, |n| {
            node_from_to(start, input, StateDefBodyElement::RequirementUsage(n))
        }),
        map(state_usage, |n| {
            node_from_to(start, input, StateDefBodyElement::StateUsage(n))
        }),
        map(transition, |n| {
            node_from_to(start, input, StateDefBodyElement::Transition(n))
        }),
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
    let (input, typ) = {
        let (peek, _) = ws_and_comments(input)?;
        if peek.fragment().starts_with(b":") && !peek.fragment().starts_with(b":>") {
            let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
            let (input, typ) = preceded(ws_and_comments, qualified_name).parse(input)?;
            (input, Some(typ))
        } else {
            (input, None)
        }
    };
    // Optional modifier before body: `parallel` or `initial` (SysML state usage)
    let (input, _) = opt(alt((
        preceded(preceded(ws_and_comments, tag(&b"parallel"[..])), ws1),
        preceded(preceded(ws_and_comments, tag(&b"initial"[..])), ws1),
    )))
    .parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = state_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            StateUsage {
                name: n,
                type_name: typ,
                body,
            },
        ),
    ))
}

pub(crate) fn transition(input: Input<'_>) -> IResult<Input<'_>, Node<Transition>> {
    let start = input;
    let (input, _) = tag(&b"transition"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, n) = {
        let (peek, _) = ws_and_comments(input)?;
        if peek.fragment().starts_with(b"first")
            || peek.fragment().starts_with(b"if")
            || peek.fragment().starts_with(b"do")
            || peek.fragment().starts_with(b"then")
        {
            (input, None)
        } else {
            let (input, n) = name(input)?;
            (input, Some(n))
        }
    };
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
