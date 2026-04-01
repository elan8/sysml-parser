#![allow(dead_code, unused_imports)]

use crate::ast::{
    CalcDef, CalcDefBody, CalcDefBodyElement, ConstraintDef, ConstraintDefBody,
    ConstraintDefBodyElement, Node, ReturnDecl,
};
use crate::parser::action::in_out_decl;
use crate::parser::expr::expression;
use crate::parser::lex::{
    identification, name, qualified_name, recover_body_element, skip_until_brace_end,
    skip_statement_or_block, starts_with_any_keyword, take_until_terminator, ws1, ws_and_comments,
    CALC_DEF_BODY_STARTERS, CONSTRAINT_DEF_BODY_STARTERS,
};
use crate::parser::node_from_to;
use crate::parser::Input;
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
    let (mut input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b";") {
        let (input, _) = tag(&b";"[..]).parse(input)?;
        return Ok((input, ConstraintDefBody::Semicolon));
    }
    let (next, _) = tag(&b"{"[..]).parse(input)?;
    input = next;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().starts_with(b"}") {
            let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
            return Ok((input, ConstraintDefBody::Brace { elements }));
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
                let (next, _) = recover_body_element(input, CONSTRAINT_DEF_BODY_STARTERS)?;
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
                return Ok((input, ConstraintDefBody::Brace { elements }));
            }
        }
    }
}

fn constraint_def_body_element(
    input: Input<'_>,
) -> IResult<Input<'_>, Node<ConstraintDefBodyElement>> {
    let start = input;
    let (input, elem) = alt((
        map(
            crate::parser::requirement::doc_comment,
            ConstraintDefBodyElement::Doc,
        ),
        map(in_out_decl, ConstraintDefBodyElement::InOutDecl),
        map(expression, ConstraintDefBodyElement::Expression),
    ))
    .parse(input)?;
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
                let (next, _) = recover_body_element(input, CALC_DEF_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Error(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
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
    let (input, elem) = alt((
        map(
            crate::parser::requirement::doc_comment,
            CalcDefBodyElement::Doc,
        ),
        map(in_out_decl, CalcDefBodyElement::InOutDecl),
        map(return_decl, CalcDefBodyElement::ReturnDecl),
        map(expression, CalcDefBodyElement::Expression),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
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
