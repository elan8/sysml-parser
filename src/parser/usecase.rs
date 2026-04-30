#![allow(dead_code, unused_imports)]

use crate::ast::{
    ActorDecl, ActorRedefinitionAssignment, ActorUsage, FirstSuccession, IncludeUseCase, Node,
    Objective, ParseErrorNode, RefRedefinition, RequirementUsage, ReturnRef, SubjectRef, ThenDone,
    ThenIncludeUseCase, ThenUseCaseUsage, UseCaseDef, UseCaseDefBody, UseCaseDefBodyElement,
    UseCaseUsage, Visibility,
};
use crate::parser::lex::{
    identification, name, qualified_name, recover_body_element, skip_statement_or_block,
    skip_until_brace_end, starts_with_any_keyword, take_until_terminator, ws1, ws_and_comments,
    USE_CASE_BODY_STARTERS,
};
use crate::parser::node_from_to;
use crate::parser::requirement::{doc_comment, parse_requirement_usage_payload, subject_decl};
use crate::parser::Input;
use crate::parser::{build_recovery_error_node, build_recovery_error_node_from_span};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::sequence::preceded;
use nom::{IResult, Parser};

fn slice_text(start: Input<'_>, end: Input<'_>) -> String {
    let delta = end
        .location_offset()
        .saturating_sub(start.location_offset());
    let bytes = start.fragment();
    let take = delta.min(bytes.len());
    String::from_utf8_lossy(&bytes[..take]).trim().to_string()
}

fn multiplicity(input: Input<'_>) -> IResult<Input<'_>, String> {
    let (input, _) = ws_and_comments(input)?;
    let frag = input.fragment();
    if !frag.starts_with(b"[") {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }
    // Reuse the generic block skipper by treating the [...] as a statement up to a `]`.
    let mut i = 0usize;
    while i < frag.len() && frag[i] != b']' {
        i += 1;
    }
    if i >= frag.len() {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }
    let s = String::from_utf8_lossy(&frag[..=i]).trim().to_string();
    let (input, _) = nom::bytes::complete::take(i + 1usize).parse(input)?;
    Ok((input, s))
}

fn subject_ref(input: Input<'_>) -> IResult<Input<'_>, Node<SubjectRef>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"subject"[..])).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((input, node_from_to(start, input, SubjectRef {})))
}

fn first_succession(input: Input<'_>) -> IResult<Input<'_>, Node<FirstSuccession>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"first"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, target) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((
        input,
        node_from_to(start, input, FirstSuccession { target }),
    ))
}

fn then_done(input: Input<'_>) -> IResult<Input<'_>, Node<ThenDone>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"then"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"done"[..]).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((input, node_from_to(start, input, ThenDone {})))
}

fn include_use_case(input: Input<'_>) -> IResult<Input<'_>, Node<IncludeUseCase>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"include"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, n) = name(input)?;
    let (input, mult) = opt(multiplicity).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, body) = use_case_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            IncludeUseCase {
                name: n,
                multiplicity: mult,
                body,
            },
        ),
    ))
}

fn then_include_use_case(input: Input<'_>) -> IResult<Input<'_>, Node<ThenIncludeUseCase>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"then"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, include) = include_use_case(input)?;
    Ok((
        input,
        node_from_to(start, input, ThenIncludeUseCase { include }),
    ))
}

fn use_case_usage_in_body(input: Input<'_>) -> IResult<Input<'_>, Node<UseCaseUsage>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"use"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"case"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, ident) = name(input)?;
    let (input, type_name) = {
        let (peek, _) = ws_and_comments(input)?;
        if peek.fragment().starts_with(b":") && !peek.fragment().starts_with(b":>") {
            let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
            let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
            (input, Some(type_name))
        } else {
            (input, None)
        }
    };
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = use_case_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            UseCaseUsage {
                name: ident,
                type_name,
                body,
            },
        ),
    ))
}

fn then_use_case_usage(input: Input<'_>) -> IResult<Input<'_>, Node<ThenUseCaseUsage>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"then"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, use_case) = use_case_usage_in_body(input)?;
    Ok((
        input,
        node_from_to(start, input, ThenUseCaseUsage { use_case }),
    ))
}

