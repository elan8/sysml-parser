use crate::ast::{DefinitionBody, FlowDef, FlowUsage, Node};
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

pub(crate) fn flow_def(input: Input<'_>) -> IResult<Input<'_>, Node<FlowDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"flow"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = definition_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            FlowDef {
                identification,
                body,
            },
        ),
    ))
}

pub(crate) fn flow_usage(input: Input<'_>) -> IResult<Input<'_>, Node<FlowUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = alt((tag(&b"flow"[..]), tag(&b"message"[..]))).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, type_name) = opt(preceded(
        preceded(ws_and_comments, tag(&b":"[..])),
        preceded(ws_and_comments, qualified_name),
    ))
    .parse(input)?;
    let (input, from) = opt(preceded(
        preceded(ws_and_comments, tag(&b"from"[..])),
        preceded(ws1, expression),
    ))
    .parse(input)?;
    let (input, to) = match from {
        Some(_) => {
            let (input, _) = preceded(ws_and_comments, tag(&b"to"[..])).parse(input)?;
            let (input, to) = preceded(ws1, expression).parse(input)?;
            (input, Some(to))
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
            FlowUsage {
                name: name_str,
                type_name,
                from,
                to,
                body,
            },
        ),
    ))
}
