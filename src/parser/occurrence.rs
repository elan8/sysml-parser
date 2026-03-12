//! Occurrence definition parsing (BNF OccurrenceDefinition).

use crate::ast::{DefinitionBody, Node, OccurrenceDef};
use crate::parser::lex::{identification, skip_until_brace_end, ws1, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::sequence::preceded;
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
