//! Enumeration definition parsing (BNF EnumerationDefinition).

use crate::ast::{EnumDef, EnumerationBody, Node};
use crate::parser::lex::{
    identification, name, skip_until_brace_end, take_until_terminator, ws1, ws_and_comments,
};
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
    alt((
        map(
            (
                name,
                nom::combinator::opt(preceded(
                    preceded(ws_and_comments, tag(&b"="[..])),
                    preceded(ws_and_comments, |i| take_until_terminator(i, b";")),
                )),
                preceded(ws_and_comments, tag(&b";"[..])),
            ),
            |(n, _, _)| n,
        ),
        map(
            nom::sequence::terminated(
                nom::bytes::complete::take_until(&b";"[..]),
                preceded(ws_and_comments, tag(&b";"[..])),
            ),
            |raw: Input<'_>| String::from_utf8_lossy(raw.fragment()).trim().to_string(),
        ),
    ))
    .parse(input)
}

fn enumeration_body(input: Input<'_>) -> IResult<Input<'_>, EnumerationBody> {
    let (input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b";") {
        let (input, _) = tag(&b";"[..]).parse(input)?;
        return Ok((input, EnumerationBody::Semicolon));
    }
    let (input, _) = tag(&b"{"[..]).parse(input)?;
    let (after_values, values) = preceded(
        ws_and_comments,
        many0(preceded(ws_and_comments, enumerated_value)),
    )
    .parse(input)?;
    let (input, _) = if after_values.fragment().starts_with(b"}") {
        preceded(ws_and_comments, tag(&b"}"[..])).parse(after_values)?
    } else {
        let (input, _) = skip_until_brace_end(after_values)?;
        preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?
    };
    Ok((input, EnumerationBody::Brace { values }))
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
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
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
