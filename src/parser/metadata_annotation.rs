//! Metadata/annotation parsing helpers.

use crate::ast::{Annotation, MetadataAnnotation, Node};
use crate::parser::interface::connect_body;
use crate::parser::lex::{qualified_name, take_until_terminator, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::bytes::complete::tag;
use nom::combinator::opt;
use nom::sequence::preceded;
use nom::IResult;
use nom::Parser;

/// Metadata usage: @ Identification ( : Type )? MetadataBody
/// Simplified: @ name ( : qualified_name )? ( ; | { ... } )
pub(crate) fn metadata_annotation(
    input: Input<'_>,
) -> IResult<Input<'_>, Node<MetadataAnnotation>> {
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

/// Generic annotation usage: either `@Name ...` or `#something ...`, followed by `;` or `{ ... }`.
pub(crate) fn annotation(input: Input<'_>) -> IResult<Input<'_>, Node<Annotation>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b"@") {
        let (input, _) = tag(&b"@"[..]).parse(input)?;
        let (input, _) = ws_and_comments(input)?;
        let (input, head) = qualified_name.parse(input)?;
        let (input, type_name) = opt(preceded(
            preceded(ws_and_comments, tag(&b":"[..])),
            preceded(ws_and_comments, qualified_name),
        ))
        .parse(input)?;
        let (input, body) = connect_body(input)?;
        return Ok((
            input,
            node_from_to(
                start,
                input,
                Annotation {
                    sigil: "@".to_string(),
                    head,
                    type_name,
                    body,
                },
            ),
        ));
    }

    let (input, _) = tag(&b"#"[..]).parse(input)?;
    let (input, head) = take_until_terminator(input, b";{")?;
    let (input, body) = connect_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            Annotation {
                sigil: "#".to_string(),
                head: head.trim().to_string(),
                type_name: None,
                body,
            },
        ),
    ))
}
