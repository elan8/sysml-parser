use crate::ast::{
    CommentAnnotation, DocComment, Node, RequireConstraint, RequirementDef, RequirementDefBody,
    RequirementDefBodyElement, SubjectDecl, Satisfy, RequirementUsage, ConstraintBody, TextualRepresentation,
};
use crate::parser::expr::expression;
use crate::parser::lex::{identification, name, ws, ws1, ws_and_comments, skip_until_brace_end, qualified_name};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
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

pub(crate) fn requirement_def_body(input: Input<'_>) -> IResult<Input<'_>, RequirementDefBody> {
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

/// KerML STRING_VALUE: double-quoted string, returns the inner string.
fn string_value(input: Input<'_>) -> IResult<Input<'_>, String> {
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag(&b"\""[..]).parse(input)?;
    let frag = input.fragment();
    let mut i = 0usize;
    while i < frag.len() {
        if frag[i] == b'\\' && i + 1 < frag.len() {
            i += 2;
            continue;
        }
        if frag[i] == b'"' {
            let s = String::from_utf8_lossy(&frag[..i]).replace("\\\"", "\"");
            let (input, _) = nom::bytes::complete::take(i + 1).parse(input)?;
            return Ok((input, s));
        }
        i += 1;
    }
    let s = String::from_utf8_lossy(frag).replace("\\\"", "\"");
    let (input, _) = nom::bytes::complete::take(frag.len()).parse(input)?;
    Ok((input, s))
}

/// KerML Documentation: 'doc' Identification? ( 'locale' STRING_VALUE )? body = REGULAR_COMMENT.
/// We only parse optional Identification and locale when the next token is not "/*", so that
/// ws_and_comments inside identification does not consume the doc body.
pub(crate) fn doc_comment(input: Input<'_>) -> IResult<Input<'_>, Node<DocComment>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"doc"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, ident_parsed, locale) = if input.fragment().starts_with(b"/*") {
        (input, None, None)
    } else {
        let (input, ident_parsed) = opt(identification).parse(input)?;
        let (input, locale) = opt(preceded(
            preceded(ws_and_comments, tag(&b"locale"[..])),
            preceded(ws1, string_value),
        ))
        .parse(input)?;
        (input, ident_parsed, locale)
    };
    // Use ws (not ws_and_comments) so we don't consume the doc body as a block comment.
    let (input, _) = preceded(ws, tag(&b"/*"[..])).parse(input)?;
    let (input, text_bytes) = nom::bytes::complete::take_until("*/").parse(input)?;
    let (input, _) = tag(&b"*/"[..]).parse(input)?;
    let text = String::from_utf8_lossy(text_bytes.fragment()).to_string();
    let ident = ident_parsed.and_then(|i| {
        if i.short_name.is_some() || i.name.is_some() {
            Some(i)
        } else {
            None
        }
    });
    Ok((
        input,
        node_from_to(
            start,
            input,
            DocComment {
                identification: ident,
                locale,
                text,
            },
        ),
    ))
}

/// KerML Comment: ( 'comment' Identification? )? ( 'locale' STRING_VALUE )? body = REGULAR_COMMENT.
pub(crate) fn comment_annotation(input: Input<'_>) -> IResult<Input<'_>, Node<CommentAnnotation>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"comment"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, ident_parsed) = opt(identification).parse(input)?;
    let (input, locale) = opt(preceded(
        preceded(ws_and_comments, tag(&b"locale"[..])),
        preceded(ws1, string_value),
    ))
    .parse(input)?;
    // Use ws so we don't consume the comment body as a block comment.
    let (input, _) = preceded(ws, tag(&b"/*"[..])).parse(input)?;
    let (input, text_bytes) = nom::bytes::complete::take_until("*/").parse(input)?;
    let (input, _) = tag(&b"*/"[..]).parse(input)?;
    let text = String::from_utf8_lossy(text_bytes.fragment()).to_string();
    let ident = ident_parsed.and_then(|i| {
        if i.short_name.is_some() || i.name.is_some() {
            Some(i)
        } else {
            None
        }
    });
    Ok((
        input,
        node_from_to(
            start,
            input,
            CommentAnnotation {
                identification: ident,
                locale,
                text,
            },
        ),
    ))
}

/// KerML TextualRepresentation: ( 'rep' Identification )? 'language' STRING_VALUE body = REGULAR_COMMENT.
pub(crate) fn textual_representation(input: Input<'_>) -> IResult<Input<'_>, Node<TextualRepresentation>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, rep_identification) = opt(preceded(
        preceded(tag(&b"rep"[..]), ws1),
        identification,
    ))
    .parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"language"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, language) = string_value(input)?;
    // Use ws so we don't consume the body as a block comment.
    let (input, _) = preceded(ws, tag(&b"/*"[..])).parse(input)?;
    let (input, text_bytes) = nom::bytes::complete::take_until("*/").parse(input)?;
    let (input, _) = tag(&b"*/"[..]).parse(input)?;
    let text = String::from_utf8_lossy(text_bytes.fragment()).to_string();
    let rep_id = rep_identification.and_then(|i| {
        if i.short_name.is_some() || i.name.is_some() {
            Some(i)
        } else {
            None
        }
    });
    Ok((
        input,
        node_from_to(
            start,
            input,
            TextualRepresentation {
                rep_identification: rep_id,
                language,
                text,
            },
        ),
    ))
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
