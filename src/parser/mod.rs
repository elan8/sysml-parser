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
mod item;
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
mod view;

pub(crate) use span::{node_from_to, span_from_to, with_span, Input};

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

const FOUND_SNIPPET_MAX_LEN: usize = 40;

/// Take a short snippet from the input at the error position for "found" display.
/// Uses first line or first FOUND_SNIPPET_MAX_LEN bytes, UTF-8 with replacement char.
fn fragment_to_found_snippet(fragment: &[u8]) -> (String, usize) {
    let take = fragment
        .iter()
        .position(|&b| b == b'\n' || b == b'\r')
        .map(|p| p.min(FOUND_SNIPPET_MAX_LEN))
        .unwrap_or_else(|| fragment.len().min(FOUND_SNIPPET_MAX_LEN));
    let slice = fragment.get(..take).unwrap_or(fragment);
    let s = String::from_utf8_lossy(slice)
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    let len = slice.len();
    (s.trim_end().to_string(), len)
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

/// Map nom error kind to a specific code for LSP/quick fixes.
fn nom_error_kind_to_code(code: &nom::error::ErrorKind) -> &'static str {
    use nom::error::ErrorKind;
    match code {
        ErrorKind::Tag => "expected_keyword",
        ErrorKind::Digit => "expected_number",
        ErrorKind::Alpha | ErrorKind::AlphaNumeric => "expected_identifier",
        ErrorKind::Space | ErrorKind::MultiSpace => "expected_whitespace",
        ErrorKind::Eof => "unexpected_eof",
        ErrorKind::TakeUntil => "expected_terminator",
        ErrorKind::TakeWhile1 => "expected_token",
        ErrorKind::Alt => "expected_alt",
        ErrorKind::Many0 | ErrorKind::Many1 => "expected_list",
        _ => "parse_error",
    }
}

fn nom_err_to_parse_error(
    e: &Error<Input<'_>>,
    length_override: Option<usize>,
    expected_context: Option<&'static str>,
) -> ParseError {
    let offset = e.input.location_offset();
    let line = e.input.location_line();
    let column = e.input.get_column();
    let fragment = e.input.fragment();
    let (found_snippet, found_len) = fragment_to_found_snippet(fragment);
    let message = nom_error_kind_to_message(&e.code).to_string();
    let span_len = length_override.unwrap_or(found_len).max(1);
    let mut pe = ParseError::new(message)
        .with_location(offset, line, column)
        .with_length(span_len)
        .with_code(nom_error_kind_to_code(&e.code));
    if !found_snippet.is_empty() {
        pe = pe.with_found(found_snippet);
    }
    if let Some(ctx) = expected_context {
        pe = pe.with_expected(ctx);
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
                let (found_snippet, found_len) = fragment_to_found_snippet(rest.fragment());
                let mut pe = ParseError::new("expected end of input")
                    .with_location(offset, rest.location_line(), rest.get_column())
                    .with_length(found_len.max(1))
                    .with_code("expected_end_of_input");
                if !found_snippet.is_empty() {
                    pe = pe.with_found(found_snippet);
                }
                Err(pe)
            }
        }
        Err(nom::Err::Error(e)) => Err(nom_err_to_parse_error(
            &e,
            None,
            Some("'package', 'namespace', or 'import' at top level; or valid element in package body"),
        )),
        Err(nom::Err::Failure(e)) => Err(nom_err_to_parse_error(
            &e,
            None,
            Some("'package', 'namespace', or 'import' at top level; or valid element in package body"),
        )),
        Err(nom::Err::Incomplete(_)) => Err(ParseError::new("unexpected end of input").with_code("unexpected_eof")),
    }
}

const MAX_RECOVERY_ERRORS: usize = 100;

