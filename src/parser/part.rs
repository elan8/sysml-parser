//! Part definition and part usage parsing.

use crate::ast::{
    Allocate, Bind, Connect, ConnectBody, ExhibitState, InOut, InterfaceUsage, InterfaceUsageBodyElement,
    Node, PartDef, PartDefBody, PartDefBodyElement, PartUsage, PartUsageBody, PartUsageBodyElement,
    Perform, PerformBody, PerformBodyElement, PerformInOutBinding, RefBody,
};
use crate::parser::attribute::{attribute_def, attribute_usage};
use crate::parser::expr::{expression, path_expression};
use crate::parser::interface::connect_body;
use crate::parser::lex::{identification, name, qualified_name, skip_until_brace_end, ws1, ws_and_comments};
use crate::parser::{node_from_to, span_from_to};
use crate::parser::port::port_usage;
use crate::parser::with_span;
use crate::parser::requirement::doc_comment;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::combinator::{map, opt, value};
use nom::multi::many0;
use nom::sequence::preceded;
use nom::Parser;
use nom::IResult;

/// Part def body: ';' or '{' PartDefBodyElement* '}'
pub(crate) fn part_def_body(input: Input<'_>) -> IResult<Input<'_>, PartDefBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| PartDefBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                preceded(
                    ws_and_comments,
                    many0(preceded(ws_and_comments, part_def_body_element)),
                ),
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |elements| PartDefBody::Brace { elements },
        ),
    ))
    .parse(input)
}

/// Exhibit state usage: `exhibit state` name `:` type `;`
fn exhibit_state(input: Input<'_>) -> IResult<Input<'_>, Node<ExhibitState>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"exhibit"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"state"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            ExhibitState {
                name: name_str,
                type_name,
            },
        ),
    ))
}