fn actor_redefinition_assignment(
    input: Input<'_>,
) -> IResult<Input<'_>, Node<ActorRedefinitionAssignment>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"actor"[..])).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b":>>"[..]).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, n) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"="[..])).parse(input)?;
    let (input, rhs) = take_until_terminator(input, b";")?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((
        input,
        node_from_to(start, input, ActorRedefinitionAssignment { name: n, rhs }),
    ))
}

fn ref_redefinition(input: Input<'_>) -> IResult<Input<'_>, Node<RefRedefinition>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"ref"[..])).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b":>>"[..]).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, n) = name(input)?;
    let (input, _) = ws_and_comments(input)?;
    let body_start = input;
    let (input, _) = skip_statement_or_block(input)?;
    let body = slice_text(body_start, input);
    Ok((
        input,
        node_from_to(start, input, RefRedefinition { name: n, body }),
    ))
}

fn return_ref(input: Input<'_>) -> IResult<Input<'_>, Node<ReturnRef>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"return"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"ref"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, n) = name(input)?;
    let (input, mult) = opt(multiplicity).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let body_start = input;
    let (input, _) = skip_statement_or_block(input)?;
    let body = slice_text(body_start, input);
    Ok((
        input,
        node_from_to(
            start,
            input,
            ReturnRef {
                name: n,
                multiplicity: mult,
                body,
            },
        ),
    ))
}

fn other_use_case_body_element(input: Input<'_>) -> IResult<Input<'_>, UseCaseDefBodyElement> {
    let (input, _) = ws_and_comments(input)?;
    let start_after_ws = input;

    // If this looks like a genuine syntax error we have a targeted diagnostic for (e.g. `actor: User;`),
    // let the body recovery path create an `Error` element so `parse_with_diagnostics` surfaces it.
    let trimmed = start_after_ws.fragment();
    let is_redefinition = trimmed.windows(3).any(|w| w == b":>>");
    let diag = build_recovery_error_node(
        start_after_ws,
        USE_CASE_BODY_STARTERS,
        "use case body",
        "recovered_use_case_body_element",
    );
    if matches!(
        diag.code.as_str(),
        "missing_member_name"
            | "missing_type_reference"
            | "unexpected_keyword_in_scope"
            | "missing_expression_after_operator"
            | "unsupported_annotation_syntax"
    ) && !is_redefinition
    {
        return Err(nom::Err::Error(nom::error::Error::new(
            start_after_ws,
            nom::error::ErrorKind::Tag,
        )));
    }

    let (input, _) = skip_statement_or_block(input)?;
    if input.location_offset() == start_after_ws.location_offset() {
        return Err(nom::Err::Error(nom::error::Error::new(
            start_after_ws,
            nom::error::ErrorKind::Many0,
        )));
    }
    let frag = start_after_ws.fragment();
    let take = frag.len().min(80);
    let preview = String::from_utf8_lossy(&frag[..take]).trim().to_string();
    Ok((input, UseCaseDefBodyElement::Other(preview)))
}

pub(crate) fn actor_decl(input: Input<'_>) -> IResult<Input<'_>, Node<ActorDecl>> {
    let start = input;
    let (input, _) = tag(&b"actor"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, ident) = identification(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            ActorDecl {
                identification: ident,
            },
        ),
    ))
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
    let (input, type_name) = {
        let (peek, _) = ws_and_comments(input)?;
        if peek.fragment().starts_with(b":") && !peek.fragment().starts_with(b":>") {
            let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
            let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
            (input, Some(type_name))
        } else {
            (input, None)
        }
    };
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = use_case_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            UseCaseUsage {
                name: ident,
                type_name,
                body,
            },
        ),
    ))
}

pub(crate) fn use_case_def(input: Input<'_>) -> IResult<Input<'_>, Node<UseCaseDef>> {
    let start = input;
    let (input, _) = keyword_use_case_def(input)?;
    let (input, ident) = identification(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = use_case_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            UseCaseDef {
                identification: ident,
                body,
            },
        ),
    ))
}

