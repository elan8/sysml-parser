//! Import and relationship body parsing.

use crate::ast::{FilterPackageMember, Import, Node, Visibility};
use crate::parser::expr::expression;
use crate::parser::lex::{qualified_name, skip_until_brace_end, ws1, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::multi::many1;
use nom::sequence::{delimited, preceded};
use nom::IResult;
use nom::Parser;

/// RelationshipBody: ';' or '{' ... '}'. For '{' we skip content until matching '}'.
pub(crate) fn relationship_body(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| ()),
        map(
            delimited(
                tag(&b"{"[..]),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| (),
        ),
    ))
    .parse(input)
}

/// Import: visibility? 'import' isImportAll? (QualifiedName | QualifiedName '::' '*') RelationshipBody
pub(crate) fn import_(input: Input<'_>) -> IResult<Input<'_>, Node<Import>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, visibility) = opt(alt((
        map(preceded(tag(&b"public"[..]), ws1), |_| Visibility::Public),
        map(preceded(tag(&b"private"[..]), ws1), |_| Visibility::Private),
        map(preceded(tag(&b"protected"[..]), ws1), |_| {
            Visibility::Protected
        }),
    )))
    .parse(input)?;
    let (input, _) = tag(&b"import"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = opt(preceded(tag(&b"all"[..]), ws1)).parse(input)?;
    let (input, qname) = qualified_name.parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    // KerML: NamespaceImport = QualifiedName '::' '*' (::**)? | FilterPackage; MembershipImport = QualifiedName (::**)?
    let (input, target, is_import_all, is_recursive, filter_members) =
        if input.fragment().starts_with(b"::") {
            let (input, _) = preceded(ws_and_comments, tag(&b"::"[..])).parse(input)?;
            let (input, _) = ws_and_comments(input)?;
            if input.fragment().starts_with(b"*")
                && input.fragment().get(1).is_none_or(|c| *c != b'*')
            {
                let (input, _) = preceded(ws_and_comments, tag(&b"*"[..])).parse(input)?;
                let (input, rec_opt) = opt((
                    preceded(ws_and_comments, tag(&b"::"[..])),
                    preceded(ws_and_comments, tag(&b"**"[..])),
                ))
                .parse(input)?;
                (
                    input,
                    format!("{}::*", qname),
                    true,
                    rec_opt.is_some(),
                    None,
                )
            } else if input.fragment().starts_with(b"**") {
                let (input, _) = preceded(ws_and_comments, tag(&b"**"[..])).parse(input)?;
                let (input, filter_opt) = opt(many1(delimited(
                    preceded(ws_and_comments, tag(&b"["[..])),
                    preceded(ws_and_comments, expression),
                    preceded(ws_and_comments, tag(&b"]"[..])),
                )))
                .parse(input)?;
                let filter_members = filter_opt.map(|members| {
                    members
                        .into_iter()
                        .map(|e| Node::new(e.span.clone(), FilterPackageMember { expression: e }))
                        .collect()
                });
                (input, qname, false, true, filter_members)
            } else {
                return Err(nom::Err::Error(nom::error::make_error(
                    input,
                    nom::error::ErrorKind::Tag,
                )));
            }
        } else if input.fragment().starts_with(b"[") {
            // FilterPackage form: QualifiedName [ expr ] [ expr ]+
            let (input, members) = many1(delimited(
                preceded(ws_and_comments, tag(&b"["[..])),
                preceded(ws_and_comments, expression),
                preceded(ws_and_comments, tag(&b"]"[..])),
            ))
            .parse(input)?;
            let filter_members: Vec<Node<FilterPackageMember>> = members
                .into_iter()
                .map(|e| Node::new(e.span.clone(), FilterPackageMember { expression: e }))
                .collect();
            (input, qname, true, false, Some(filter_members))
        } else {
            let (input, rec_opt) = opt((
                preceded(ws_and_comments, tag(&b"::"[..])),
                preceded(ws_and_comments, tag(&b"**"[..])),
            ))
            .parse(input)?;
            (input, qname, false, rec_opt.is_some(), None)
        };
    let (input, _) = relationship_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            Import {
                visibility,
                is_import_all,
                target,
                is_recursive,
                filter_members,
            },
        ),
    ))
}