fn part_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<PartDefBodyElement>> {
    let (input, _) = ws_and_comments(input)?;
    let start = input;
    let (input, elem) = alt((
        map(doc_comment, PartDefBodyElement::Doc),
        map(exhibit_state, PartDefBodyElement::ExhibitState),
        map(perform_action_decl, PartDefBodyElement::Perform),
        map(perform_usage, PartDefBodyElement::Perform),
        map(allocate_, PartDefBodyElement::Allocate),
        map(connect_, PartDefBodyElement::Connect),
        map(part_usage, |p| PartDefBodyElement::PartUsage(Box::new(p))),
        map(port_usage, PartDefBodyElement::PortUsage),
        map(attribute_usage, PartDefBodyElement::AttributeUsage),
        map(attribute_def, PartDefBodyElement::AttributeDef),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

/// Part definition: 'part' 'def' Identification ( ':>' qualified_name )? body
pub(crate) fn part_def(input: Input<'_>) -> IResult<Input<'_>, Node<PartDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"part"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let before_specializes = input;
    let (input, opt_specializes) = opt((
        preceded(ws_and_comments, tag(&b":>"[..])),
        preceded(ws_and_comments, qualified_name),
    ))
    .parse(input)?;
    let (specializes, specializes_span) = match opt_specializes {
        Some((_, type_name)) => (Some(type_name), Some(span_from_to(before_specializes, input))),
        None => (None, None),
    };
    let (input, body) = part_def_body(input)?;
    Ok((
        input,
        node_from_to(start, input, PartDef {
            identification,
            specializes,
            specializes_span,
            body,
        }),
    ))
}

/// Multiplicity: '[' ... ']' as string
fn multiplicity(input: Input<'_>) -> IResult<Input<'_>, String> {
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"["[..]).parse(input)?;
    let (input, content) = take_until(&b"]"[..]).parse(input)?;
    let (input, _) = tag(&b"]"[..]).parse(input)?;
    let s = format!("[{}]", String::from_utf8_lossy(content.fragment()).trim());
    Ok((input, s))
}

/// Value part for usages: `= expr` | `:= expr` | `default = expr` | `default := expr`.
fn usage_value_part(input: Input<'_>) -> IResult<Input<'_>, Node<crate::ast::Expression>> {
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = alt((
        preceded(tag(&b"="[..]), ws_and_comments),
        preceded(tag(&b":="[..]), ws_and_comments),
        preceded(
            preceded(tag(&b"default"[..]), ws1),
            preceded(alt((tag(&b"="[..]), tag(&b":="[..]))), ws_and_comments),
        ),
    ))
    .parse(input)?;
    expression(input)
}

/// Part usage: 'part' (':>>')? name ':' type_name multiplicity? 'ordered'? ( 'subsets' name '=' value )? ( 'redefines'|':>>' qualified_name )? value? body
pub(crate) fn part_usage(input: Input<'_>) -> IResult<Input<'_>, Node<PartUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"part"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = opt(preceded(ws_and_comments, tag(&b":>>"[..]))).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, (name_span, name_str)) = with_span(name).parse(input)?;
    let (input, type_result) = opt(preceded(
        preceded(ws_and_comments, tag(&b":"[..])),
        preceded(ws_and_comments, with_span(qualified_name)),
    ))
    .parse(input)?;
    let (type_ref_span, type_name) = type_result
        .map(|(s, t)| (Some(s), t))
        .unwrap_or((None, String::new()));
    let (input, multiplicity_opt) = opt(multiplicity).parse(input)?;
    let (input, ordered) = opt(preceded(ws_and_comments, tag(&b"ordered"[..]))).parse(input)?;
    let mut subsets_parser = opt(preceded(
        alt((
            preceded(ws_and_comments, tag(&b":>"[..])),
            preceded(ws_and_comments, tag(&b"subsets"[..])),
        )),
        preceded(
            ws_and_comments,
            (
                name,
                opt(preceded(
                    preceded(ws_and_comments, tag(&b"="[..])),
                    preceded(ws_and_comments, expression),
                )),
            ),
        ),
    ));
    let (input, subsets) = subsets_parser.parse(input)?;
    let (input, redefines) = opt(alt((
        preceded(
            preceded(ws_and_comments, tag(&b"redefines"[..])),
            preceded(ws1, qualified_name),
        ),
        preceded(
            preceded(ws_and_comments, tag(&b":>>"[..])),
            preceded(ws_and_comments, qualified_name),
        ),
    )))
    .parse(input)?;
    let (input, value) = opt(preceded(ws_and_comments, usage_value_part)).parse(input)?;
    let (input, body) = part_usage_body(input)?;
    Ok((
        input,
        node_from_to(start, input, PartUsage {
            name: name_str,
            type_name,
            multiplicity: multiplicity_opt,
            ordered: ordered.is_some(),
            subsets: subsets.map(|(n, v)| (n, v)),
            redefines,
            value,
            body,
            name_span: Some(name_span),
            type_ref_span,
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
        map(tag(&b";"[..]), |_| PartUsageBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                preceded(
                    ws_and_comments,
                    many0(preceded(ws_and_comments, part_usage_body_element)),
                ),
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |elements| {
                log::debug!("part_usage_body: brace ok, {} elements", elements.len());
                PartUsageBody::Brace { elements }
            },
        ),
    ))
    .parse(input);
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
    let mut rest_parser = many0(preceded(
        preceded(ws_and_comments, tag(&b"."[..])),
        preceded(ws_and_comments, name),
    ));
    let (input, rest) = rest_parser.parse(input)?;
    let action_name = std::iter::once(first)
        .chain(rest)
        .collect::<Vec<_>>()
        .join(".");
    Ok((input, action_name))
}

/// In/out binding inside a perform body: `in` name `=` expr `;` or `out` name `=` expr `;`.
fn perform_in_out_binding(input: Input<'_>) -> IResult<Input<'_>, Node<PerformInOutBinding>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, direction) = alt((
        value(InOut::In, tag(&b"in"[..])),
        value(InOut::Out, tag(&b"out"[..])),
    ))
    .parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"="[..])).parse(input)?;
    let (input, value_expr) = preceded(ws_and_comments, path_expression).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            PerformInOutBinding {
                direction,
                name: name_str,
                value: value_expr,
            },
        ),
    ))
}

/// Perform body element: doc comment or in/out binding.
fn perform_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<PerformBodyElement>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, elem) = alt((
        map(doc_comment, PerformBodyElement::Doc),
        map(perform_in_out_binding, PerformBodyElement::InOut),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

/// Perform body: `{` PerformBodyElement* `}`.
fn perform_body(input: Input<'_>) -> IResult<Input<'_>, PerformBody> {
    let (input, _) = ws_and_comments(input)?;
    let (input, elements) = nom::sequence::delimited(
        tag(&b"{"[..]),
        preceded(
            ws_and_comments,
            many0(preceded(ws_and_comments, perform_body_element)),
        ),
        preceded(ws_and_comments, tag(&b"}"[..])),
    )
    .parse(input)?;
    Ok((input, PerformBody::Brace { elements }))
}

/// Perform usage: `perform` action_path body (with optional `{ }` body).
fn perform_usage(input: Input<'_>) -> IResult<Input<'_>, Node<Perform>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"perform"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, action_name) = perform_action_path(input)?;
    let (input, body) = perform_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            Perform {
                action_name,
                type_name: None,
                body,
            },
        ),
    ))
}

