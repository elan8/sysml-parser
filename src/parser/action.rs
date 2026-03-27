//! Action definition and action usage parsing (function-based behavior).

use crate::ast::{
    ActionDef, ActionDefBody, ActionUsage, ActionUsageBody, ActionUsageBodyElement,
    FirstMergeBody, FirstStmt, Flow, InOut, InOutDecl, MergeStmt, Node, ParseErrorNode,
};
use crate::parser::expr::path_expression;
use crate::parser::interface::connect_body;
use crate::parser::lex::{
    identification, looks_like_missing_semicolon, name, qualified_name, recover_body_element, skip_until_brace_end,
    take_until_terminator,
    starts_with_any_keyword, ws1, ws_and_comments, ACTION_BODY_STARTERS,
};
use crate::parser::node_from_to;
use crate::parser::part::bind_;
use crate::parser::with_span;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::sequence::preceded;
use nom::Parser;
use nom::IResult;

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
        let (input, param_name) = name(input)?;
        let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
        let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
        let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
        Ok::<_, nom::Err<nom::error::Error<Input<'_>>>>((input, (param_name, type_name)))
    })();
    let (input, (param_name, type_name)) = match parsed {
        Ok(v) => v,
        Err(_) => {
            let (input, raw_text) = take_until_terminator(input, b";")?;
            let raw_text = raw_text.trim().to_string();
            let name_guess = raw_text
                .split(|c: char| c.is_whitespace() || c == ':' || c == '[' || c == ',' || c == ';')
                .find(|s| !s.is_empty() && *s != ":>>")
                .unwrap_or("param")
                .to_string();
            let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
            (input, (name_guess, raw_text))
        }
    };
    Ok((
        input,
        node_from_to(start, input, InOutDecl {
            direction,
            name: param_name,
            type_name,
        }),
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
            Err(_) if starts_with_any_keyword(input.fragment(), ACTION_BODY_STARTERS) => {
                let (next, _) = recover_body_element(input, ACTION_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                let error_node = if looks_like_missing_semicolon(input, ACTION_BODY_STARTERS) {
                    ParseErrorNode {
                        message: "missing semicolon before next declaration".to_string(),
                        code: "missing_semicolon".to_string(),
                        expected: Some("';'".to_string()),
                        found: recovery_found_snippet(input),
                        suggestion: Some("Insert ';' before this declaration.".to_string()),
                    }
                } else {
                    ParseErrorNode {
                        message: "recovered action definition body element".to_string(),
                        code: "recovered_action_def_body_element".to_string(),
                        expected: Some("valid action definition body element".to_string()),
                        found: recovery_found_snippet(input),
                        suggestion: Some(
                            "Fix this action definition member and re-run parsing.".to_string(),
                        ),
                    }
                };
                elements.push(node_from_to(
                    input,
                    next,
                    crate::ast::ActionDefBodyElement::Error(Node::new(
                        crate::ast::Span::dummy(),
                        error_node,
                    )),
                ));
                input = next;
            }
            Err(_) => {
                return Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Tag,
                )));
            }
        }
    }
}

/// Element inside an action definition body: InOutDecl | Doc | Perform
fn action_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<crate::ast::ActionDefBodyElement>> {
    use crate::ast::ActionDefBodyElement;
    use crate::parser::part::perform_action_decl;
    use crate::parser::requirement::doc_comment;

    let (input, _) = ws_and_comments(input)?;
    let start = input;
    let (input, elem) = nom::branch::alt((
        map(in_out_decl, ActionDefBodyElement::InOutDecl),
        map(doc_comment, ActionDefBodyElement::Doc),
        map(perform_action_decl, ActionDefBodyElement::Perform),
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
        node_from_to(start, input, ActionDef {
            identification,
            body,
        }),
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
        node_from_to(start, input, Flow {
            from: from_expr,
            to: to_expr,
            body,
        }),
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
        node_from_to(start, input, FirstStmt {
            first: first_expr,
            then: then_expr,
            body,
        }),
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
        node_from_to(start, input, MergeStmt {
            merge: merge_expr,
            body,
        }),
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
            Err(_) if input.fragment().starts_with(b"doc") => {
                let (next, _) = recover_body_element(input, ACTION_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                input = next;
            }
            Err(_) if starts_with_any_keyword(input.fragment(), ACTION_BODY_STARTERS) => {
                let (next, _) = recover_body_element(input, ACTION_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                elements.push(node_from_to(
                    input,
                    next,
                    ActionUsageBodyElement::Error(Node::new(
                        crate::ast::Span::dummy(),
                        if looks_like_missing_semicolon(input, ACTION_BODY_STARTERS) {
                            ParseErrorNode {
                                message: "missing semicolon before next declaration".to_string(),
                                code: "missing_semicolon".to_string(),
                                expected: Some("';'".to_string()),
                                found: recovery_found_snippet(input),
                                suggestion: Some(
                                    "Insert ';' before this declaration.".to_string(),
                                ),
                            }
                        } else {
                            ParseErrorNode {
                                message: "recovered action usage body element".to_string(),
                                code: "recovered_action_usage_body_element".to_string(),
                                expected: Some("valid action usage body element".to_string()),
                                found: recovery_found_snippet(input),
                                suggestion: Some(
                                    "Fix this action usage member and re-run parsing.".to_string(),
                                ),
                            }
                        },
                    )),
                ));
                input = next;
            }
            Err(_) => {
                return Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Tag,
                )));
            }
        }
    }
}

/// Action usage body element: InOutDecl | Bind | Flow | FirstStmt | MergeStmt | ActionUsage
fn action_usage_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<ActionUsageBodyElement>> {
    let (input, _) = ws_and_comments(input)?;
    let start = input;
    let (input, elem) = alt((
        map(in_out_decl, ActionUsageBodyElement::InOutDecl),
        map(bind_, ActionUsageBodyElement::Bind),
        map(flow_, ActionUsageBodyElement::Flow),
        map(first_stmt, ActionUsageBodyElement::FirstStmt),
        map(merge_stmt, ActionUsageBodyElement::MergeStmt),
        map(action_usage, |a| ActionUsageBodyElement::ActionUsage(Box::new(a))),
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
                preceded(ws_and_comments, with_span(qualified_name)),
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
            |(_, (span, type_name), accept)| (Some(span), type_name, accept.map(|(pn, _, tn)| (pn, tn))),
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
            |(param_name, _, param_type)| (None, param_type.clone(), Some((param_name, param_type))),
        ),
    )))
    .parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (type_ref_span, type_name, accept) = type_accept.unwrap_or((None, String::new(), None));
    let (input, body) = action_usage_body(input)?;
    Ok((
        input,
        node_from_to(start, input, ActionUsage {
            name: name_str,
            type_name,
            accept,
            body,
            name_span: Some(name_span),
            type_ref_span,
        }),
    ))
}
