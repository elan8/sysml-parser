//! Action definition and action usage parsing (function-based behavior).

use crate::ast::{
    ActionDef, ActionDefBody, ActionUsage, ActionUsageBody, ActionUsageBodyElement, FirstMergeBody,
    FirstStmt, Flow, InOut, InOutDecl, MergeStmt, Node,
};
use crate::parser::expr::path_expression;
use crate::parser::interface::connect_body;
use crate::parser::lex::{
    identification, name, qualified_name, skip_until_brace_end, skip_statement_or_block,
    take_until_terminator, ws1, ws_and_comments,
};
use crate::parser::node_from_to;
use crate::parser::part::bind_;
use crate::parser::with_span;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::sequence::{delimited, preceded};
use nom::IResult;
use nom::Parser;

fn optional_multiplicity_brackets(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = opt(preceded(
        ws_and_comments,
        delimited(
            tag(&b"["[..]),
            nom::bytes::complete::take_until(&b"]"[..]),
            tag(&b"]"[..]),
        ),
    ))
    .parse(input)?;
    Ok((input, ()))
}

/// Ref declaration inside an action body.
///
/// The Systems Library often uses `ref action name: Type :>> ...;` in action definitions.
/// We parse the structured `ref ... name: Type` prefix and accept `= expr` bindings; any
/// remaining tokens up to the statement terminator are skipped.
fn action_ref_decl(input: Input<'_>) -> IResult<Input<'_>, Node<crate::ast::RefDecl>> {
    use crate::parser::expr::expression;

    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = opt(alt((
        preceded(tag(&b"public"[..]), ws1),
        preceded(tag(&b"private"[..]), ws1),
        preceded(tag(&b"protected"[..]), ws1),
    )))
    .parse(input)?;
    let (input, _) = tag(&b"ref"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = opt(preceded(tag(&b"action"[..]), ws1)).parse(input)?;
    let (input, (name_span, name_str)) = with_span(name).parse(input)?;
    // Optional multiplicity right after the name: `var[0..1]`
    let (input, _) = optional_multiplicity_brackets(input)?;

    // Standard library uses either `:` typing, `:>` specialization-like typing, or `:>>` feature redefinition.
    let (input, uses_shift) = preceded(
        ws_and_comments,
        alt((
            map(tag(&b":>>"[..]), |_| true),
            map(tag(&b":>"[..]), |_| false),
            map(tag(&b":"[..]), |_| false),
        )),
    )
    .parse(input)?;
    let (input, (type_ref_span, type_name)) = if uses_shift {
        (input, (crate::ast::Span::dummy(), String::new()))
    } else {
        preceded(ws_and_comments, with_span(qualified_name)).parse(input)?
    };

    let (input, _) = ws_and_comments(input)?;
    let (mut input, value) = opt(preceded(
        preceded(ws_and_comments, tag(&b"="[..])),
        preceded(ws_and_comments, expression),
    ))
    .parse(input)?;

    // Accept and skip shorthand redeclaration forms like `:>> Performance::self;`
    // (we don't model this binding yet, but we must consume it to avoid cascading errors).
    if !input.fragment().is_empty()
        && !input.fragment().starts_with(b";")
        && !input.fragment().starts_with(b"{")
    {
        let (next, _) = take_until_terminator(input, b";{")?;
        input = next;
    }

    let (input, body) = preceded(
        ws_and_comments,
        alt((
            map(tag(&b";"[..]), |_| crate::ast::RefBody::Semicolon),
            map(
                delimited(
                    tag(&b"{"[..]),
                    skip_until_brace_end,
                    preceded(ws_and_comments, tag(&b"}"[..])),
                ),
                |_| crate::ast::RefBody::Brace,
            ),
        )),
    )
    .parse(input)?;

    Ok((
        input,
        node_from_to(
            start,
            input,
            crate::ast::RefDecl {
                name: name_str,
                type_name,
                value,
                body,
                name_span: Some(name_span),
                type_ref_span: Some(type_ref_span),
            },
        ),
    ))
}

/// First/merge body: `;` or `{` ... `}`
fn first_merge_body(input: Input<'_>) -> IResult<Input<'_>, FirstMergeBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| FirstMergeBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| FirstMergeBody::Brace,
        ),
    ))
    .parse(input)
}