/// Perform action declaration: `perform action` name (`:` type_name)? (`;` or body).
pub(crate) fn perform_action_decl(input: Input<'_>) -> IResult<Input<'_>, Node<Perform>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"perform"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"action"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, action_name) = name(input)?;
    let (input, type_name) = opt(preceded(
        preceded(ws_and_comments, tag(&b":"[..])),
        preceded(ws_and_comments, qualified_name),
    ))
    .parse(input)?;
    let (input, body_cb) = connect_body(input)?;
    let body = match body_cb {
        ConnectBody::Semicolon => PerformBody::Semicolon,
        ConnectBody::Brace => PerformBody::Brace {
            elements: vec![],
        },
    };
    Ok((
        input,
        node_from_to(
            start,
            input,
            Perform {
                action_name,
                type_name,
                body,
            },
        ),
    ))
}

/// Allocate: `allocate` source `to` target body.
fn allocate_(input: Input<'_>) -> IResult<Input<'_>, Node<Allocate>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"allocate"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, source) = path_expression(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"to"[..])).parse(input)?;
    let (input, target) = preceded(ws_and_comments, path_expression).parse(input)?;
    let (input, body) = connect_body(input)?;
    Ok((
        input,
        node_from_to(start, input, Allocate { source, target, body }),
    ))
}

/// Bind: `bind` path `=` path (`;` or `{ }`)
pub(crate) fn bind_(input: Input<'_>) -> IResult<Input<'_>, Node<Bind>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"bind"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, left) = path_expression(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"="[..])).parse(input)?;
    let (input, right) = preceded(ws_and_comments, path_expression).parse(input)?;
    let mut body_parser = alt((
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| Some(ConnectBody::Semicolon)),
        map(
            nom::sequence::delimited(
                preceded(ws_and_comments, tag(&b"{"[..])),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| Some(ConnectBody::Brace),
        ),
    ));
    let (input, body) = body_parser.parse(input)?;
    Ok((input, node_from_to(start, input, Bind { left, right, body })))
}

/// Connect (part usage level): `connect` path `to` path body
fn connect_(input: Input<'_>) -> IResult<Input<'_>, Node<Connect>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"connect"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, from_expr) = path_expression(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"to"[..])).parse(input)?;
    let (input, to_expr) = preceded(ws_and_comments, path_expression).parse(input)?;
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
    let (input, _) = tag(&b"ref"[..]).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":>>"[..])).parse(input)?;
    let (input, ref_name) = preceded(ws_and_comments, name).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"="[..])).parse(input)?;
    let (input, value) = preceded(ws_and_comments, expression).parse(input)?;
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
        map(tag(&b";"[..]), |_| RefBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| RefBody::Brace,
        ),
    ))
    .parse(input)
}

/// Connect body for interface usage (TypedConnect): `;` or `{` body_elements* `}`
fn connect_body_with_elements(input: Input<'_>) -> IResult<Input<'_>, (ConnectBody, Vec<Node<InterfaceUsageBodyElement>>)> {
    let (input, _) = ws_and_comments(input)?;
    let mut parser = alt((
        map(tag(&b";"[..]), |_| (ConnectBody::Semicolon, vec![])),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                preceded(
                    ws_and_comments,
                    many0(preceded(ws_and_comments, interface_usage_body_element)),
                ),
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |elements| (ConnectBody::Brace, elements),
        ),
    ));
    parser.parse(input)
}

/// Interface usage: `interface` ( `:Type` )? `connect` path `to` path body  OR  `interface` path `to` path body?
fn interface_usage(input: Input<'_>) -> IResult<Input<'_>, Node<InterfaceUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"interface"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, interface_type) = opt(preceded(
        tag(&b":"[..]),
        preceded(ws_and_comments, qualified_name),
    ))
    .parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b"connect") {
        let (input, _) = tag(&b"connect"[..]).parse(input)?;
        let (input, _) = ws1(input)?;
        let (input, from_expr) = path_expression(input)?;
        let (input, _) = preceded(ws_and_comments, tag(&b"to"[..])).parse(input)?;
        let (input, to_expr) = preceded(ws_and_comments, path_expression).parse(input)?;
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
        let (input, _) = preceded(ws_and_comments, tag(&b"to"[..])).parse(input)?;
        let (input, to_expr) = preceded(ws_and_comments, path_expression).parse(input)?;
        let (input, _) = opt(connect_body).parse(input)?;
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
        map(doc_comment, PartUsageBodyElement::Doc),
        map(perform_action_decl, PartUsageBodyElement::Perform),
        map(perform_usage, PartUsageBodyElement::Perform),
        map(allocate_, PartUsageBodyElement::Allocate),
        map(attribute_usage, PartUsageBodyElement::AttributeUsage),
        map(part_usage, |p| PartUsageBodyElement::PartUsage(Box::new(p))),
        map(port_usage, PartUsageBodyElement::PortUsage),
        map(bind_, PartUsageBodyElement::Bind),
        map(interface_usage, PartUsageBodyElement::InterfaceUsage),
        map(connect_, PartUsageBodyElement::Connect),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}
