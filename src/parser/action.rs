//! Action definition and action usage parsing (function-based behavior).

use crate::ast::{
    ActionDef, ActionDefBody, ActionUsage, ActionUsageBody, ActionUsageBodyElement, FirstMergeBody,
    FirstStmt, Flow, InOut, InOutDecl, MergeStmt, Node,
};
use crate::parser::expr::path_expression;
use crate::parser::interface::connect_body;
use crate::parser::lex::{identification, name, qualified_name, skip_until_brace_end, ws1, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::part::bind_;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::preceded;
use nom::Parser;
use nom::IResult;

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
    let (input, param_name) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
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
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                preceded(
                    ws_and_comments,
                    many0(preceded(ws_and_comments, action_def_body_element)),
                ),
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |elements| ActionDefBody::Brace { elements },
        ),
    ))
    .parse(input)
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
    let (input, _) = tag(&b"action"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
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
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                preceded(
                    ws_and_comments,
                    many0(preceded(ws_and_comments, action_usage_body_element)),
                ),
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |elements| ActionUsageBody::Brace { elements },
        ),
    ))
    .parse(input)
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

/// Action usage: `action` name ( `:` type_name ( `accept` param `:` param_type )? | `accept` param_name `:` param_type ) body
pub(crate) fn action_usage(input: Input<'_>) -> IResult<Input<'_>, Node<ActionUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"action"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, (type_name, accept)) = nom::branch::alt((
        nom::combinator::map(
            (
                preceded(ws_and_comments, tag(&b":"[..])),
                preceded(ws_and_comments, name),
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
            |(_, type_name, accept)| (type_name, accept.map(|(pn, _, tn)| (pn, tn))),
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
            |(param_name, _, param_type)| (param_type.clone(), Some((param_name, param_type))),
        ),
    ))
    .parse(input)?;
    let (input, body) = action_usage_body(input)?;
    Ok((
        input,
        node_from_to(start, input, ActionUsage {
            name: name_str,
            type_name,
            accept,
            body,
        }),
    ))
}
