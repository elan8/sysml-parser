//! Part definition and part usage parsing.

use crate::ast::{
    Allocate, Bind, Connect, ConnectBody, DefinitionPrefix, ExhibitState, InOut, InterfaceUsage,
    InterfaceUsageBodyElement, Node, PartDef, PartDefBody, PartDefBodyElement, PartUsage,
    PartUsageBody, PartUsageBodyElement, ParseErrorNode, Perform, PerformBody, PerformBodyElement,
    PerformInOutBinding, RefBody, RefDecl,
};
use crate::parser::attribute::{attribute_def, attribute_usage, attribute_usage_shorthand};
use crate::parser::expr::{expression, path_expression};
use crate::parser::interface::connect_body;
use crate::parser::lex::{
    identification, name, qualified_name, recover_body_element, skip_until_brace_end,
    starts_with_any_keyword, ws1, ws_and_comments, PART_BODY_STARTERS,
};
use nom::sequence::delimited;
use crate::parser::{node_from_to, span_from_to};
use crate::parser::port::port_usage;
use crate::parser::with_span;
use crate::parser::metadata_annotation::metadata_annotation;
use crate::parser::requirement::doc_comment;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::combinator::{map, opt, value};
use nom::multi::many0;
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
    if snippet.is_empty() {
        None
    } else {
        Some(snippet)
    }
}

/// Result of parsing either a part definition or part usage (used for package body to avoid part_def consuming "part" before part_usage can run).
#[derive(Debug)]
pub(crate) enum PartDefOrUsage {
    Def(Node<PartDef>),
    Usage(Node<PartUsage>),
}

/// Part def body: ';' or '{' PartDefBodyElement* '}'
pub(crate) fn part_def_body(input: Input<'_>) -> IResult<Input<'_>, PartDefBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((map(tag(&b";"[..]), |_| PartDefBody::Semicolon), part_def_body_brace)).parse(input)
}

