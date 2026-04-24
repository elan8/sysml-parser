#![allow(dead_code, unused_imports)]

use crate::ast::{
    CalcDef, CalcDefBody, CalcDefBodyElement, ConstraintDef, ConstraintDefBody,
    ConstraintDefBodyElement, Node, ParseErrorNode, ReturnDecl,
};
use crate::parser::action::in_out_decl;
use crate::parser::expr::expression;
use crate::parser::lex::{
    identification, name, qualified_name, recover_body_element, skip_statement_or_block,
    skip_until_brace_end, starts_with_any_keyword, starts_with_keyword, take_until_terminator, ws1,
    ws_and_comments, CALC_DEF_BODY_STARTERS, CONSTRAINT_DEF_BODY_STARTERS,
};
use crate::parser::Input;
use crate::parser::{build_recovery_error_node_from_span, node_from_to};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};

pub(crate) fn constraint_def(input: Input<'_>) -> IResult<Input<'_>, Node<ConstraintDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"private"[..]), ws1)).parse(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"private"[..]), ws1)).parse(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"constraint"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"def"[..]), ws1)).parse(input)?;
    let (input, ident) = identification(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = constraint_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            ConstraintDef {
                identification: ident,
                body,
            },
        ),
    ))
}

fn constraint_def_body(input: Input<'_>) -> IResult<Input<'_>, ConstraintDefBody> {
    let (input, body) = structured_constraint_body(input)?;
    let body = match body {
        StructuredConstraintBody::Semicolon => ConstraintDefBody::Semicolon,
        StructuredConstraintBody::Brace { elements } => ConstraintDefBody::Brace { elements },
    };
    Ok((input, body))
}

pub(crate) enum StructuredConstraintBody {
    Semicolon,
    Brace {
        elements: Vec<Node<ConstraintDefBodyElement>>,
    },
}

pub(crate) fn structured_constraint_body(
    input: Input<'_>,
) -> IResult<Input<'_>, StructuredConstraintBody> {
    let (mut input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b";") {
        let (input, _) = tag(&b";"[..]).parse(input)?;
        return Ok((input, StructuredConstraintBody::Semicolon));
    }
    let (next, _) = tag(&b"{"[..]).parse(input)?;
    input = next;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().starts_with(b"}") {
            let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
            return Ok((input, StructuredConstraintBody::Brace { elements }));
        }
        match constraint_def_body_element(input) {
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
            Err(_) if starts_with_any_keyword(input.fragment(), CONSTRAINT_DEF_BODY_STARTERS) => {
                let start_unknown = input;
                let (next, _) = recover_body_element(input, CONSTRAINT_DEF_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                let recovery = build_recovery_error_node_from_span(
                    start_unknown,
                    next,
                    CONSTRAINT_DEF_BODY_STARTERS,
                    "constraint body",
                    "recovered_constraint_body_element",
                );
                let node: Node<ParseErrorNode> = node_from_to(start_unknown, next, recovery);
                elements.push(node_from_to(
                    start_unknown,
                    next,
                    ConstraintDefBodyElement::Error(node),
                ));
                input = next;
            }
            Err(_) => {
                let start_unknown = input;
                let (next, _) = skip_statement_or_block(input)?;
                if next.location_offset() == start_unknown.location_offset() {
                    let (input, _) = skip_until_brace_end(input)?;
                    let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
                    return Ok((input, StructuredConstraintBody::Brace { elements }));
                }
                elements.push(node_from_to(
                    start_unknown,
                    next,
                    ConstraintDefBodyElement::Other(
                        String::from_utf8_lossy(
                            &start_unknown.fragment()[..start_unknown.fragment().len().min(60)],
                        )
                        .trim()
                        .to_string(),
                    ),
                ));
                input = next;
            }
        }
    }
}

