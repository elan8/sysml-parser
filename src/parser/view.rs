//! View, viewpoint, and rendering parsing (SysML v2 Clause 8.2.2.26).
#![allow(dead_code, unused_imports)]

use crate::ast::{
    ExposeMember, FilterMember, Node, RenderingDef, RenderingDefBody, RenderingUsage,
    SatisfyViewMember, ViewBody, ViewBodyElement, ViewDef, ViewDefBody, ViewDefBodyElement,
    ViewRenderingUsage, ViewUsage, ViewpointDef, ViewpointUsage,
};
use crate::parser::interface::connect_body;
use crate::parser::lex::{
    identification, name, qualified_name, recover_body_element, skip_until_brace_end,
    starts_with_any_keyword, take_until_terminator, ws1, ws_and_comments, VIEW_BODY_STARTERS,
    VIEW_DEF_BODY_STARTERS,
};
use crate::parser::node_from_to;
use crate::parser::requirement::{doc_comment, requirement_def_body};
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, success};
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};

fn keyword_view_def(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"view"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    Ok((input, ()))
}

fn view_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<ViewDefBodyElement>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, elem) = alt((
        map(doc_comment, ViewDefBodyElement::Doc),
        map(view_filter_member, ViewDefBodyElement::Filter),
        map(view_rendering_usage, ViewDefBodyElement::ViewRendering),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

fn view_filter_member(input: Input<'_>) -> IResult<Input<'_>, Node<FilterMember>> {
    crate::parser::package::filter_member(input)
}

fn view_rendering_usage(input: Input<'_>) -> IResult<Input<'_>, Node<ViewRenderingUsage>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"render"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, type_name) = nom::combinator::opt(preceded(
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
            ViewRenderingUsage {
                name: name_str,
                type_name,
                body,
            },
        ),
    ))
}

fn view_def_body(input: Input<'_>) -> IResult<Input<'_>, ViewDefBody> {
    let (mut input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b";") {
        let (input, _) = tag(&b";"[..]).parse(input)?;
        return Ok((input, ViewDefBody::Semicolon));
    }
    let (next, _) = tag(&b"{"[..]).parse(input)?;
    input = next;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().starts_with(b"}") {
            let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
            return Ok((input, ViewDefBody::Brace { elements }));
        }
        match view_def_body_element(input) {
            Ok((next, element)) => {
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                elements.push(element);
                input = next;
            }
            Err(_) if starts_with_any_keyword(input.fragment(), VIEW_DEF_BODY_STARTERS) => {
                let (next, _) = recover_body_element(input, VIEW_DEF_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                input = next;
            }
            Err(_) => {
                let (input, _) = skip_until_brace_end(input)?;
                let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
                return Ok((input, ViewDefBody::Brace { elements }));
            }
        }
    }
}

pub(crate) fn view_def(input: Input<'_>) -> IResult<Input<'_>, Node<ViewDef>> {
    let start = input;
    let (input, _) = keyword_view_def(input)?;
    let (input, ident) = identification(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = view_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            ViewDef {
                identification: ident,
                body,
            },
        ),
    ))
}

fn keyword_viewpoint_def(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"viewpoint"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    Ok((input, ()))
}

pub(crate) fn viewpoint_def(input: Input<'_>) -> IResult<Input<'_>, Node<ViewpointDef>> {
    let start = input;
    let (input, _) = keyword_viewpoint_def(input)?;
    let (input, ident) = identification(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = requirement_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            ViewpointDef {
                identification: ident,
                body,
            },
        ),
    ))
}

fn keyword_rendering_def(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"rendering"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    Ok((input, ()))
}

fn rendering_def_body(input: Input<'_>) -> IResult<Input<'_>, RenderingDefBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(&b";"[..]), |_| RenderingDefBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                crate::parser::lex::skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| RenderingDefBody::Brace,
        ),
    ))
    .parse(input)
}

pub(crate) fn rendering_def(input: Input<'_>) -> IResult<Input<'_>, Node<RenderingDef>> {
    let start = input;
    let (input, _) = keyword_rendering_def(input)?;
    let (input, ident) = identification(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = rendering_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            RenderingDef {
                identification: ident,
                body,
            },
        ),
    ))
}

