//! Import and relationship body parsing.

use crate::ast::{Import, Node, Visibility};
use crate::parser::lex::{qualified_name, skip_until_brace_end, ws1, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::sequence::{delimited, preceded, tuple};
use nom::IResult;

/// RelationshipBody: ';' or '{' ... '}'. For '{' we skip content until matching '}'.
pub(crate) fn relationship_body(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(b";"), |_| ()),
        map(
            delimited(
                tag(b"{"),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(b"}")),
            ),
            |_| (),
        ),
    ))(input)
}

/// Import: visibility? 'import' isImportAll? (QualifiedName | QualifiedName '::' '*') RelationshipBody
pub(crate) fn import_(input: Input<'_>) -> IResult<Input<'_>, Node<Import>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, visibility) = opt(alt((
        map(preceded(tag(b"public"), ws1), |_| Visibility::Public),
        map(preceded(tag(b"private"), ws1), |_| Visibility::Private),
        map(preceded(tag(b"protected"), ws1), |_| Visibility::Protected),
    )))(input)?;
    let (input, _) = tag(b"import")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = opt(preceded(tag(b"all"), ws1))(input)?;
    let (input, (target, is_import_all)) = alt((
        map(
            tuple((
                qualified_name,
                preceded(ws_and_comments, tag(b"::")),
                preceded(ws_and_comments, tag(b"*")),
            )),
            |(q, _, _)| {
                log::debug!("import_: parsed target (with ::*) = {}::*", q);
                (format!("{}::*", q), true)
            },
        ),
        map(qualified_name, |q| {
            log::debug!("import_: parsed target (no ::*) = {}", q);
            (q, false)
        }),
    ))(input)?;
    let (input, _) = relationship_body(input)?;
    Ok((
        input,
        node_from_to(start, input, Import {
            visibility,
            is_import_all,
            target,
        }),
    ))
}