/// When inside a package, we only report errors that look like invalid top-level elements:
/// e.g. `test {}`, `test2 {}`, `xyz {}`. We skip reporting when the found snippet looks
/// like valid nested content (e.g. `transition ...`, `totalThrust >= ...`, `}`, `//`).
fn should_report_error_inside_package(found: &str) -> bool {
    let trimmed = found.trim_start();
    // Valid package body starters - don't report
    if trimmed.starts_with('}')
        || trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with("//*")
        || trimmed.starts_with("doc ")
        || trimmed.starts_with("package ")
        || trimmed.starts_with("part ")
        || trimmed.starts_with("port ")
        || trimmed.starts_with("attribute ")
        || trimmed.starts_with("action ")
        || trimmed.starts_with("requirement ")
        || trimmed.starts_with("item ")
        || trimmed.starts_with("use ")
        || trimmed.starts_with("state ")
        || trimmed.starts_with("constraint ")
        || trimmed.starts_with("calc ")
        || trimmed.starts_with("connect ")
        || trimmed.starts_with("allocate ")
        || trimmed.starts_with("filter ")
        || trimmed.starts_with("abstract ")
        || trimmed.starts_with("private ")
        || trimmed.starts_with("protected ")
        || trimmed.starts_with("public ")
        || trimmed.starts_with("in ")
        || trimmed.starts_with("out ")
        || trimmed.starts_with("return ")
        || trimmed.starts_with("subject ")
        || trimmed.starts_with("actor ")
        || trimmed.starts_with("require ")
        || trimmed.starts_with("perform ")
        || trimmed.starts_with("objective ")
        || trimmed.starts_with("transition ")
        || trimmed.starts_with("view ")
        || trimmed.starts_with("viewpoint ")
        || trimmed.starts_with("rendering ")
        || trimmed.starts_with("render ")
    {
        return false;
    }
    // Nested content (state machine, constraint body, etc.) - don't report
    if trimmed.contains(" >= ")
        || trimmed.contains(" == ")
        || trimmed.contains(" then ")
        || trimmed.contains(" first ")
        || trimmed.contains(" / ")
        || trimmed.contains(" * ")
        || trimmed.contains(" + ")
        || trimmed.contains(" - ")
    {
        return false;
    }
    // Likely invalid top-level: identifier followed by {} (e.g. test {}, test2 {}, xyz {})
    trimmed.contains(" {}")
}

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

    while errors.len() < MAX_RECOVERY_ERRORS {
        // Skip leading ws/comments; if nothing left, we're done (avoids parsing "" as root_element).
        let (rest, _) = lex::ws_and_comments(input).unwrap_or((input, ()));
        input = rest;
        if input.fragment().is_empty() {
            break;
        }
        match package::root_element(input) {
            Ok((rest, elem)) => {
                elements.push(elem);
                input = rest;
            }
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                let pe = nom_err_to_parse_error(&e, None, Some("'package', 'namespace', or 'import'"));
                let consumed = &bytes[..e.input.location_offset()];
                let depth = consumed.iter().filter(|&&b| b == b'{').count()
                    - consumed.iter().filter(|&&b| b == b'}').count();
                // When inside a package, only add errors for invalid identifiers (e.g. "test", "test2"),
                // not for valid syntax we hit while skipping (e.g. "}", "//", "part def").
                let is_inside_package = depth > 0;
                let should_report = !is_inside_package || should_report_error_inside_package(pe.found.as_deref().unwrap_or(""));
                if should_report {
                    errors.push(pe);
                }
                let skip_result = lex::skip_to_next_sync_point(e.input);
                match skip_result {
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
        let (found_snippet, found_len) = fragment_to_found_snippet(input.fragment());
        let mut pe = ParseError::new("expected end of input")
            .with_location(input.location_offset(), input.location_line(), input.get_column())
            .with_length(found_len.max(1))
            .with_code("expected_end_of_input");
        if !found_snippet.is_empty() {
            pe = pe.with_found(found_snippet);
        }
        errors.push(pe);
    }

    ParseResult {
        root: RootNamespace { elements },
        errors,
    }
}
