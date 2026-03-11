//! Nom-based parser for SysML v2 textual notation.

use crate::ast::{Identification, Package, PackageBody, PackageBodyElement, RootNamespace};
use crate::error::ParseError;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while, take_while1};
use nom::combinator::{map, opt};
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
use nom::IResult;

/// Skip optional whitespace (space, tab, newline).
fn ws(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = take_while(|c: u8| c == b' ' || c == b'\t' || c == b'\n' || c == b'\r')(input)?;
    Ok((input, ()))
}

/// Parse one or more whitespace characters (consumes at least one).
fn ws1(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = take_while1(|c: u8| c == b' ' || c == b'\t' || c == b'\n' || c == b'\r')(input)?;
    Ok((input, ()))
}

/// NAME: identifier (alphanumeric or underscore). Simplified for initial tests.
fn name(input: &[u8]) -> IResult<&[u8], String> {
    let (input, raw) = take_while1(|c: u8| c.is_ascii_alphanumeric() || c == b'_')(input)?;
    let s = String::from_utf8_lossy(raw).into_owned();
    Ok((input, s))
}

/// Keyword "package" with following whitespace.
fn keyword_package(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = tag("package")(input)?;
    let (input, _) = ws1(input)?;
    Ok((input, ()))
}

/// Identification: ( '<' ShortName '>' )? ( Name )?
fn identification(input: &[u8]) -> IResult<&[u8], Identification> {
    let (input, short_name) = opt(delimited(
        preceded(ws, tag("<")),
        preceded(ws, name),
        preceded(ws, tag(">")),
    ))(input)?;
    let (input, decl_name) = opt(preceded(ws, name))(input)?;
    Ok((
        input,
        Identification {
            short_name,
            name: decl_name,
        },
    ))
}

/// PackageBody: ';' | '{' PackageBodyElement* '}'
fn package_body(input: &[u8]) -> IResult<&[u8], PackageBody> {
    alt((
        map(preceded(ws, tag(";")), |_| PackageBody::Semicolon),
        map(
            delimited(
                preceded(ws, tag("{")),
                preceded(ws, many0(preceded(ws, package_body_element))),
                preceded(ws, tag("}")),
            ),
            |elements| PackageBody::Brace { elements },
        ),
    ))(input)
}

/// PackageBodyElement for now: only Package (no imports, filters, etc. yet).
fn package_body_element(input: &[u8]) -> IResult<&[u8], PackageBodyElement> {
    map(package_, PackageBodyElement::Package)(input)
}

/// package Identification PackageBody
fn package_(input: &[u8]) -> IResult<&[u8], Package> {
    let (input, _) = keyword_package(input)?;
    let (input, identification) = identification(input)?;
    let (input, body) = package_body(input)?;
    Ok((
        input,
        Package {
            identification,
            body,
        },
    ))
}

/// Root: PackageBodyElement*
fn root_namespace(input: &[u8]) -> IResult<&[u8], RootNamespace> {
    let (input, elements) = many0(preceded(ws, package_body_element))(input)?;
    let (input, _) = ws(input)?;
    Ok((input, RootNamespace { elements }))
}

/// Parse full input; must consume entire input.
pub fn parse_root(input: &str) -> Result<RootNamespace, ParseError> {
    let bytes = input.as_bytes();
    match root_namespace(bytes) {
        Ok((rest, root)) => {
            if rest.is_empty() {
                Ok(root)
            } else {
                let offset = bytes.len() - rest.len();
                Err(ParseError::new("expected end of input").with_offset(offset))
            }
        }
        Err(nom::Err::Error(e)) => {
            let offset = bytes.len() - e.input.len();
            Err(ParseError::new(format!("parse error: {:?}", e.code)).with_offset(offset))
        }
        Err(nom::Err::Failure(e)) => {
            let offset = bytes.len() - e.input.len();
            Err(ParseError::new(format!("parse error: {:?}", e.code)).with_offset(offset))
        }
        Err(nom::Err::Incomplete(_)) => Err(ParseError::new("unexpected end of input")),
    }
}