fn view_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<ViewBodyElement>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, elem) = alt((
        map(doc_comment, ViewBodyElement::Doc),
        map(view_filter_member, ViewBodyElement::Filter),
        map(view_rendering_usage, ViewBodyElement::ViewRendering),
        map(expose_member, ViewBodyElement::Expose),
        map(satisfy_view_member, ViewBodyElement::Satisfy),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

/// expose (MembershipImport | NamespaceImport) RelationshipBody
/// MembershipImport = QualifiedName (::**)?
/// NamespaceImport = QualifiedName :: * (::**)?
fn expose_member(input: Input<'_>) -> IResult<Input<'_>, Node<ExposeMember>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"expose"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, first) = qualified_name.parse(input)?;
    let (input, target) = alt((
        // ::*::** (try before ::* since * would consume first char of **)
        map(
            (
                preceded(ws_and_comments, tag(&b"::"[..])),
                preceded(ws_and_comments, tag(&b"*"[..])),
                preceded(ws_and_comments, tag(&b"::"[..])),
                preceded(ws_and_comments, tag(&b"**"[..])),
            ),
            |_| format!("{}::*::**", first),
        ),
        // ::** (try before ::*)
        map(
            (
                preceded(ws_and_comments, tag(&b"::"[..])),
                preceded(ws_and_comments, tag(&b"**"[..])),
            ),
            |_| format!("{}::**", first),
        ),
        // ::*
        map(
            (
                preceded(ws_and_comments, tag(&b"::"[..])),
                preceded(ws_and_comments, tag(&b"*"[..])),
            ),
            |_| format!("{}::*", first),
        ),
        // plain
        map(success(()), |_| first.clone()),
    ))
    .parse(input)?;
    // Optional filter [ expr ] - skip content to reach body
    let (input, _) = nom::combinator::opt(nom::sequence::delimited(
        preceded(ws_and_comments, tag(&b"["[..])),
        nom::bytes::complete::take_until(&b"]"[..]),
        preceded(ws_and_comments, tag(&b"]"[..])),
    ))
    .parse(input)?;
    let (input, body) = connect_body(input)?;
    Ok((
        input,
        node_from_to(start, input, ExposeMember { target, body }),
    ))
}

/// satisfy QualifiedName RelationshipBody (simplified form in view body)
fn satisfy_view_member(input: Input<'_>) -> IResult<Input<'_>, Node<SatisfyViewMember>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"satisfy"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, viewpoint_ref) = qualified_name.parse(input)?;
    let (input, body) = connect_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            SatisfyViewMember {
                viewpoint_ref,
                body,
            },
        ),
    ))
}

fn view_body(input: Input<'_>) -> IResult<Input<'_>, ViewBody> {
    let (mut input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b";") {
        let (input, _) = tag(&b";"[..]).parse(input)?;
        return Ok((input, ViewBody::Semicolon));
    }
    let (next, _) = tag(&b"{"[..]).parse(input)?;
    input = next;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().starts_with(b"}") {
            let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
            return Ok((input, ViewBody::Brace { elements }));
        }
        match view_body_element(input) {
            Ok((next, element)) => {
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                elements.push(element);
                input = next;
            }
            Err(_) if starts_with_any_keyword(input.fragment(), VIEW_BODY_STARTERS) => {
                let (next, _) = recover_body_element(input, VIEW_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                input = next;
            }
            Err(_) => {
                let (input, _) = skip_until_brace_end(input)?;
                let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
                return Ok((input, ViewBody::Brace { elements }));
            }
        }
    }
}

pub(crate) fn view_usage(input: Input<'_>) -> IResult<Input<'_>, Node<ViewUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"view"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, type_name) = preceded(
        ws_and_comments,
        nom::combinator::opt(preceded(
            tag(&b":"[..]),
            preceded(ws_and_comments, qualified_name),
        )),
    )
    .parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = view_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            ViewUsage {
                name: name_str,
                type_name,
                body,
            },
        ),
    ))
}

pub(crate) fn viewpoint_usage(input: Input<'_>) -> IResult<Input<'_>, Node<ViewpointUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"viewpoint"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = requirement_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            ViewpointUsage {
                name: name_str,
                type_name,
                body,
            },
        ),
    ))
}

pub(crate) fn rendering_usage(input: Input<'_>) -> IResult<Input<'_>, Node<RenderingUsage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"rendering"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, type_name) = preceded(
        ws_and_comments,
        nom::combinator::opt(preceded(
            tag(&b":"[..]),
            preceded(ws_and_comments, qualified_name),
        )),
    )
    .parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = connect_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            RenderingUsage {
                name: name_str,
                type_name,
                body,
            },
        ),
    ))
}