/// In/out decl: `in` name `:` type `;` or `out` name `:` type `;`
pub(crate) fn in_out_decl(input: Input<'_>) -> IResult<Input<'_>, Node<InOutDecl>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, direction) = alt((
        map(preceded(tag(&b"in"[..]), ws1), |_| InOut::In),
        map(preceded(tag(&b"out"[..]), ws1), |_| InOut::Out),
    ))
    .parse(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"attribute"[..]), ws1)).parse(input)?;
    let parsed = (|| {
        // Library shorthand: `in action body { ... }` (treat as name `body` typed as `action`)
        let (input, action_typed_name) = opt(preceded(tag(&b"action"[..]), ws1)).parse(input)?;
        let (input, param_name) = name(input)?;
        // In action usages, pin declarations may omit the type (e.g. `out videoStream;`)
        // to reference the corresponding typed parameter on the referenced action definition.
        // Action definitions generally include the type (e.g. `out videoStream : String;`),
        // but accepting the shorthand here prevents recovery errors in common models.
        let (input, type_name) = nom::combinator::opt(map(
            (
                preceded(ws_and_comments, tag(&b":"[..])),
                preceded(ws_and_comments, qualified_name),
            ),
            |(_, tn)| tn,
        ))
        .parse(input)?;
        let mut type_name = type_name.unwrap_or_default();
        if action_typed_name.is_some() && type_name.is_empty() {
            type_name = "action".to_string();
        }

        // Optional `default { ... }` initializer used in the standard library.
        let (input, _) = opt((
            preceded(ws_and_comments, tag(&b"default"[..])),
            ws1,
            delimited(
                tag(&b"{"[..]),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
        ))
        .parse(input)?;

        // Standard library sometimes uses braced pin bodies without a trailing semicolon.
        // Accept either `;` or `{ ... }` as a terminator.
        let (input, _) = preceded(
            ws_and_comments,
            alt((
                map(tag(&b";"[..]), |_| ()),
                map(
                    delimited(
                        tag(&b"{"[..]),
                        skip_until_brace_end,
                        preceded(ws_and_comments, tag(&b"}"[..])),
                    ),
                    |_| (),
                ),
            )),
        )
        .parse(input)?;
        Ok::<_, nom::Err<nom::error::Error<Input<'_>>>>((input, (param_name, type_name)))
    })();
    let (input, (param_name, type_name)) = match parsed {
        Ok(v) => v,
        Err(_) => {
            // Best-effort fallback: consume to `;` or start of a braced body.
            let (input, raw_text) = take_until_terminator(input, b";{")?;
            let raw_text = raw_text.trim().to_string();
            let name_guess = raw_text
                .split(|c: char| c.is_whitespace() || c == ':' || c == '[' || c == ',' || c == ';')
                .find(|s| !s.is_empty() && *s != ":>>")
                .unwrap_or("param")
                .to_string();
            // Accept `;` or a braced body after the unstructured prefix.
            let (input, _) = preceded(
                ws_and_comments,
                alt((
                    map(tag(&b";"[..]), |_| ()),
                    map(
                        delimited(
                            tag(&b"{"[..]),
                            skip_until_brace_end,
                            preceded(ws_and_comments, tag(&b"}"[..])),
                        ),
                        |_| (),
                    ),
                )),
            )
            .parse(input)?;
            // If we can't parse a structured `: Type`, keep the raw text as a best-effort
            // stand-in so downstream tools still have something to display.
            (input, (name_guess, raw_text))
        }
    };
    Ok((
        input,
        node_from_to(
            start,
            input,
            InOutDecl {
                direction,
                name: param_name,
                type_name,
            },
        ),
    ))
}

/// Action def body: `;` or `{` ActionDefBodyElement* `}`
fn action_def_body(input: Input<'_>) -> IResult<Input<'_>, ActionDefBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| ActionDefBody::Semicolon),
        action_def_body_brace,
    ))
    .parse(input)
}

