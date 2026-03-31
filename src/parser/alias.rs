//! Alias definition parsing.

use crate::ast::{AliasBody, AliasDef, Node};
use crate::parser::lex::{
    identification, qualified_name, skip_until_brace_end, ws1, ws_and_comments,
};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::sequence::preceded;
use nom::IResult;
use nom::Parser;

/// Alias body: `;` or `{` ... `}`
fn alias_body(input: Input<'_>) -> IResult<Input<'_>, AliasBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| AliasBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| AliasBody::Brace,
        ),
    ))
    .parse(input)
}

/// Alias definition: `alias` Identification `for` qualified_name body
pub(crate) fn alias_def(input: Input<'_>) -> IResult<Input<'_>, Node<AliasDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"alias"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"for"[..])).parse(input)?;
    let (input, target) = preceded(ws1, qualified_name).parse(input)?;
    let (input, body) = alias_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            AliasDef {
                identification,
                target,
                body,
            },
        ),
    ))
}
