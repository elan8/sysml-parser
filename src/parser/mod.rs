//! Nom-based parser for SysML v2 textual notation.
//!
//! Organized into modules:
//! - [lex]: whitespace, comments, names, qualified names, skip helpers
//! - [attribute]: attribute definition and usage
//! - [import]: import and relationship body
//! - [part]: part definition and part usage
//! - [package]: package and root namespace

mod action;
mod alias;
mod attribute;
mod constraint;
mod expr;
mod import;
mod interface;
mod lex;
mod package;
mod part;
mod port;
mod requirement;
mod span;
mod state;
mod usecase;

pub(crate) use span::{node_from_to, Input};

use crate::ast::RootNamespace;
use crate::error::ParseError;
use nom::error::Error;
use nom_locate::LocatedSpan;

/// Result of parsing with error recovery: a (possibly partial) AST and zero or more diagnostics.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// Root namespace; contains all successfully parsed top-level elements (partial when errors occurred).
    pub root: RootNamespace,
    /// All parse errors encountered (multiple when recovery is used).
    pub errors: Vec<ParseError>,
}

impl ParseResult {
    /// True if the document parsed fully with no errors.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Map nom error kind to a human-readable message for language server diagnostics.
fn nom_error_kind_to_message(code: &nom::error::ErrorKind) -> &'static str {
    use nom::error::ErrorKind;
    match code {
        ErrorKind::Tag => "expected keyword or token",
        ErrorKind::Digit => "expected number",
        ErrorKind::Alpha => "expected identifier",
        ErrorKind::AlphaNumeric => "expected identifier",
        ErrorKind::Space => "expected whitespace",
        ErrorKind::MultiSpace => "expected whitespace",
        ErrorKind::Eof => "unexpected end of input",
        ErrorKind::TakeUntil => "expected terminator",
        ErrorKind::TakeWhile1 => "expected token",
        ErrorKind::Alt => "expected package, import, part, port, interface, alias, attribute, or action",
        ErrorKind::Many0 | ErrorKind::Many1 => "expected list of elements",
        _ => "parse error",
    }
}

fn nom_err_to_parse_error(e: &Error<Input<'_>>, length: Option<usize>) -> ParseError {
    let offset = e.input.location_offset();
    let line = e.input.location_line();
    let column = e.input.get_column();
    let message = nom_error_kind_to_message(&e.code).to_string();
    let mut pe = ParseError::new(message).with_location(offset, line, column);
    if let Some(len) = length {
        pe = pe.with_length(len);
    }
    pe
}

/// Parse full input; must consume entire input. Strips UTF-8 BOM if present.
pub fn parse_root(input: &str) -> Result<RootNamespace, ParseError> {
    let bytes = input
        .strip_prefix('\u{FEFF}')
        .map(str::as_bytes)
        .unwrap_or_else(|| input.as_bytes());
    let located = LocatedSpan::new(bytes);
    match package::root_namespace(located) {
        Ok((rest, root)) => {
            if rest.fragment().is_empty() {
                log::debug!("parse_root: success, {} top-level elements", root.elements.len());
                Ok(root)
            } else {
                let offset = located.location_offset() + located.fragment().len() - rest.fragment().len();
                let unconsumed = rest.fragment();
                let first_80 = unconsumed.get(..80.min(unconsumed.len())).unwrap_or(unconsumed);
                log::debug!(
                    "parse_root: expected end of input; parsed {} elements; unconsumed len={}, offset={}, first 80 bytes: {:?}",
                    root.elements.len(),
                    unconsumed.len(),
                    offset,
                    first_80,
                );
                log::debug!(
                    "parse_root: unconsumed as str: {:?}",
                    String::from_utf8_lossy(first_80),
                );
                Err(ParseError::new("expected end of input")
                    .with_location(offset, rest.location_line(), rest.get_column())
                    .with_code("expected_end_of_input"))
            }
        }
        Err(nom::Err::Error(e)) => {
            Err(nom_err_to_parse_error(&e, Some(1)).with_code("parse_error"))
        }
        Err(nom::Err::Failure(e)) => Err(nom_err_to_parse_error(&e, Some(1)).with_code("parse_error")),
        Err(nom::Err::Incomplete(_)) => Err(ParseError::new("unexpected end of input").with_code("unexpected_eof")),
    }
}

const MAX_RECOVERY_ERRORS: usize = 100;

/// Parse input with error recovery: collects multiple diagnostics and returns a partial AST when errors occur.
/// Use this for language servers so the user sees all parse errors and features (e.g. hover) can use the partial AST.
pub fn parse_with_diagnostics(input: &str) -> ParseResult {
    let bytes = input
        .strip_prefix('\u{FEFF}')
        .map(str::as_bytes)
        .unwrap_or_else(|| input.as_bytes());
    let located = LocatedSpan::new(bytes);

    let mut elements = Vec::new();
    let mut errors = Vec::new();

    let (mut input, _) = match lex::ws_and_comments(located) {
        Ok(x) => x,
        Err(_) => {
            return ParseResult {
                root: RootNamespace { elements: vec![] },
                errors: vec![ParseError::new("invalid input").with_code("invalid_input")],
            };
        }
    };

    while !input.fragment().is_empty() && errors.len() < MAX_RECOVERY_ERRORS {
        match package::package_body_element(input) {
            Ok((rest, elem)) => {
                elements.push(elem);
                input = rest;
            }
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                let pe = nom_err_to_parse_error(&e, Some(1)).with_code("parse_error");
                errors.push(pe);
                match lex::skip_to_next_sync_point(e.input) {
                    Ok((rest, _)) => input = rest,
                    Err(_) => break,
                }
            }
            Err(nom::Err::Incomplete(_)) => {
                errors.push(
                    ParseError::new("unexpected end of input")
                        .with_location(input.location_offset(), input.location_line(), input.get_column())
                        .with_length(1)
                        .with_code("unexpected_eof"),
                );
                break;
            }
        }
    }

    let (input, _) = lex::ws_and_comments(input).unwrap_or((input, ()));

    if !input.fragment().is_empty() {
        errors.push(
            ParseError::new("expected end of input")
                .with_location(input.location_offset(), input.location_line(), input.get_column())
                .with_length(input.fragment().len().min(1))
                .with_code("expected_end_of_input"),
        );
    }

    ParseResult {
        root: RootNamespace { elements },
        errors,
    }
}
