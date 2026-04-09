//! Occurrence definition and usage parsing.

use crate::ast::{
    DefinitionBody, Node, OccurrenceBodyElement, OccurrenceDef, OccurrenceUsage,
    OccurrenceUsageBody, ParseErrorNode,
};
use crate::parser::attribute::attribute_usage;
use crate::parser::build_recovery_error_node;
use crate::parser::lex::{
    identification, name, qualified_name, recover_body_element, skip_until_brace_end, ws1,
    ws_and_comments,
};
use crate::parser::metadata_annotation::annotation;
use crate::parser::node_from_to;
use crate::parser::part::part_usage;
use crate::parser::requirement::doc_comment;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::sequence::preceded;
use nom::IResult;
use nom::Parser;

const OCCURRENCE_BODY_STARTERS: &[&[u8]] = &[
    b"doc",
    b"attribute",
    b"part",
    b"individual",
    b"occurrence",
    b"snapshot",
    b"timeslice",
    b"@",
    b"#",
];

fn definition_body(input: Input<'_>) -> IResult<Input<'_>, DefinitionBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| DefinitionBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| DefinitionBody::Brace,
        ),
    ))
    .parse(input)
}

pub(crate) fn occurrence_def(input: Input<'_>) -> IResult<Input<'_>, Node<OccurrenceDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, is_abstract) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1))
        .parse(input)
        .map(|(i, o)| (i, o.is_some()))?;
    let (input, _) = tag(&b"occurrence"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, body) = definition_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            OccurrenceDef {
                is_abstract,
                identification,
                body,
            },
        ),
    ))
}

pub(crate) fn occurrence_usage(input: Input<'_>) -> IResult<Input<'_>, Node<OccurrenceUsage>> {
    occurrence_usage_with_modifiers(input, false, None)
}

pub(crate) fn individual_usage(input: Input<'_>) -> IResult<Input<'_>, Node<OccurrenceUsage>> {
    let (input, _) = preceded(ws_and_comments, tag(&b"individual"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    occurrence_usage_tail(input, true, None)
}

pub(crate) fn snapshot_usage(input: Input<'_>) -> IResult<Input<'_>, Node<OccurrenceUsage>> {
    let (input, _) = preceded(ws_and_comments, tag(&b"snapshot"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    occurrence_usage_tail(input, false, Some("snapshot".to_string()))
}

pub(crate) fn timeslice_usage(input: Input<'_>) -> IResult<Input<'_>, Node<OccurrenceUsage>> {
    let (input, _) = preceded(ws_and_comments, tag(&b"timeslice"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    occurrence_usage_tail(input, false, Some("timeslice".to_string()))
}

fn occurrence_usage_with_modifiers(
    input: Input<'_>,
    is_individual: bool,
    portion_kind: Option<String>,
) -> IResult<Input<'_>, Node<OccurrenceUsage>> {
    let (input, _) = preceded(ws_and_comments, tag(&b"occurrence"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    occurrence_usage_tail(input, is_individual, portion_kind)
}

fn occurrence_usage_tail(
    input: Input<'_>,
    is_individual: bool,
    portion_kind: Option<String>,
) -> IResult<Input<'_>, Node<OccurrenceUsage>> {
    let start = input;
    let (input, name_str) = name(input)?;
    let (input, type_name) = opt(preceded(
        preceded(ws_and_comments, tag(&b":"[..])),
        preceded(ws_and_comments, qualified_name),
    ))
    .parse(input)?;
    let (input, body) = occurrence_usage_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            OccurrenceUsage {
                is_individual,
                portion_kind,
                name: name_str,
                type_name,
                body,
            },
        ),
    ))
}

fn occurrence_usage_body(input: Input<'_>) -> IResult<Input<'_>, OccurrenceUsageBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| OccurrenceUsageBody::Semicolon),
        occurrence_usage_body_brace,
    ))
    .parse(input)
}

fn occurrence_usage_body_brace(input: Input<'_>) -> IResult<Input<'_>, OccurrenceUsageBody> {
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
            return Ok((input, OccurrenceUsageBody::Brace { elements }));
        }
        match occurrence_body_element(input) {
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
                let (next, _) = recover_body_element(input, OCCURRENCE_BODY_STARTERS)?;
                if next.location_offset() == start_unknown.location_offset() {
                    let (input, _) = skip_until_brace_end(input)?;
                    let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
                    return Ok((input, OccurrenceUsageBody::Brace { elements }));
                }
                let recovery = build_recovery_error_node(
                    start_unknown,
                    OCCURRENCE_BODY_STARTERS,
                    "occurrence body",
                    "recovered_occurrence_body_element",
                );
                let node: Node<ParseErrorNode> = node_from_to(start_unknown, next, recovery);
                elements.push(node_from_to(
                    start_unknown,
                    next,
                    OccurrenceBodyElement::Error(node),
                ));
                input = next;
            }
        }
    }
}

fn occurrence_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<OccurrenceBodyElement>> {
    let (input, _) = ws_and_comments(input)?;
    let start = input;
    let (input, elem) = alt((
        map(doc_comment, OccurrenceBodyElement::Doc),
        map(annotation, OccurrenceBodyElement::Annotation),
        map(attribute_usage, OccurrenceBodyElement::AttributeUsage),
        map(part_usage, |p| OccurrenceBodyElement::PartUsage(Box::new(p))),
        map(individual_usage, |n| {
            OccurrenceBodyElement::OccurrenceUsage(Box::new(n))
        }),
        map(snapshot_usage, |n| {
            OccurrenceBodyElement::OccurrenceUsage(Box::new(n))
        }),
        map(timeslice_usage, |n| {
            OccurrenceBodyElement::OccurrenceUsage(Box::new(n))
        }),
        map(occurrence_usage, |n| {
            OccurrenceBodyElement::OccurrenceUsage(Box::new(n))
        }),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}
