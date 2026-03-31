use crate::ast::{AllocationDef, AllocationUsage, DefinitionBody, Node};
use crate::parser::expr::expression;
use crate::parser::lex::{
    identification, name, qualified_name, skip_until_brace_end, take_until_terminator, ws1,
    ws_and_comments,
};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::sequence::preceded;
use nom::IResult;
use nom::Parser;

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

pub(crate) fn allocation_def(input: Input<'_>) -> IResult<Input<'_>, Node<AllocationDef>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"allocation"[..])).parse(input)?;
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
            AllocationDef {
                identification,
                body,
            },
        ),
    ))
}

pub(crate) fn allocation_usage(input: Input<'_>) -> IResult<Input<'_>, Node<AllocationUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"allocation"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, type_name) = opt(preceded(
        preceded(ws_and_comments, tag(&b":"[..])),
        preceded(ws_and_comments, qualified_name),
    ))
    .parse(input)?;
    let (input, source) = opt(preceded(
        preceded(ws_and_comments, tag(&b"allocate"[..])),
        preceded(ws1, expression),
    ))
    .parse(input)?;
    let (input, target) = match source {
        Some(_) => {
            let (input, _) = preceded(ws_and_comments, tag(&b"to"[..])).parse(input)?;
            let (input, target) = preceded(ws1, expression).parse(input)?;
            (input, Some(target))
        }
        None => (input, None),
    };
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = definition_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            AllocationUsage {
                name: name_str,
                type_name,
                source,
                target,
                body,
            },
        ),
    ))
}

pub(crate) fn allocate_usage(input: Input<'_>) -> IResult<Input<'_>, Node<AllocationUsage>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"allocate"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, source) = expression(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"to"[..])).parse(input)?;
    let (input, target) = preceded(ws1, expression).parse(input)?;
    let (input, body) = definition_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            AllocationUsage {
                name: String::new(),
                type_name: None,
                source: Some(source),
                target: Some(target),
                body,
            },
        ),
    ))
}