pub(crate) fn constraint_def_body_element(
    input: Input<'_>,
) -> IResult<Input<'_>, Node<ConstraintDefBodyElement>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, elem) = if starts_with_keyword(input.fragment(), b"doc") {
        map(
            crate::parser::requirement::doc_comment,
            ConstraintDefBodyElement::Doc,
        )
        .parse(input)?
    } else if starts_with_keyword(input.fragment(), b"in")
        || starts_with_keyword(input.fragment(), b"out")
        || starts_with_keyword(input.fragment(), b"inout")
    {
        if named_in_out_missing_type(input) {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }
        map(in_out_decl, ConstraintDefBodyElement::InOutDecl).parse(input)?
    } else {
        map(expression, ConstraintDefBodyElement::Expression).parse(input)?
    };
    Ok((input, node_from_to(start, input, elem)))
}

fn safe_constraint_def_body_element(
    input: Input<'_>,
) -> IResult<Input<'_>, Node<ConstraintDefBodyElement>> {
    let start = input;
    let mut parser = alt((
        map(in_out_decl, |n| {
            node_from_to(start, input, ConstraintDefBodyElement::InOutDecl(n))
        }),
        map(expression, |n| {
            node_from_to(start, input, ConstraintDefBodyElement::Expression(n))
        }),
    ));
    parser.parse(input)
}

pub(crate) fn calc_def(input: Input<'_>) -> IResult<Input<'_>, Node<CalcDef>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"private"[..]), ws1)).parse(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"abstract"[..]), ws1)).parse(input)?;
    let (input, _) = tag(&b"calc"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = nom::combinator::opt(preceded(tag(&b"def"[..]), ws1)).parse(input)?;
    let (input, ident) = identification(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = take_until_terminator(input, b";{")?;
    let (input, body) = calc_def_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            CalcDef {
                identification: ident,
                body,
            },
        ),
    ))
}

fn calc_def_body(input: Input<'_>) -> IResult<Input<'_>, CalcDefBody> {
    let (mut input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b";") {
        let (input, _) = tag(&b";"[..]).parse(input)?;
        return Ok((input, CalcDefBody::Semicolon));
    }
    let (next, _) = tag(&b"{"[..]).parse(input)?;
    input = next;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().starts_with(b"}") {
            let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
            return Ok((input, CalcDefBody::Brace { elements }));
        }
        match calc_def_body_element(input) {
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
            Err(_) if starts_with_any_keyword(input.fragment(), CALC_DEF_BODY_STARTERS) => {
                let start_unknown = input;
                let (next, _) = recover_body_element(input, CALC_DEF_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                let recovery = build_recovery_error_node_from_span(
                    start_unknown,
                    next,
                    CALC_DEF_BODY_STARTERS,
                    "calc body",
                    "recovered_calc_body_element",
                );
                let node: Node<ParseErrorNode> = node_from_to(start_unknown, next, recovery);
                elements.push(node_from_to(
                    start_unknown,
                    next,
                    CalcDefBodyElement::Error(node),
                ));
                input = next;
            }
            Err(_) => {
                // Calc bodies in the standard library can contain additional constructs (e.g. `objective ... { ... }`)
                // that we don't model yet. Skip one statement/block and keep parsing later siblings.
                let start_unknown = input;
                let (next, _) = skip_statement_or_block(input)?;
                if next.location_offset() == start_unknown.location_offset() {
                    // Fallback: avoid infinite loop by bailing out to end of calc body.
                    let (input, _) = skip_until_brace_end(input)?;
                    let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
                    return Ok((input, CalcDefBody::Brace { elements }));
                }
                let frag = start_unknown.fragment();
                let take = frag.len().min(60);
                let preview = String::from_utf8_lossy(&frag[..take]).trim().to_string();
                elements.push(node_from_to(
                    start_unknown,
                    next,
                    CalcDefBodyElement::Other(preview),
                ));
                input = next;
            }
        }
    }
}

fn calc_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<CalcDefBodyElement>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, elem) = if starts_with_keyword(input.fragment(), b"doc") {
        map(
            crate::parser::requirement::doc_comment,
            CalcDefBodyElement::Doc,
        )
        .parse(input)?
    } else if starts_with_keyword(input.fragment(), b"in")
        || starts_with_keyword(input.fragment(), b"out")
        || starts_with_keyword(input.fragment(), b"inout")
    {
        if named_in_out_missing_type(input) {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }
        map(in_out_decl, CalcDefBodyElement::InOutDecl).parse(input)?
    } else if starts_with_keyword(input.fragment(), b"return") {
        if named_return_missing_type(input) {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }
        if let Ok((input, decl)) = return_decl(input) {
            (input, CalcDefBodyElement::ReturnDecl(decl))
        } else {
            other_calc_return(input)?
        }
    } else {
        map(expression, CalcDefBodyElement::Expression).parse(input)?
    };
    Ok((input, node_from_to(start, input, elem)))
}

