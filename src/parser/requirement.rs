use crate::ast::{
    Node, RequireConstraint, RequirementDef, RequirementDefBody, RequirementDefBodyElement, SubjectDecl,
    Satisfy, RequirementUsage, ConstraintBody, DocComment
};
use crate::parser::expr::expression;
use crate::parser::lex::{identification, name, ws1, ws_and_comments, skip_until_brace_end, take_until_terminator, qualified_name};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::multi::many0;
use nom::sequence::{delimited, preceded, tuple};
use nom::{IResult, Parser};

fn keyword_requirement_def(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = tag(&b"requirement"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    Ok((input, ()))
}

pub(crate) fn requirement_def(input: Input<'_>) -> IResult<Input<'_>, Node<RequirementDef>> {
    let start = input;
    let (input, _) = keyword_requirement_def(input)?;
    let (input, ident) = identification(input)?;
    let (input, body) = requirement_def_body(input)?;
    Ok((input, node_from_to(start, input, RequirementDef { identification: ident, body })))
}

fn requirement_def_body(input: Input<'_>) -> IResult<Input<'_>, RequirementDefBody> {
    alt((
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| RequirementDefBody::Semicolon),
        map(
            delimited(
                preceded(ws_and_comments, tag(&b"{"[..])),
                many0(preceded(ws_and_comments, requirement_def_body_element)),
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |elements| RequirementDefBody::Brace { elements },
        ),
    ))
    .parse(input)
}

fn requirement_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<RequirementDefBodyElement>> {
    let start = input;
    let mut parser = alt((
        map(subject_decl, |n| node_from_to(start, input, RequirementDefBodyElement::SubjectDecl(n))),
        map(require_constraint, |n| node_from_to(start, input, RequirementDefBodyElement::RequireConstraint(n))),
        map(doc_comment, |n| node_from_to(start, input, RequirementDefBodyElement::Doc(n))),
    ));
    parser.parse(input)
}

pub(crate) fn subject_decl(input: Input<'_>) -> IResult<Input<'_>, Node<SubjectDecl>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"subject"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, n) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((input, node_from_to(start, input, SubjectDecl { name: n, type_name })))
}

pub(crate) fn require_constraint(input: Input<'_>) -> IResult<Input<'_>, Node<RequireConstraint>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"require"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"constraint"[..]).parse(input)?;
    let (input, body) = constraint_body(input)?;
    Ok((input, node_from_to(start, input, RequireConstraint { body })))
}

pub(crate) fn constraint_body(input: Input<'_>) -> IResult<Input<'_>, ConstraintBody> {
    alt((
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| ConstraintBody::Semicolon),
        map(
            delimited(
                preceded(ws_and_comments, tag(&b"{"[..])),
                skip_until_brace_end, // Simplification for now, we just skip whatever is inside constraint body
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| ConstraintBody::Brace,
        ),
    ))
    .parse(input)
}

pub(crate) fn doc_comment(input: Input<'_>) -> IResult<Input<'_>, Node<DocComment>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"doc"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"/*"[..]).parse(input)?;
    let (input, text_bytes) = nom::bytes::complete::take_until("*/").parse(input)?;
    let (input, _) = tag(&b"*/"[..]).parse(input)?;
    // the lexer might skip the doc comment entirely if it's placed top-level, but here we parse it explicitely
    let text = String::from_utf8_lossy(text_bytes.fragment()).to_string();
    Ok((input, node_from_to(start, input, DocComment { text })))
}

pub(crate) fn satisfy(input: Input<'_>) -> IResult<Input<'_>, Node<Satisfy>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"satisfy"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, source) = expression(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"by"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, target) = expression(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((input, node_from_to(start, input, Satisfy { source, target, body: crate::ast::ConnectBody::Semicolon })))
}

pub(crate) fn requirement_usage(input: Input<'_>) -> IResult<Input<'_>, Node<RequirementUsage>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"requirement"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, ident) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
    let (input, body) = requirement_def_body(input)?;
    Ok((input, node_from_to(start, input, RequirementUsage { name: ident, type_name, body })))
}
