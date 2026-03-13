use crate::ast::{
    AnalysisCaseDef, AnalysisCaseUsage, CaseDef, CaseUsage, Node,
    VerificationCaseDef, VerificationCaseUsage,
};
use crate::parser::lex::{identification, name, qualified_name, ws1, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::usecase::use_case_def_body;
use crate::parser::Input;
use nom::bytes::complete::tag;
use nom::combinator::opt;
use nom::sequence::preceded;
use nom::Parser;
use nom::IResult;

pub(crate) fn case_def(input: Input<'_>) -> IResult<Input<'_>, Node<CaseDef>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"case"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, body) = use_case_def_body(input)?;
    Ok((input, node_from_to(start, input, CaseDef { identification, body })))
}

pub(crate) fn case_usage(input: Input<'_>) -> IResult<Input<'_>, Node<CaseUsage>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"case"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, usage) = case_like_usage_body(input)?;
    Ok((input, node_from_to(start, input, usage)))
}

pub(crate) fn analysis_case_def(input: Input<'_>) -> IResult<Input<'_>, Node<AnalysisCaseDef>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"analysis"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, body) = use_case_def_body(input)?;
    Ok((input, node_from_to(start, input, AnalysisCaseDef { identification, body })))
}

pub(crate) fn analysis_case_usage(input: Input<'_>) -> IResult<Input<'_>, Node<AnalysisCaseUsage>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"analysis"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, usage) = case_like_usage_body(input)?;
    Ok((input, node_from_to(start, input, AnalysisCaseUsage { name: usage.name, type_name: usage.type_name, body: usage.body })))
}

pub(crate) fn verification_case_def(input: Input<'_>) -> IResult<Input<'_>, Node<VerificationCaseDef>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"verification"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag(&b"def"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, body) = use_case_def_body(input)?;
    Ok((input, node_from_to(start, input, VerificationCaseDef { identification, body })))
}

pub(crate) fn verification_case_usage(input: Input<'_>) -> IResult<Input<'_>, Node<VerificationCaseUsage>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"verification"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, usage) = case_like_usage_body(input)?;
    Ok((input, node_from_to(start, input, VerificationCaseUsage { name: usage.name, type_name: usage.type_name, body: usage.body })))
}

fn case_like_usage_body(input: Input<'_>) -> IResult<Input<'_>, CaseUsage> {
    let (input, name) = name(input)?;
    let (input, type_name) = opt(preceded(
        preceded(ws_and_comments, tag(&b":"[..])),
        preceded(ws_and_comments, qualified_name),
    ))
    .parse(input)?;
    let (input, body) = use_case_def_body(input)?;
    Ok((input, CaseUsage { name, type_name, body }))
}
