//! Metadata annotation parsing (BNF MetadataUsage, MetadataBody).
//! Parses @Name; or @Name{...} as used on part/port/attribute usages.

use crate::ast::{MetadataAnnotation, Node};
use crate::parser::interface::connect_body;
use crate::parser::lex::{qualified_name, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::bytes::complete::tag;
use nom::combinator::opt;
use nom::sequence::preceded;
use nom::Parser;
use nom::IResult;

/// Metadata usage: @ Identification ( : Type )? MetadataBody
/// Simplified: @ name ( : qualified_name )? ( ; | { ... } )
pub(crate) fn metadata_annotation(input: Input<'_>) -> IResult<Input<'_>, Node<MetadataAnnotation>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"@"[..])).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    // Name can be qualified (e.g. Safety, Security)
    let (input, name_str) = qualified_name.parse(input)?;
    let (input, type_name) = opt(preceded(
        preceded(ws_and_comments, tag(&b":"[..])),
        preceded(ws_and_comments, qualified_name),
    ))
    .parse(input)?;
    let (input, body) = connect_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            MetadataAnnotation {
                name: name_str,
                type_name,
                body,
            },
        ),
    ))
}
