#![allow(dead_code, unused_imports)]

use crate::ast::{
    Node, ConstraintDef, ConstraintDefBody, ConstraintDefBodyElement,
    CalcDef, CalcDefBody, CalcDefBodyElement, ReturnDecl
};
use crate::parser::action::in_out_decl;
use crate::parser::expr::expression;
use crate::parser::lex::{
    identification, name, qualified_name, skip_until_brace_end, take_until_terminator, ws1,
    ws_and_comments,
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
    Ok((input, node_from_to(start, input, ConstraintDef { identification: ident, body })))
}

fn constraint_def_body(input: Input<'_>) -> IResult<Input<'_>, ConstraintDefBody> {
    let (input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b";") {
        let (input, _) = tag(&b";"[..]).parse(input)?;
        return Ok((input, ConstraintDefBody::Semicolon));
    }
    let (input, _) = tag(&b"{"[..]).parse(input)?;
    let (input, _) = skip_until_brace_end(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
    Ok((input, ConstraintDefBody::Brace { elements: vec![] }))
}

fn constraint_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<ConstraintDefBodyElement>> {
    let start = input;
    let (input, elem) = alt((
        map(crate::parser::requirement::doc_comment, ConstraintDefBodyElement::Doc),
        map(in_out_decl, ConstraintDefBodyElement::InOutDecl),
        map(expression, ConstraintDefBodyElement::Expression),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

fn safe_constraint_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<ConstraintDefBodyElement>> {
    let start = input;
    let mut parser = alt((
        map(in_out_decl, |n| node_from_to(start, input, ConstraintDefBodyElement::InOutDecl(n))),
        map(expression, |n| node_from_to(start, input, ConstraintDefBodyElement::Expression(n))),
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
    Ok((input, node_from_to(start, input, CalcDef { identification: ident, body })))
}

fn calc_def_body(input: Input<'_>) -> IResult<Input<'_>, CalcDefBody> {
    let (input, _) = ws_and_comments(input)?;
    if input.fragment().starts_with(b";") {
        let (input, _) = tag(&b";"[..]).parse(input)?;
        return Ok((input, CalcDefBody::Semicolon));
    }
    let (input, _) = tag(&b"{"[..]).parse(input)?;
    let (input, _) = skip_until_brace_end(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
    Ok((input, CalcDefBody::Brace { elements: vec![] }))
}

fn calc_def_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<CalcDefBodyElement>> {
    let start = input;
    let (input, elem) = alt((
        map(crate::parser::requirement::doc_comment, CalcDefBodyElement::Doc),
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
    Ok((input, node_from_to(start, input, ReturnDecl { name: n, type_name })))
}
