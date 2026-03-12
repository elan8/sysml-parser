//! Enumeration definition parsing (BNF EnumerationDefinition).

use crate::ast::{EnumDef, EnumerationBody, Node};
use crate::parser::lex::{identification, name, ws1, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::preceded;
use nom::Parser;
use nom::IResult;

/// Enumerated value: optional `enum` keyword + name + `;`
fn enumerated_value(input: Input<'_>) -> IResult<Input<'_>, String> {
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"enum"[..]), ws1)).parse(input)?;
    let (input, n) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((input, n))
}

fn enumeration_body(input: Input<'_>) -> IResult<Input<'_>, EnumerationBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| EnumerationBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                preceded(
                    ws_and_comments,
                    many0(preceded(ws_and_comments, enumerated_value)),
                ),
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |values| EnumerationBody::Brace { values },
        ),
    ))
    .parse(input)
}

/// Enumeration definition: `enum def` Identification EnumerationBody.
pub(crate) fn enum_def(input: Input<'_>) -> IResult<Input<'_>, Node<EnumDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"enum"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, body) = enumeration_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            EnumDef {
                identification,
                body,
            },
        ),
    ))
}