fn part_def_body_brace(input: Input<'_>) -> IResult<Input<'_>, PartDefBody> {
    let (mut input, _) = tag(&b"{"[..]).parse(input)?;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().starts_with(b"}") {
            let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
            return Ok((input, PartDefBody::Brace { elements }));
        }
        match part_def_body_element(input) {
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
            Err(_) if starts_with_any_keyword(input.fragment(), PART_BODY_STARTERS) => {
                let (next, _) = recover_body_element(input, PART_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                elements.push(node_from_to(
                    input,
                    next,
                    PartDefBodyElement::Error(Node::new(
                        crate::ast::Span::dummy(),
                        ParseErrorNode {
                            message: "recovered part definition body element".to_string(),
                            code: "recovered_part_def_body_element".to_string(),
                            expected: Some("valid part definition body element".to_string()),
                            found: recovery_found_snippet(input),
                            suggestion: None,
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
        map(attribute_usage_shorthand, PartDefBodyElement::AttributeUsage),
        map(attribute_def, PartDefBodyElement::AttributeDef),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

/// Part definition: ( 'abstract' | 'variation' )? 'part' 'def' Identification ( ':>' qualified_name )? body
pub(crate) fn part_def(input: Input<'_>) -> IResult<Input<'_>, Node<PartDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, definition_prefix) = opt(alt((
        map(preceded(tag(&b"abstract"[..]), ws1), |_| DefinitionPrefix::Abstract),
        map(preceded(tag(&b"variation"[..]), ws1), |_| DefinitionPrefix::Variation),
    )))
    .parse(input)?;
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
            definition_prefix,
            identification,
            specializes,
            specializes_span,
            body,
        }),
    ))
}

/// Parses "part" then dispatches: if "def" follows, part_def; else part_usage. Used in package body so "part name" is not consumed by part_def.
pub(crate) fn part_def_or_usage(input: Input<'_>) -> IResult<Input<'_>, PartDefOrUsage> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, definition_prefix) = opt(alt((
        map(preceded(tag(&b"abstract"[..]), ws1), |_| DefinitionPrefix::Abstract),
        map(preceded(tag(&b"variation"[..]), ws1), |_| DefinitionPrefix::Variation),
    )))
    .parse(input)?;
    let (input, _) = tag(&b"part"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    if let Ok((input, _)) = tag::<_, _, nom::error::Error<Input>>(&b"def"[..]).parse(input) {
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
        return Ok((
            input,
            PartDefOrUsage::Def(node_from_to(
                start,
                input,
                PartDef {
                    definition_prefix,
                    identification,
                    specializes,
                    specializes_span,
                    body,
                },
            )),
        ));
    }
    if let Ok((input, usage)) = part_usage_redefines_only(start, input) {
        return Ok((input, PartDefOrUsage::Usage(usage)));
    }
    let (input, usage) = part_usage_named(start, input)?;
    Ok((input, PartDefOrUsage::Usage(usage)))
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

/// Part usage redefines-only: ':>>' qualified_name multiplicity? ordered? value? body (no name/type).
fn part_usage_redefines_only<'a>(
    start: Input<'a>,
    input: Input<'a>,
) -> IResult<Input<'a>, Node<PartUsage>> {
    let (input, _) = preceded(ws_and_comments, tag(&b":>>"[..])).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, redefines_qname) = qualified_name.parse(input)?;
    let (input, multiplicity_opt) = opt(multiplicity).parse(input)?;
    let (input, ordered) = opt(preceded(ws_and_comments, tag(&b"ordered"[..]))).parse(input)?;
    let (input, value) = opt(preceded(ws_and_comments, usage_value_part)).parse(input)?;
    let (input, body) = part_usage_body(input)?;
    Ok((
        input,
        node_from_to(start, input, PartUsage {
            name: String::new(),
            type_name: String::new(),
            multiplicity: multiplicity_opt,
            ordered: ordered.is_some(),
            subsets: None,
            redefines: Some(redefines_qname),
            value,
            body,
            name_span: None,
            type_ref_span: None,
        }),
    ))
}

/// Part usage with name (and optional type, redefines, etc.): (':>>')? name ':' type_name? ...
fn part_usage_named<'a>(
    start: Input<'a>,
    input: Input<'a>,
) -> IResult<Input<'a>, Node<PartUsage>> {
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
    let (input, subsets) = opt(preceded(
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
    ))
    .parse(input)?;
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

/// Part usage: 'part' ( ':>>' qualified_name | (':>>')? name ':' type_name? ... ) multiplicity? ... body
pub(crate) fn part_usage(input: Input<'_>) -> IResult<Input<'_>, Node<PartUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"part"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    if let Ok((input, usage)) = part_usage_redefines_only(start, input) {
        return Ok((input, usage));
    }
    let (input, usage) = part_usage_named(start, input)?;
    Ok((input, usage))
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
        part_usage_body_brace,
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

fn part_usage_body_brace(input: Input<'_>) -> IResult<Input<'_>, PartUsageBody> {
    let (mut input, _) = tag(&b"{"[..]).parse(input)?;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().starts_with(b"}") {
            let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
            log::debug!("part_usage_body: brace ok, {} elements", elements.len());
            return Ok((input, PartUsageBody::Brace { elements }));
        }
        match part_usage_body_element(input) {
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
            Err(_) if starts_with_any_keyword(input.fragment(), PART_BODY_STARTERS) => {
                let (next, _) = recover_body_element(input, PART_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                elements.push(node_from_to(
                    input,
                    next,
                    PartUsageBodyElement::Error(Node::new(
                        crate::ast::Span::dummy(),
                        ParseErrorNode {
                            message: "recovered part usage body element".to_string(),
                            code: "recovered_part_usage_body_element".to_string(),
                            expected: Some("valid part usage body element".to_string()),
                            found: recovery_found_snippet(input),
                            suggestion: None,
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
    if let Ok((input, _)) = tag::<_, _, nom::error::Error<Input>>(&b";"[..]).parse(input) {
        return Ok((input, (ConnectBody::Semicolon, vec![])));
    }

    let (mut input, _) = tag(&b"{"[..]).parse(input)?;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().starts_with(b"}") {
            let (input, _) = tag(&b"}"[..]).parse(input)?;
            return Ok((input, (ConnectBody::Brace, elements)));
        }
        let (next, element) = interface_usage_body_element(input)?;
        if next.location_offset() == input.location_offset() {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Many0,
            )));
        }
        elements.push(element);
        input = next;
    }
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

/// Ref in part usage body: `ref` name `:` type body.
fn part_ref_usage(input: Input<'_>) -> IResult<Input<'_>, Node<RefDecl>> {
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
        map(metadata_annotation, PartUsageBodyElement::MetadataAnnotation),
        map(perform_action_decl, PartUsageBodyElement::Perform),
        map(perform_usage, PartUsageBodyElement::Perform),
        map(allocate_, PartUsageBodyElement::Allocate),
        map(attribute_usage, PartUsageBodyElement::AttributeUsage),
        map(attribute_usage_shorthand, PartUsageBodyElement::AttributeUsage),
        map(part_usage, |p| PartUsageBodyElement::PartUsage(Box::new(p))),
        map(port_usage, PartUsageBodyElement::PortUsage),
        map(part_ref_usage, PartUsageBodyElement::Ref),
        map(bind_, PartUsageBodyElement::Bind),
        map(interface_usage, PartUsageBodyElement::InterfaceUsage),
        map(connect_, PartUsageBodyElement::Connect),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}