fn action_def_body_brace(input: Input<'_>) -> IResult<Input<'_>, ActionDefBody> {
    let (mut input, _) = tag(&b"{"[..]).parse(input)?;
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
            return Ok((input, ActionDefBody::Brace { elements }));
        }
        match action_def_body_element(input) {
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
                // Action bodies in the standard library include many behavioral statements we don't
                // model yet (e.g. `assign`, `while`, `then private action ...`). Instead of aborting
                // the enclosing action definition (which cascades into top-level errors), skip one
                // statement/block and keep it as a non-diagnostic `Other(...)` element.
                let start_unknown = input;
                let (next, _) = skip_statement_or_block(input)?;
                if next.location_offset() == start_unknown.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                let frag = start_unknown.fragment();
                let take = frag.len().min(60);
                let preview = String::from_utf8_lossy(&frag[..take]).trim().to_string();
                elements.push(node_from_to(
                    start_unknown,
                    next,
                    crate::ast::ActionDefBodyElement::Other(preview),
                ));
                input = next;
            }
        }
    }
}

/// Element inside an action definition body.
///
/// SysML v2 ActionBodyItem includes both declarations and action behavior usages.
/// We support a pragmatic subset used by function-based behavior examples.
fn action_def_body_element(
    input: Input<'_>,
) -> IResult<Input<'_>, Node<crate::ast::ActionDefBodyElement>> {
    use crate::ast::ActionDefBodyElement;
    use crate::parser::part::perform_action_decl;
    use crate::parser::requirement::doc_comment;
    use crate::parser::state::state_usage;

    let (input, _) = ws_and_comments(input)?;
    let start = input;
    let (input, elem) = nom::branch::alt((
        map(in_out_decl, ActionDefBodyElement::InOutDecl),
        map(doc_comment, ActionDefBodyElement::Doc),
        map(action_ref_decl, ActionDefBodyElement::RefDecl),
        map(perform_action_decl, ActionDefBodyElement::Perform),
        map(bind_, ActionDefBodyElement::Bind),
        map(flow_, ActionDefBodyElement::Flow),
        map(first_stmt, ActionDefBodyElement::FirstStmt),
        map(merge_stmt, ActionDefBodyElement::MergeStmt),
        map(state_usage, ActionDefBodyElement::StateUsage),
        map(action_usage, |a| {
            ActionDefBodyElement::ActionUsage(Box::new(a))
        }),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

/// Action definition: `action` `def` Identification body
pub(crate) fn action_def(input: Input<'_>) -> IResult<Input<'_>, Node<ActionDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"action"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = action_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            ActionDef {
                identification,
                body,
            },
        ),
    ))
}

/// Flow: `flow` path `to` path body
fn flow_(input: Input<'_>) -> IResult<Input<'_>, Node<Flow>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"flow"[..]).parse(input)?;
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
            Flow {
                from: from_expr,
                to: to_expr,
                body,
            },
        ),
    ))
}

/// First stmt: `first` path `then` path body
fn first_stmt(input: Input<'_>) -> IResult<Input<'_>, Node<FirstStmt>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"first"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, first_expr) = path_expression(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"then"[..])).parse(input)?;
    let (input, then_expr) = preceded(ws_and_comments, path_expression).parse(input)?;
    let (input, body) = first_merge_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            FirstStmt {
                first: first_expr,
                then: then_expr,
                body,
            },
        ),
    ))
}

/// Merge stmt: `merge` path body
fn merge_stmt(input: Input<'_>) -> IResult<Input<'_>, Node<MergeStmt>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"merge"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, merge_expr) = path_expression(input)?;
    let (input, body) = first_merge_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            MergeStmt {
                merge: merge_expr,
                body,
            },
        ),
    ))
}

/// Action usage body: `;` or `{` ActionUsageBodyElement* `}`
fn action_usage_body(input: Input<'_>) -> IResult<Input<'_>, ActionUsageBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| ActionUsageBody::Semicolon),
        action_usage_body_brace,
    ))
    .parse(input)
}