pub(crate) fn use_case_def_body(input: Input<'_>) -> IResult<Input<'_>, UseCaseDefBody> {
    alt((
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| {
            UseCaseDefBody::Semicolon
        }),
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
        if input.fragment().is_empty() {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }
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
            Err(_) => {
                // Library analysis-case bodies contain many constructs we don't model yet (e.g. `objective name : Type { ... }`,
                // feature redefinitions with `:>>`, nested calcs/returns). Skip one statement/block to keep parsing stable
                // but still emit a recoverable diagnostic for malformed or unsupported members.
                let start_unknown = input;
                let (next, _) = recover_body_element(input, USE_CASE_BODY_STARTERS)?;
                if next.location_offset() == start_unknown.location_offset() {
                    // Fall back to aborting this body to avoid infinite loops.
                    let (input, _) = skip_until_brace_end(input)?;
                    let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
                    return Ok((input, UseCaseDefBody::Brace { elements }));
                }
                // Emit diagnostics only for likely-user mistakes. The SysML v2 release libraries and
                // validation fixtures use many valid constructs we don't fully model yet (notably `:>>`
                // redefinitions); those should be captured as `Other` without diagnostics so strict
                // suites can remain diagnostic-free.
                let trimmed = start_unknown.fragment();
                let is_redefinition = trimmed.windows(3).any(|w| w == b":>>");
                let recovery = build_recovery_error_node_from_span(
                    start_unknown,
                    next,
                    USE_CASE_BODY_STARTERS,
                    "use case body",
                    "recovered_use_case_body_element",
                );
                let should_error = matches!(
                    recovery.code.as_str(),
                    "missing_member_name" | "missing_type_reference"
                ) && !is_redefinition;

                if should_error {
                    let node: Node<ParseErrorNode> = node_from_to(start_unknown, next, recovery);
                    elements.push(node_from_to(
                        start_unknown,
                        next,
                        UseCaseDefBodyElement::Error(node),
                    ));
                } else {
                    let frag = start_unknown.fragment();
                    let take = frag.len().min(80);
                    let preview = String::from_utf8_lossy(&frag[..take]).trim().to_string();
                    elements.push(node_from_to(
                        start_unknown,
                        next,
                        UseCaseDefBodyElement::Other(preview),
                    ));
                }
                input = next;
            }
        }
    }
}

pub(crate) fn use_case_def_body_element(
    input: Input<'_>,
) -> IResult<Input<'_>, Node<UseCaseDefBodyElement>> {
    let (input, _) = ws_and_comments(input)?;
    let start = input;
    let (input, elem) = alt((
        map(doc_comment, UseCaseDefBodyElement::Doc),
        map(subject_decl, UseCaseDefBodyElement::SubjectDecl),
        map(subject_ref, UseCaseDefBodyElement::SubjectRef),
        map(actor_usage, UseCaseDefBodyElement::ActorUsage),
        map(
            actor_redefinition_assignment,
            UseCaseDefBodyElement::ActorRedefinitionAssignment,
        ),
        map(objective, UseCaseDefBodyElement::Objective),
        map(first_succession, UseCaseDefBodyElement::FirstSuccession),
        map(then_done, UseCaseDefBodyElement::ThenDone),
        map(
            then_include_use_case,
            UseCaseDefBodyElement::ThenIncludeUseCase,
        ),
        map(then_use_case_usage, UseCaseDefBodyElement::ThenUseCaseUsage),
        map(include_use_case, UseCaseDefBodyElement::IncludeUseCase),
        map(ref_redefinition, UseCaseDefBodyElement::RefRedefinition),
        map(return_ref, UseCaseDefBodyElement::ReturnRef),
        map(
            crate::parser::action::assign_stmt,
            UseCaseDefBodyElement::Assign,
        ),
        map(
            crate::parser::action::for_loop,
            UseCaseDefBodyElement::ForLoop,
        ),
        map(
            crate::parser::action::then_action,
            UseCaseDefBodyElement::ThenAction,
        ),
        other_use_case_body_element,
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
    Ok((
        input,
        node_from_to(start, input, ActorUsage { name: n, type_name }),
    ))
}

pub(crate) fn objective(input: Input<'_>) -> IResult<Input<'_>, Node<Objective>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, visibility) = opt(alt((
        map(preceded(tag(&b"private"[..]), ws1), |_| Visibility::Private),
        map(preceded(tag(&b"protected"[..]), ws1), |_| Visibility::Protected),
        map(preceded(tag(&b"public"[..]), ws1), |_| Visibility::Public),
    )))
    .parse(input)?;
    let (input, _) = tag(&b"objective"[..]).parse(input)?;
    let (input, requirement) = parse_requirement_usage_payload(input, Some("objective"))?;
    let requirement = node_from_to(start, input, requirement);
    Ok((
        input,
        node_from_to(
            start,
            input,
            Objective {
                visibility,
                requirement,
            },
        ),
    ))
}
