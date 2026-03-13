use crate::ast::{
    CommentAnnotation, ConcernUsage, DocComment, FrameMember, Node, RequireConstraint,
    RequirementDef, RequirementDefBody, RequirementDefBodyElement, RequirementUsage,
    ConstraintBody, ParseErrorNode, SubjectDecl, Satisfy, TextualRepresentation,
};
use crate::parser::expr::expression;
use crate::parser::import::import_;
use crate::parser::lex::{
    identification, name, qualified_name, recover_body_element, skip_until_brace_end,
    starts_with_any_keyword, ws, ws1, ws_and_comments, REQUIREMENT_BODY_STARTERS,
};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};

fn recovery_found_snippet(input: Input<'_>) -> Option<String> {
    let frag = input.fragment();
    let take = frag
        .iter()
        .position(|&b| b == b'\n' || b == b'\r')
        .unwrap_or(frag.len())
        .min(60);
    let snippet = String::from_utf8_lossy(&frag[..take]).trim().to_string();
    if snippet.is_empty() {
        None
    } else {
        Some(snippet)
    }
}

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
        requirement_def_body_brace,
    ))
    .parse(input)
}

fn requirement_def_body_brace(input: Input<'_>) -> IResult<Input<'_>, RequirementDefBody> {
    let (mut input, _) = preceded(ws_and_comments, tag(&b"{"[..])).parse(input)?;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().starts_with(b"}") {
            let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
            return Ok((input, RequirementDefBody::Brace { elements }));
        }
        match requirement_def_body_element(input) {
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
            Err(_) if starts_with_any_keyword(input.fragment(), REQUIREMENT_BODY_STARTERS) => {
                let (next, _) = recover_body_element(input, REQUIREMENT_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                elements.push(node_from_to(
                    input,
                    next,
                    RequirementDefBodyElement::Error(Node::new(
                        crate::ast::Span::dummy(),
                        ParseErrorNode {
                            message: "recovered requirement body element".to_string(),
                            found: recovery_found_snippet(input),
                        },
                    )),
                ));
                input = next;
            }
            Err(_) => {
                return Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Tag,
                )));
            }
        }
    }
}

fn requirement_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<RequirementDefBodyElement>> {
    let start = input;
    let (rest, elem) = alt((
        map(import_, RequirementDefBodyElement::Import),
        map(subject_decl, RequirementDefBodyElement::SubjectDecl),
        map(require_constraint, RequirementDefBodyElement::RequireConstraint),
        map(frame_member, RequirementDefBodyElement::Frame),
        map(doc_comment, RequirementDefBodyElement::Doc),
    ))
    .parse(input)?;
    Ok((rest, node_from_to(start, rest, elem)))
}

fn frame_member(input: Input<'_>) -> IResult<Input<'_>, Node<FrameMember>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"frame"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, n) = name(input)?;
    let (input, body) = requirement_def_body(input)?;
    Ok((input, node_from_to(start, input, FrameMember { name: n, body })))
}

pub(crate) fn subject_decl(input: Input<'_>) -> IResult<Input<'_>, Node<SubjectDecl>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"subject"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, n) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
    let (input, _) = alt((
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| ()),
        map(
            delimited(
                preceded(ws_and_comments, tag(&b"{"[..])),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| (),
        ),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, SubjectDecl { name: n, type_name })))
}

pub(crate) fn require_constraint(input: Input<'_>) -> IResult<Input<'_>, Node<RequireConstraint>> {
    let start = input;
    let (input, _) = preceded(
        ws_and_comments,
        alt((tag(&b"require"[..]), tag(&b"assume"[..]))),
    )
    .parse(input)?;
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
    let (input, body) = alt((
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| crate::ast::ConnectBody::Semicolon),
        map(
            delimited(
                preceded(ws_and_comments, tag(&b"{"[..])),
                skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| crate::ast::ConnectBody::Brace,
        ),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, Satisfy { source, target, body })))
}

pub(crate) fn concern_usage(input: Input<'_>) -> IResult<Input<'_>, Node<ConcernUsage>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"concern"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, ident) = name(input)?;
    let (input, type_name) = opt(preceded(
        preceded(ws_and_comments, tag(&b":"[..])),
        preceded(ws_and_comments, qualified_name),
    ))
    .parse(input)?;
    let (input, body) = requirement_def_body(input)?;
    let val = ConcernUsage { name: ident, type_name, body };
    Ok((input, node_from_to(start, input, val)))
}

pub(crate) fn requirement_usage(input: Input<'_>) -> IResult<Input<'_>, Node<RequirementUsage>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"requirement"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, ident) = name(input)?;
    let (input, type_name) = opt(preceded(
        preceded(ws_and_comments, tag(&b":"[..])),
        preceded(ws_and_comments, qualified_name),
    ))
    .parse(input)?;
    let (input, body) = requirement_def_body(input)?;
    let val = RequirementUsage { name: ident, type_name, body };
    Ok((input, node_from_to(start, input, val)))
}
