//! Individual definition parsing.

use crate::ast::{IndividualDef, Node};
use crate::parser::attribute::attribute_body;
use crate::parser::lex::{
    identification, qualified_name, specialization_operator, ws1, ws_and_comments,
};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::bytes::complete::tag;
use nom::combinator::opt;
use nom::sequence::preceded;
use nom::IResult;
use nom::Parser;

/// Individual definition: `individual def` Identification ( (`:>` | `specializes`) qualified_name )? body
pub(crate) fn individual_def(input: Input<'_>) -> IResult<Input<'_>, Node<IndividualDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"individual"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, specializes) = opt(preceded(
        preceded(ws_and_comments, specialization_operator),
        preceded(ws_and_comments, qualified_name),
    ))
    .parse(input)?;
    let (input, body) = attribute_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            IndividualDef {
                identification,
                specializes,
                body,
            },
        ),
    ))
}
