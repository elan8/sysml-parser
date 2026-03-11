//! Part definition and part usage parsing.

use crate::ast::{
    Bind, Connect, ConnectBody, InOut, InterfaceUsage, InterfaceUsageBodyElement, Node, PartDef,
    PartDefBody, PartDefBodyElement, PartUsage, PartUsageBody, PartUsageBodyElement, Perform,
    PerformBody, PerformBodyElement, RefBody,
};
use crate::parser::attribute::{attribute_def, attribute_usage};
use crate::parser::expr::{expression, path_expression};
use crate::parser::interface::connect_body;
use crate::parser::lex::{identification, name, qualified_name, skip_until_brace_end, ws1, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::port::port_usage;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::combinator::{map, opt, value};
use nom::multi::many0;
use nom::sequence::{preceded, tuple};
use nom::IResult;

/// Part def body: ';' or '{' PartDefBodyElement* '}'
pub(crate) fn part_def_body(input: Input<'_>) -> IResult<Input<'_>, PartDefBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(b";"), |_| PartDefBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(b"{"),
                preceded(
                    ws_and_comments,
                    many0(preceded(ws_and_comments, part_def_body_element)),
                ),
                preceded(ws_and_comments, tag(b"}")),
            ),
            |elements| PartDefBody::Brace { elements },
        ),
    ))(input)
}