fn named_in_out_missing_type(input: Input<'_>) -> bool {
    let Ok((after_ws, _)) = ws_and_comments(input) else {
        return false;
    };
    let direction_len = if starts_with_keyword(after_ws.fragment(), b"inout") {
        5
    } else if starts_with_keyword(after_ws.fragment(), b"out") {
        3
    } else {
        2
    };
    let mut rest = &after_ws.fragment()[direction_len..];
    while let Some(first) = rest.first() {
        if first.is_ascii_whitespace() {
            rest = &rest[1..];
        } else {
            break;
        }
    }
    if rest.starts_with(b":") {
        return false;
    }
    let mut name_len = 0usize;
    while name_len < rest.len()
        && (rest[name_len].is_ascii_alphanumeric() || rest[name_len] == b'_')
    {
        name_len += 1;
    }
    if name_len == 0 {
        return false;
    }
    rest = &rest[name_len..];
    while let Some(first) = rest.first() {
        if first.is_ascii_whitespace() {
            rest = &rest[1..];
        } else {
            break;
        }
    }
    if !rest.starts_with(b":") {
        return false;
    }
    rest = &rest[1..];
    while let Some(first) = rest.first() {
        if first.is_ascii_whitespace() {
            rest = &rest[1..];
        } else {
            break;
        }
    }
    rest.is_empty() || rest.starts_with(b";") || rest.starts_with(b"{") || rest.starts_with(b"}")
}

fn named_return_missing_type(input: Input<'_>) -> bool {
    let Ok((after_ws, _)) = ws_and_comments(input) else {
        return false;
    };
    let mut rest = *after_ws.fragment();
    if !starts_with_keyword(rest, b"return") {
        return false;
    }
    rest = &rest[6..];
    while let Some(first) = rest.first() {
        if first.is_ascii_whitespace() {
            rest = &rest[1..];
        } else {
            break;
        }
    }
    if rest.starts_with(b":") {
        return false;
    }
    let mut name_len = 0usize;
    while name_len < rest.len()
        && (rest[name_len].is_ascii_alphanumeric() || rest[name_len] == b'_')
    {
        name_len += 1;
    }
    if name_len == 0 {
        return false;
    }
    rest = &rest[name_len..];
    while let Some(first) = rest.first() {
        if first.is_ascii_whitespace() {
            rest = &rest[1..];
        } else {
            break;
        }
    }
    if rest.starts_with(b":") {
        rest = &rest[1..];
        while let Some(first) = rest.first() {
            if first.is_ascii_whitespace() {
                rest = &rest[1..];
            } else {
                break;
            }
        }
        return rest.is_empty()
            || rest.starts_with(b";")
            || rest.starts_with(b"{")
            || rest.starts_with(b"}");
    }
    false
}

fn other_calc_return(input: Input<'_>) -> IResult<Input<'_>, CalcDefBodyElement> {
    let start_unknown = input;
    let (next, _) = skip_statement_or_block(input)?;
    if next.location_offset() == start_unknown.location_offset() {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Many0,
        )));
    }
    let preview = String::from_utf8_lossy(
        &start_unknown.fragment()[..start_unknown.fragment().len().min(60)],
    )
    .trim()
    .to_string();
    Ok((next, CalcDefBodyElement::Other(preview)))
}

pub(crate) fn return_decl(input: Input<'_>) -> IResult<Input<'_>, Node<ReturnDecl>> {
    let start = input;
    let (input, _) = tag(&b"return"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, n) = name(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b":"[..])).parse(input)?;
    let (input, type_name) = preceded(ws_and_comments, qualified_name).parse(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((
        input,
        node_from_to(start, input, ReturnDecl { name: n, type_name }),
    ))
}
