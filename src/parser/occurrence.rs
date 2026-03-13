//! Occurrence definition parsing (BNF OccurrenceDefinition).

use crate::ast::{DefinitionBody, Node, OccurrenceDef, OccurrenceUsage};
use crate::parser::lex::{identification, name, qualified_name, skip_until_brace_end, ws1, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::sequence::preceded;
use nom::Parser;
use nom::IResult;

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

/// Occurrence definition: `occurrence def` Identification body (optional `abstract` prefix).
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
    let (input, body) = definition_body(input)?;
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
