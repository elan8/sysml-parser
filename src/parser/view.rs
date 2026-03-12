//! View, viewpoint, and rendering parsing (SysML v2 Clause 8.2.2.26).

use crate::ast::{
    FilterMember, Node, ViewBody, ViewBodyElement, ViewDef, ViewDefBody, ViewDefBodyElement,
    ViewRenderingUsage, ViewUsage, ViewpointDef, ViewpointUsage, RenderingDef, RenderingDefBody,
    RenderingUsage,
};
use crate::parser::interface::connect_body;
use crate::parser::lex::{identification, name, qualified_name, ws1, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::requirement::{doc_comment, requirement_def_body};
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};

fn keyword_view_def(input: Input<'_>) -> IResult<Input<'_>, ()> {
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
    alt((
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| ViewDefBody::Semicolon),
        map(
            delimited(
                preceded(ws_and_comments, tag(&b"{"[..])),
                many0(preceded(ws_and_comments, view_def_body_element)),
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |elements| ViewDefBody::Brace { elements },
        ),
    ))
    .parse(input)
}

pub(crate) fn view_def(input: Input<'_>) -> IResult<Input<'_>, Node<ViewDef>> {
    let start = input;
    let (input, _) = keyword_view_def(input)?;
    let (input, ident) = identification(input)?;
    let (input, body) = view_def_body(input)?;
    Ok((input, node_from_to(start, input, ViewDef { identification: ident, body })))
}

fn keyword_viewpoint_def(input: Input<'_>) -> IResult<Input<'_>, ()> {
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
    let (input, body) = requirement_def_body(input)?;
    Ok((input, node_from_to(start, input, ViewpointDef { identification: ident, body })))
}

fn keyword_rendering_def(input: Input<'_>) -> IResult<Input<'_>, ()> {
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
    let (input, body) = rendering_def_body(input)?;
    Ok((input, node_from_to(start, input, RenderingDef { identification: ident, body })))
}

fn view_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<ViewBodyElement>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, elem) = alt((
        map(doc_comment, ViewBodyElement::Doc),
        map(view_filter_member, ViewBodyElement::Filter),
        map(view_rendering_usage, ViewBodyElement::ViewRendering),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

fn view_body(input: Input<'_>) -> IResult<Input<'_>, ViewBody> {
    alt((
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| ViewBody::Semicolon),
        map(
            delimited(
                preceded(ws_and_comments, tag(&b"{"[..])),
                many0(preceded(ws_and_comments, view_body_element)),
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |elements| ViewBody::Brace { elements },
        ),
    ))
    .parse(input)
}

pub(crate) fn view_usage(input: Input<'_>) -> IResult<Input<'_>, Node<ViewUsage>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"view"[..])).parse(input)?;
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
    let (input, _) = preceded(ws_and_comments, tag(&b"viewpoint"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
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
    let (input, _) = preceded(ws_and_comments, tag(&b"rendering"[..])).parse(input)?;
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