fn part_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<PartDefBodyElement>> {
    let (input, _) = ws_and_comments(input)?;
    let start = input;
    let (input, elem) = alt((
        map(port_usage, PartDefBodyElement::PortUsage),
        map(attribute_def, PartDefBodyElement::AttributeDef),
    ))(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

/// Part definition: 'part' 'def' Identification ( ':>' qualified_name )? body
pub(crate) fn part_def(input: Input<'_>) -> IResult<Input<'_>, Node<PartDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(b"part")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(b"def")(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, specializes) = opt(preceded(
        preceded(ws_and_comments, tag(b":>")),
        preceded(ws_and_comments, qualified_name),
    ))(input)?;
    let (input, body) = part_def_body(input)?;
    Ok((
        input,
        node_from_to(start, input, PartDef {
            identification,
            specializes,
            body,
        }),
    ))
}

/// Multiplicity: '[' ... ']' as string
fn multiplicity(input: Input<'_>) -> IResult<Input<'_>, String> {
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(b"[")(input)?;
    let (input, content) = take_until(&b"]"[..])(input)?;
    let (input, _) = tag(b"]")(input)?;
    let s = format!("[{}]", String::from_utf8_lossy(content.fragment()).trim());
    Ok((input, s))
}

/// Part usage: 'part' name ':' type_name multiplicity? 'ordered'? ( 'subsets' name '=' value )? body
pub(crate) fn part_usage(input: Input<'_>) -> IResult<Input<'_>, Node<PartUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(b"part")(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = alt((
        preceded(
            preceded(ws_and_comments, tag(b":>>")),
            preceded(ws_and_comments, name),
        ),
        preceded(ws_and_comments, name),
    ))(input)?;
    let (input, type_name) = opt(preceded(
        preceded(ws_and_comments, tag(b":")),
        preceded(ws_and_comments, qualified_name),
    ))(input)?;
    let (input, multiplicity_opt) = opt(multiplicity)(input)?;
    let (input, ordered) = opt(preceded(ws_and_comments, tag(b"ordered")))(input)?;
    let (input, subsets) = opt(alt((
        preceded(
            preceded(ws_and_comments, tag(b"subsets")),
            preceded(ws1, tuple((
                name,
                opt(preceded(
                    preceded(ws_and_comments, tag(b"=")),
                    preceded(ws_and_comments, expression),
                )),
            ))),
        ),
        preceded(
            preceded(ws_and_comments, tag(b":>")),
            preceded(ws_and_comments, tuple((
                name,
                opt(preceded(
                    preceded(ws_and_comments, tag(b"=")),
                    preceded(ws_and_comments, expression),
                )),
            ))),
        ),
    )))(input)?;
    let (input, body) = part_usage_body(input)?;
    Ok((
        input,
        node_from_to(start, input, PartUsage {
            name: name_str,
            type_name: type_name.unwrap_or_else(String::new),
            multiplicity: multiplicity_opt,
            ordered: ordered.is_some(),
            subsets: subsets.map(|(n, v)| (n, v)),
            body,
        }),
    ))
}

/// Part usage body: ';' or '{' PartUsageBodyElement* '}'
fn part_usage_body(input: Input<'_>) -> IResult<Input<'_>, PartUsageBody> {
    let (input, _) = ws_and_comments(input)?;
    let frag = input.fragment();
    log::debug!(
        "part_usage_body: first 40 bytes: {:?}",
        frag.get(..40.min(frag.len())).unwrap_or(frag),
    );
    let result = alt((
        map(tag(b";"), |_| PartUsageBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(b"{"),
                preceded(
                    ws_and_comments,
                    many0(preceded(ws_and_comments, part_usage_body_element)),
                ),
                preceded(ws_and_comments, tag(b"}")),
            ),
            |elements| {
                log::debug!("part_usage_body: brace ok, {} elements", elements.len());
                PartUsageBody::Brace { elements }
            },
        ),
    ))(input);
    if let Err(_) = &result {
        log::debug!(
            "part_usage_body: failed at: {:?}",
            String::from_utf8_lossy(frag.get(..60.min(frag.len())).unwrap_or(frag)),
        );
    }
    result
}

/// Action path for perform: name ( '.' name )* -> joined with ".".
fn perform_action_path(input: Input<'_>) -> IResult<Input<'_>, String> {
    let (input, first) = name(input)?;
    let (input, rest) = many0(preceded(
        preceded(ws_and_comments, tag(b".")),
        preceded(ws_and_comments, name),
    ))(input)?;
    let action_name = std::iter::once(first)
        .chain(rest)
        .collect::<Vec<_>>()
        .join(".");
    Ok((input, action_name))
}

/// Perform body element: `in` name `=` expr `;` or `out` name `=` expr `;`.
fn perform_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<PerformBodyElement>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, direction) = alt((
        value(InOut::In, tag(b"in")),
        value(InOut::Out, tag(b"out")),
    ))(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(b"="))(input)?;
    let (input, value_expr) = preceded(ws_and_comments, path_expression)(input)?;
    let (input, _) = preceded(ws_and_comments, tag(b";"))(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            PerformBodyElement {
                direction,
                name: name_str,
                value: value_expr,
            },
        ),
    ))
}

/// Perform body: `{` (doc/comments and perform_body_element* ) `}`.
fn perform_body(input: Input<'_>) -> IResult<Input<'_>, PerformBody> {
    let (input, _) = ws_and_comments(input)?;
    let (input, elements) = nom::sequence::delimited(
        tag(b"{"),
        preceded(
            ws_and_comments,
            many0(preceded(ws_and_comments, perform_body_element)),
        ),
        preceded(ws_and_comments, tag(b"}")),
    )(input)?;
    Ok((input, PerformBody::Brace { elements }))
}

/// Perform usage: `perform` action_path body.
fn perform_usage(input: Input<'_>) -> IResult<Input<'_>, Node<Perform>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(b"perform")(input)?;
    let (input, _) = ws1(input)?;
    let (input, action_name) = perform_action_path(input)?;
    let (input, body) = perform_body(input)?;
    Ok((
        input,
        node_from_to(start, input, Perform { action_name, body }),
    ))
}

/// Bind: `bind` path `=` path (`;` or `{ }`)
pub(crate) fn bind_(input: Input<'_>) -> IResult<Input<'_>, Node<Bind>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(b"bind")(input)?;
    let (input, _) = ws1(input)?;
    let (input, left) = path_expression(input)?;
    let (input, _) = preceded(ws_and_comments, tag(b"="))(input)?;
    let (input, right) = preceded(ws_and_comments, path_expression)(input)?;
    let (input, body) = alt((
        map(preceded(ws_and_comments, tag(b";")), |_| Some(ConnectBody::Semicolon)),
        map(
            nom::sequence::delimited(
                preceded(ws_and_comments, tag(b"{")),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(b"}")),
            ),
            |_| Some(ConnectBody::Brace),
        ),
    ))(input)?;
    Ok((input, node_from_to(start, input, Bind { left, right, body })))
}

/// Connect (part usage level): `connect` path `to` path body
fn connect_(input: Input<'_>) -> IResult<Input<'_>, Node<Connect>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(b"connect")(input)?;
    let (input, _) = ws1(input)?;
    let (input, from_expr) = path_expression(input)?;
    let (input, _) = preceded(ws_and_comments, tag(b"to"))(input)?;
    let (input, to_expr) = preceded(ws_and_comments, path_expression)(input)?;
    let (input, body) = connect_body(input)?;
    Ok((
        input,
        node_from_to(start, input, Connect {
            from: from_expr,
            to: to_expr,
            body,
        }),
    ))
}

/// Interface usage body elements: `ref` `:>>` name `=` value body (RefRedef)
fn interface_usage_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<InterfaceUsageBodyElement>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(b"ref")(input)?;
    let (input, _) = preceded(ws_and_comments, tag(b":>>"))(input)?;
    let (input, ref_name) = preceded(ws_and_comments, name)(input)?;
    let (input, _) = preceded(ws_and_comments, tag(b"="))(input)?;
    let (input, value) = preceded(ws_and_comments, expression)(input)?;
    let (input, body) = ref_body_parse(input)?;
    Ok((
        input,
        node_from_to(start, input, InterfaceUsageBodyElement::RefRedef {
            name: ref_name,
            value,
            body,
        }),
    ))
}