fn action_usage_body_brace(input: Input<'_>) -> IResult<Input<'_>, ActionUsageBody> {
    let (mut input, _) = tag(&b"{"[..]).parse(input)?;
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
            return Ok((input, ActionUsageBody::Brace { elements }));
        }
        match action_usage_body_element(input) {
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
                let start_unknown = input;
                let (next, _) = skip_statement_or_block(input)?;
                if next.location_offset() == start_unknown.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                let frag = start_unknown.fragment();
                let take = frag.len().min(60);
                let preview = String::from_utf8_lossy(&frag[..take]).trim().to_string();
                elements.push(node_from_to(
                    start_unknown,
                    next,
                    ActionUsageBodyElement::Other(preview),
                ));
                input = next;
            }
        }
    }
}

/// Action usage body element: InOutDecl | Bind | Flow | FirstStmt | MergeStmt | ActionUsage
fn action_usage_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<ActionUsageBodyElement>> {
    use crate::parser::state::state_usage;

    let (input, _) = ws_and_comments(input)?;
    let start = input;
    let (input, elem) = alt((
        map(in_out_decl, ActionUsageBodyElement::InOutDecl),
        map(action_ref_decl, ActionUsageBodyElement::RefDecl),
        map(bind_, ActionUsageBodyElement::Bind),
        map(flow_, ActionUsageBodyElement::Flow),
        map(first_stmt, ActionUsageBodyElement::FirstStmt),
        map(merge_stmt, ActionUsageBodyElement::MergeStmt),
        map(state_usage, ActionUsageBodyElement::StateUsage),
        map(action_usage, |a| {
            ActionUsageBodyElement::ActionUsage(Box::new(a))
        }),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

/// Action usage: `action` name ( `:` type_name ( `accept` param `:` param_type )? | `accept` param_name `:` param_type )? body
pub(crate) fn action_usage(input: Input<'_>) -> IResult<Input<'_>, Node<ActionUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"action"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, (name_span, name_str)) = with_span(name).parse(input)?;
    let (input, type_accept) = nom::combinator::opt(nom::branch::alt((
        nom::combinator::map(
            (
                preceded(ws_and_comments, tag(&b":"[..])),
                preceded(
                    ws_and_comments,
                    nom::combinator::map(
                        (with_span(qualified_name), optional_multiplicity_brackets),
                        |((span, tn), _)| (span, tn),
                    ),
                ),
                nom::combinator::opt(preceded(
                    preceded(ws_and_comments, tag(&b"accept"[..])),
                    preceded(
                        ws1,
                        (
                            name,
                            preceded(ws_and_comments, tag(&b":"[..])),
                            preceded(ws_and_comments, qualified_name),
                        ),
                    ),
                )),
            ),
            |(_, (span, type_name), accept)| {
                (Some(span), type_name, accept.map(|(pn, _, tn)| (pn, tn)))
            },
        ),
        nom::combinator::map(
            preceded(
                preceded(ws_and_comments, tag(&b"accept"[..])),
                preceded(
                    ws1,
                    (
                        name,
                        preceded(ws_and_comments, tag(&b":"[..])),
                        preceded(ws_and_comments, name),
                    ),
                ),
            ),
            |(param_name, _, param_type)| {
                (None, param_type.clone(), Some((param_name, param_type)))
            },
        ),
    )))
    .parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (type_ref_span, type_name, accept) = type_accept.unwrap_or((None, String::new(), None));
    let (input, body) = action_usage_body(input)?;
    // Spec-wise, a braced body does not require a trailing semicolon. However, in practice some
    // sources write `... { ... };` as a statement terminator. We accept an optional `;` here to
    // avoid cascading recovery errors in action bodies.
    let (input, _) =
        nom::combinator::opt(preceded(ws_and_comments, tag(&b";"[..]))).parse(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            ActionUsage {
                name: name_str,
                type_name,
                accept,
                body,
                name_span: Some(name_span),
                type_ref_span,
            },
        ),
    ))
}