fn ref_body_parse(input: Input<'_>) -> IResult<Input<'_>, RefBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(b";"), |_| RefBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(b"{"),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(b"}")),
            ),
            |_| RefBody::Brace,
        ),
    ))(input)
}

/// Connect body for interface usage (TypedConnect): `;` or `{` body_elements* `}`
fn connect_body_with_elements(input: Input<'_>) -> IResult<Input<'_>, (ConnectBody, Vec<Node<InterfaceUsageBodyElement>>)> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(b";"), |_| (ConnectBody::Semicolon, vec![])),
        map(
            nom::sequence::delimited(
                tag(b"{"),
                preceded(
                    ws_and_comments,
                    many0(preceded(ws_and_comments, interface_usage_body_element)),
                ),
                preceded(ws_and_comments, tag(b"}")),
            ),
            |elements| (ConnectBody::Brace, elements),
        ),
    ))(input)
}

/// Interface usage: `interface` ( `:Type` )? `connect` path `to` path body  OR  `interface` path `to` path body?
fn interface_usage(input: Input<'_>) -> IResult<Input<'_>, Node<InterfaceUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(b"interface")(input)?;
    let (input, _) = ws1(input)?;
    let (input, interface_type) = opt(preceded(
        tag(b":"),
        preceded(ws_and_comments, qualified_name),
    ))(input)?;
    let (input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b"connect") {
        let (input, _) = tag(b"connect")(input)?;
        let (input, _) = ws1(input)?;
        let (input, from_expr) = path_expression(input)?;
        let (input, _) = preceded(ws_and_comments, tag(b"to"))(input)?;
        let (input, to_expr) = preceded(ws_and_comments, path_expression)(input)?;
        let (input, (body, body_elements)) = connect_body_with_elements(input)?;
        Ok((
            input,
            node_from_to(start, input, InterfaceUsage::TypedConnect {
                interface_type,
                from: from_expr,
                to: to_expr,
                body,
                body_elements,
            }),
        ))
    } else {
        let (input, from_expr) = path_expression(input)?;
        let (input, _) = preceded(ws_and_comments, tag(b"to"))(input)?;
        let (input, to_expr) = preceded(ws_and_comments, path_expression)(input)?;
        let (input, _) = opt(connect_body)(input)?;
        Ok((
            input,
            node_from_to(start, input, InterfaceUsage::Connection {
                from: from_expr,
                to: to_expr,
                body_elements: vec![],
            }),
        ))
    }
}

fn part_usage_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<PartUsageBodyElement>> {
    let (input, _) = ws_and_comments(input)?;
    let start = input;
    let frag = start.fragment();
    let first_30 = frag.get(..30.min(frag.len())).unwrap_or(frag);
    log::debug!(
        "part_usage_body_element: first 30 bytes: {:?} (str: {:?})",
        first_30,
        String::from_utf8_lossy(first_30),
    );
    let (input, elem) = alt((
        map(perform_usage, PartUsageBodyElement::Perform),
        map(attribute_usage, PartUsageBodyElement::AttributeUsage),
        map(part_usage, |p| PartUsageBodyElement::PartUsage(Box::new(p))),
        map(port_usage, PartUsageBodyElement::PortUsage),
        map(bind_, PartUsageBodyElement::Bind),
        map(interface_usage, PartUsageBodyElement::InterfaceUsage),
        map(connect_, PartUsageBodyElement::Connect),
    ))(input)?;
    Ok((input, node_from_to(start, input, elem)))
}
