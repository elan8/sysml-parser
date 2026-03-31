//! Nom-based parser for SysML v2 textual notation.
//!
//! Organized into modules:
//! - [lex]: whitespace, comments, names, qualified names, skip helpers
//! - [attribute]: attribute definition and usage
//! - [import]: import and relationship body
//! - [part]: part definition and part usage
//! - [package]: package and root namespace

mod action;
mod allocation;
mod alias;
mod attribute;
mod case;
mod connection;
mod constraint;
mod dependency;
mod individual;
mod item;
mod enumeration;
mod expr;
mod flow;
mod import;
mod interface;
mod lex;
mod metadata;
mod metadata_annotation;
mod occurrence;
mod package;
mod part;
mod port;
mod requirement;
mod span;
mod state;
mod usecase;
mod view;

pub(crate) use span::{node_from_to, span_from_to, with_span, Input};

use crate::ast::{
    ActionDefBody, ActionDefBodyElement, ActionUsageBody, ActionUsageBodyElement, PackageBody,
    PackageBodyElement, ParseErrorNode, PartDefBody, PartDefBodyElement, PartUsageBody,
    PartUsageBodyElement, RequirementDefBody, RequirementDefBodyElement, RootNamespace,
    StateDefBody, StateDefBodyElement, UseCaseDefBody, UseCaseDefBodyElement,
};
use crate::error::ParseError;
use nom::error::Error;
use nom::Parser;
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
const ILLEGAL_TOP_LEVEL_STARTERS: &[&[u8]] = &[
    b"action",
    b"actor",
    b"alias",
    b"allocate",
    b"allocation",
    b"attribute",
    b"bind",
    b"calc",
    b"case",
    b"concern",
    b"connection",
    b"constraint",
    b"dependency",
    b"enum",
    b"flow",
    b"interface",
    b"item",
    b"metadata",
    b"occurrence",
    b"part",
    b"perform",
    b"port",
    b"ref",
    b"require",
    b"requirement",
    b"satisfy",
    b"state",
    b"use",
    b"verification",
    b"view",
    b"viewpoint",
];

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

pub(crate) fn recovery_found_snippet(input: Input<'_>) -> Option<String> {
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
    let at_root = expected_context.is_some_and(|ctx| {
        ctx.contains("'package', 'namespace', or 'import'") || ctx.contains("top level")
    });
    if at_root && is_illegal_top_level_definition(fragment) {
        pe.message = "illegal top-level definition".to_string();
        pe.code = Some("illegal_top_level_definition".to_string());
        pe.expected = Some("'package', 'namespace', or 'import'".to_string());
        pe.suggestion = Some(
            "Wrap this declaration in `package ... { ... }` or `namespace ... { ... }`."
                .to_string(),
        );
    }
    pe
}

fn is_illegal_top_level_definition(fragment: &[u8]) -> bool {
    let trimmed = trim_ascii_start(fragment);
    !trimmed.starts_with(b"}")
        && !trimmed.starts_with(b"//")
        && !trimmed.starts_with(b"/*")
        && lex::starts_with_any_keyword(trimmed, ILLEGAL_TOP_LEVEL_STARTERS)
}

fn trim_ascii_start(mut fragment: &[u8]) -> &[u8] {
    while let Some(first) = fragment.first() {
        if first.is_ascii_whitespace() {
            fragment = &fragment[1..];
            continue;
        }
        break;
    }
    fragment
}

fn starts_with_missing_name_after_keyword(
    fragment: &[u8],
    keyword: &[u8],
    trailing_keywords: &[&[u8]],
) -> bool {
    let mut fragment = trim_ascii_start(fragment);
    if !lex::starts_with_keyword(fragment, keyword) {
        return false;
    }
    fragment = &fragment[keyword.len()..];
    while let Some(first) = fragment.first() {
        if first.is_ascii_whitespace() {
            fragment = &fragment[1..];
            continue;
        }
        break;
    }
    for trailing in trailing_keywords {
        if lex::starts_with_keyword(fragment, trailing) {
            fragment = &fragment[trailing.len()..];
            while let Some(first) = fragment.first() {
                if first.is_ascii_whitespace() {
                    fragment = &fragment[1..];
                    continue;
                }
                break;
            }
        }
    }
    fragment.starts_with(b":")
}

fn starts_with_missing_type_after_keyword(
    fragment: &[u8],
    keyword: &[u8],
    trailing_keywords: &[&[u8]],
) -> bool {
    let mut fragment = trim_ascii_start(fragment);
    if !lex::starts_with_keyword(fragment, keyword) {
        return false;
    }
    fragment = &fragment[keyword.len()..];
    while let Some(first) = fragment.first() {
        if first.is_ascii_whitespace() {
            fragment = &fragment[1..];
            continue;
        }
        break;
    }
    for trailing in trailing_keywords {
        if lex::starts_with_keyword(fragment, trailing) {
            fragment = &fragment[trailing.len()..];
            while let Some(first) = fragment.first() {
                if first.is_ascii_whitespace() {
                    fragment = &fragment[1..];
                    continue;
                }
                break;
            }
        }
    }

    let mut name_len = 0usize;
    while name_len < fragment.len()
        && (fragment[name_len].is_ascii_alphanumeric() || fragment[name_len] == b'_')
    {
        name_len += 1;
    }
    if name_len == 0 {
        return false;
    }
    fragment = &fragment[name_len..];
    while let Some(first) = fragment.first() {
        if first.is_ascii_whitespace() {
            fragment = &fragment[1..];
            continue;
        }
        break;
    }
    if !fragment.starts_with(b":") {
        return false;
    }
    fragment = &fragment[1..];
    while let Some(first) = fragment.first() {
        if first.is_ascii_whitespace() {
            fragment = &fragment[1..];
            continue;
        }
        break;
    }

    fragment.is_empty()
        || fragment.starts_with(b";")
        || fragment.starts_with(b"{")
        || fragment.starts_with(b"}")
        || lex::starts_with_keyword(fragment, b"then")
        || lex::starts_with_keyword(fragment, b"if")
        || lex::starts_with_keyword(fragment, b"do")
}

fn missing_name_diagnostic(fragment: &[u8]) -> Option<(&'static str, String, String, String)> {
    let cases: &[(&[u8], &[&[u8]], &str, &str)] = &[
        (b"subject", &[], "subject name", "Use `subject laptop: Laptop;`."),
        (b"actor", &[], "actor name", "Use `actor user: User;`."),
        (b"state", &[], "state name", "Use `state ready: Mode;`."),
        (b"part", &[], "part name", "Use `part wheel: Wheel;`."),
        (b"ref", &[], "reference name", "Use `ref sensor: Sensor;`."),
        (b"port", &[], "port name", "Use `port power: PowerPort;`."),
        (b"attribute", &[], "attribute name", "Use `attribute mass: MassValue;`."),
        (b"in", &[], "input name", "Use `in speed: Real;`."),
        (b"out", &[], "output name", "Use `out result: Real;`."),
        (
            b"perform",
            &[b"action"],
            "action name",
            "Use `perform action run: Runner;`.",
        ),
    ];

    for (keyword, trailing, missing_what, suggestion) in cases {
        if starts_with_missing_name_after_keyword(fragment, keyword, trailing) {
            return Some((
                "missing_member_name",
                format!("expected {missing_what} before ':'"),
                format!("{missing_what} before ':'"),
                format!("{suggestion}"),
            ));
        }
    }
    None
}

fn missing_type_diagnostic(fragment: &[u8]) -> Option<(&'static str, String, String, String)> {
    let cases: &[(&[u8], &[&[u8]], &str)] = &[
        (b"subject", &[], "subject type"),
        (b"actor", &[], "actor type"),
        (b"state", &[], "state type"),
        (b"part", &[], "part type"),
        (b"ref", &[], "reference type"),
        (b"port", &[], "port type"),
        (b"attribute", &[], "attribute type"),
        (b"in", &[], "input type"),
        (b"out", &[], "output type"),
        (b"perform", &[b"action"], "action type"),
    ];

    for &(keyword, trailing, missing_what) in cases {
        if starts_with_missing_type_after_keyword(fragment, keyword, trailing) {
            let keyword_label = String::from_utf8_lossy(keyword);
            let sample_name = if keyword == &b"subject"[..] {
                "laptop"
            } else if keyword == &b"actor"[..] {
                "user"
            } else if keyword == &b"state"[..] {
                "ready"
            } else if keyword == &b"part"[..] {
                "wheel"
            } else if keyword == &b"ref"[..] {
                "sensor"
            } else if keyword == &b"port"[..] {
                "power"
            } else if keyword == &b"attribute"[..] {
                "mass"
            } else if keyword == &b"in"[..] {
                "speed"
            } else if keyword == &b"out"[..] {
                "result"
            } else if keyword == &b"perform"[..] {
                "run"
            } else {
                "member"
            };
            let sample_type = if keyword == &b"subject"[..] {
                "Laptop"
            } else if keyword == &b"actor"[..] {
                "User"
            } else if keyword == &b"state"[..] {
                "Mode"
            } else if keyword == &b"part"[..] {
                "Wheel"
            } else if keyword == &b"ref"[..] {
                "Sensor"
            } else if keyword == &b"port"[..] {
                "PowerPort"
            } else if keyword == &b"attribute"[..] {
                "MassValue"
            } else if keyword == &b"in"[..] || keyword == &b"out"[..] {
                "Real"
            } else if keyword == &b"perform"[..] {
                "Runner"
            } else {
                "Type"
            };
            let suggestion = if keyword == &b"perform"[..] {
                format!("Use `perform action {sample_name}: {sample_type};`.")
            } else {
                format!("Use `{keyword_label} {sample_name}: {sample_type};`.")
            };
            return Some((
                "missing_type_reference",
                format!("expected {missing_what} after ':'"),
                format!("{missing_what} after ':'"),
                suggestion,
            ));
        }
    }
    None
}

fn missing_closing_brace_error(bytes: &[u8], input: Input<'_>) -> Option<ParseError> {
    if !input.fragment().is_empty() {
        return None;
    }
    let consumed = &bytes[..input.location_offset().min(bytes.len())];
    let opens = consumed.iter().filter(|&&b| b == b'{').count();
    let closes = consumed.iter().filter(|&&b| b == b'}').count();
    if opens <= closes {
        return None;
    }
    Some(missing_closing_brace_error_at_eof(consumed))
}

fn missing_closing_brace_error_at_eof(bytes: &[u8]) -> ParseError {
    let (line, column) = eof_line_column(bytes);
    ParseError::new("missing closing '}'")
        .with_location(bytes.len(), line, column)
        .with_length(1)
        .with_code("missing_closing_brace")
        .with_expected("'}'")
        .with_suggestion("Add '}' to close the open body.")
}

fn has_unclosed_brace(bytes: &[u8]) -> bool {
    let opens = bytes.iter().filter(|&&b| b == b'{').count();
    let closes = bytes.iter().filter(|&&b| b == b'}').count();
    opens > closes
}

fn eof_line_column(bytes: &[u8]) -> (u32, usize) {
    let mut line = 1u32;
    let mut column = 1usize;
    for &b in bytes {
        if b == b'\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

pub(crate) fn build_recovery_error_node(
    input: Input<'_>,
    starters: &[&[u8]],
    scope_label: &str,
    generic_code: &str,
) -> ParseErrorNode {
    let trimmed = trim_ascii_start(input.fragment());

    if let Some((code, message, expected, suggestion)) = missing_name_diagnostic(trimmed) {
        return ParseErrorNode {
            message,
            code: code.to_string(),
            expected: Some(expected),
            found: recovery_found_snippet(input),
            suggestion: Some(suggestion),
        };
    }

    if let Some((code, message, expected, suggestion)) = missing_type_diagnostic(trimmed) {
        return ParseErrorNode {
            message,
            code: code.to_string(),
            expected: Some(expected),
            found: recovery_found_snippet(input),
            suggestion: Some(suggestion),
        };
    }

    if lex::looks_like_missing_semicolon(input, starters) {
        return ParseErrorNode {
            message: "missing semicolon before next declaration".to_string(),
            code: "missing_semicolon".to_string(),
            expected: Some("';'".to_string()),
            found: recovery_found_snippet(input),
            suggestion: Some("Insert ';' before this declaration.".to_string()),
        };
    }

    if lex::starts_with_keyword(trimmed, b"#") || lex::starts_with_keyword(trimmed, b"@") {
        return ParseErrorNode {
            message: format!("unsupported annotation syntax in {scope_label}"),
            code: generic_code.to_string(),
            expected: Some(format!("valid {scope_label} element")),
            found: recovery_found_snippet(input),
            suggestion: Some(
                "Remove this annotation or extend the parser to support annotated declarations."
                    .to_string(),
            ),
        };
    }

    ParseErrorNode {
        message: format!("recovered {scope_label} element"),
        code: generic_code.to_string(),
        expected: Some(format!("valid {scope_label} element")),
        found: recovery_found_snippet(input),
        suggestion: Some(format!(
            "Fix this {scope_label} member and re-run parsing."
        )),
    }
}

fn is_only_trailing_closing_braces(mut input: Input<'_>) -> bool {
    loop {
        let (next, _) = lex::ws_and_comments(input).unwrap_or((input, ()));
        input = next;
        if input.fragment().is_empty() {
            return true;
        }
        if input.fragment().starts_with(b"}") {
            match nom::bytes::complete::tag::<_, _, nom::error::Error<Input>>(&b"}"[..]).parse(input)
            {
                Ok((next, _)) => {
                    input = next;
                    continue;
                }
                Err(_) => return false,
            }
        }
        return false;
    }
}

fn parse_error_from_recovery_node(
    span: &crate::ast::Span,
    node: &ParseErrorNode,
) -> ParseError {
    let mut err = ParseError::new(node.message.clone())
        .with_location(span.offset, span.line, span.column)
        .with_length(span.len.max(1))
        .with_code(node.code.clone());
    if let Some(expected) = &node.expected {
        err = err.with_expected(expected.clone());
    }
    if let Some(found) = &node.found {
        err = err.with_found(found.clone());
    }
    if let Some(suggestion) = &node.suggestion {
        err = err.with_suggestion(suggestion.clone());
    }
    err
}

fn collect_requirement_body_errors(body: &RequirementDefBody, errors: &mut Vec<ParseError>) {
    if let RequirementDefBody::Brace { elements } = body {
        for element in elements {
            match &element.value {
                RequirementDefBodyElement::Error(n) => {
                    errors.push(parse_error_from_recovery_node(&element.span, &n.value));
                }
                RequirementDefBodyElement::Frame(n) => {
                    collect_requirement_body_errors(&n.value.body, errors)
                }
                _ => {}
            }
        }
    }
}

fn collect_action_def_body_errors(body: &ActionDefBody, errors: &mut Vec<ParseError>) {
    if let ActionDefBody::Brace { elements } = body {
        for element in elements {
            if let ActionDefBodyElement::Error(n) = &element.value {
                errors.push(parse_error_from_recovery_node(&element.span, &n.value));
            }
        }
    }
}

fn collect_action_usage_body_errors(body: &ActionUsageBody, errors: &mut Vec<ParseError>) {
    if let ActionUsageBody::Brace { elements } = body {
        for element in elements {
            match &element.value {
                ActionUsageBodyElement::Error(n) => {
                    errors.push(parse_error_from_recovery_node(&element.span, &n.value));
                }
                ActionUsageBodyElement::ActionUsage(n) => {
                    collect_action_usage_body_errors(&n.value.body, errors)
                }
                _ => {}
            }
        }
    }
}

fn collect_state_body_errors(body: &StateDefBody, errors: &mut Vec<ParseError>) {
    if let StateDefBody::Brace { elements } = body {
        for element in elements {
            match &element.value {
                StateDefBodyElement::Error(n) => {
                    errors.push(parse_error_from_recovery_node(&element.span, &n.value));
                }
                StateDefBodyElement::Entry(n) => collect_state_body_errors(&n.value.body, errors),
                StateDefBodyElement::StateUsage(n) => {
                    collect_state_body_errors(&n.value.body, errors)
                }
                _ => {}
            }
        }
    }
}

fn collect_use_case_body_errors(body: &UseCaseDefBody, errors: &mut Vec<ParseError>) {
    if let UseCaseDefBody::Brace { elements } = body {
        for element in elements {
            if let UseCaseDefBodyElement::Error(n) = &element.value {
                errors.push(parse_error_from_recovery_node(&element.span, &n.value));
            }
        }
    }
}

fn collect_part_def_body_errors(body: &PartDefBody, errors: &mut Vec<ParseError>) {
    if let PartDefBody::Brace { elements } = body {
        for element in elements {
            match &element.value {
                PartDefBodyElement::Error(n) => {
                    errors.push(parse_error_from_recovery_node(&element.span, &n.value));
                }
                PartDefBodyElement::PartUsage(n) => collect_part_usage_body_errors(&n.value.body, errors),
                PartDefBodyElement::Perform(n) => collect_perform_body_errors(&n.value.body, errors),
                _ => {}
            }
        }
    }
}

fn collect_perform_body_errors(body: &crate::ast::PerformBody, _errors: &mut Vec<ParseError>) {
    match body {
        crate::ast::PerformBody::Semicolon => {}
        crate::ast::PerformBody::Brace { .. } => {}
    }
}

fn collect_part_usage_body_errors(body: &PartUsageBody, errors: &mut Vec<ParseError>) {
    if let PartUsageBody::Brace { elements } = body {
        for element in elements {
            match &element.value {
                PartUsageBodyElement::Error(n) => {
                    errors.push(parse_error_from_recovery_node(&element.span, &n.value));
                }
                PartUsageBodyElement::PartUsage(n) => collect_part_usage_body_errors(&n.value.body, errors),
                PartUsageBodyElement::Perform(n) => collect_perform_body_errors(&n.value.body, errors),
                PartUsageBodyElement::StateUsage(n) => collect_state_body_errors(&n.value.body, errors),
                _ => {}
            }
        }
    }
}

fn collect_package_body_errors(body: &PackageBody, errors: &mut Vec<ParseError>) {
    if let PackageBody::Brace { elements } = body {
        for element in elements {
            match &element.value {
                PackageBodyElement::Error(n) => {
                    errors.push(parse_error_from_recovery_node(&element.span, &n.value));
                }
                PackageBodyElement::Package(n) => collect_package_body_errors(&n.value.body, errors),
                PackageBodyElement::LibraryPackage(n) => collect_package_body_errors(&n.value.body, errors),
                PackageBodyElement::PartDef(n) => collect_part_def_body_errors(&n.value.body, errors),
                PackageBodyElement::PartUsage(n) => collect_part_usage_body_errors(&n.value.body, errors),
                PackageBodyElement::ActionDef(n) => collect_action_def_body_errors(&n.value.body, errors),
                PackageBodyElement::ActionUsage(n) => collect_action_usage_body_errors(&n.value.body, errors),
                PackageBodyElement::RequirementDef(n) => collect_requirement_body_errors(&n.value.body, errors),
                PackageBodyElement::RequirementUsage(n) => collect_requirement_body_errors(&n.value.body, errors),
                PackageBodyElement::UseCaseDef(n) => collect_use_case_body_errors(&n.value.body, errors),
                PackageBodyElement::UseCaseUsage(n) => collect_use_case_body_errors(&n.value.body, errors),
                PackageBodyElement::ConcernUsage(n) => collect_requirement_body_errors(&n.value.body, errors),
                PackageBodyElement::StateDef(n) => collect_state_body_errors(&n.value.body, errors),
                PackageBodyElement::StateUsage(n) => collect_state_body_errors(&n.value.body, errors),
                _ => {}
            }
        }
    }
}

fn collect_recovery_errors(root: &RootNamespace) -> Vec<ParseError> {
    let mut errors = Vec::new();
    for element in &root.elements {
        match &element.value {
            crate::ast::RootElement::Package(n) => collect_package_body_errors(&n.value.body, &mut errors),
            crate::ast::RootElement::LibraryPackage(n) => collect_package_body_errors(&n.value.body, &mut errors),
            crate::ast::RootElement::Namespace(n) => collect_package_body_errors(&n.value.body, &mut errors),
            crate::ast::RootElement::Import(_) => {}
        }
    }
    errors
}

/// Parse full input; must consume entire input. Strips UTF-8 BOM if present.
#[allow(clippy::result_large_err)]
pub fn parse_root(input: &str) -> Result<RootNamespace, ParseError> {
    let bytes = input
        .strip_prefix('\u{FEFF}')
        .map(str::as_bytes)
        .unwrap_or_else(|| input.as_bytes());
    let located = LocatedSpan::new(bytes);
    match package::root_namespace(located) {
        Ok((rest, root)) => {
            if !rest.fragment().is_empty() && has_unclosed_brace(bytes) {
                return Err(missing_closing_brace_error_at_eof(bytes));
            }
            if rest.fragment().is_empty() || is_only_trailing_closing_braces(rest) {
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
                if root.elements.is_empty() && is_illegal_top_level_definition(rest.fragment()) {
                    pe = pe
                        .with_code("illegal_top_level_definition")
                        .with_expected("'package', 'namespace', or 'import'")
                        .with_suggestion(
                            "Wrap this declaration in `package ... { ... }` or `namespace ... { ... }`.",
                        );
                    pe.message = "illegal top-level definition".to_string();
                }
                Err(pe)
            }
        }
        Err(nom::Err::Error(e)) => Err(missing_closing_brace_error(bytes, e.input).unwrap_or_else(|| {
            nom_err_to_parse_error(
                &e,
                None,
                Some("'package', 'namespace', or 'import' at top level; or valid element in package body"),
            )
        })),
        Err(nom::Err::Failure(e)) => Err(missing_closing_brace_error(bytes, e.input).unwrap_or_else(|| {
            nom_err_to_parse_error(
                &e,
                None,
                Some("'package', 'namespace', or 'import' at top level; or valid element in package body"),
            )
        })),
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
        || lex::starts_with_any_keyword(trimmed.as_bytes(), lex::PACKAGE_BODY_STARTERS)
        || trimmed.starts_with("abstract ")
        || trimmed.starts_with("in ")
        || trimmed.starts_with("out ")
        || trimmed.starts_with("perform ")
        || trimmed.starts_with("transition ")
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
                let pe = missing_closing_brace_error(bytes, e.input).unwrap_or_else(|| {
                    nom_err_to_parse_error(
                        &e,
                        None,
                        Some("'package', 'namespace', or 'import'"),
                    )
                });
                let consumed = &bytes[..e.input.location_offset()];
                let opens = consumed.iter().filter(|&&b| b == b'{').count();
                let closes = consumed.iter().filter(|&&b| b == b'}').count();
                let depth = opens.saturating_sub(closes);
                // When inside a package, only add errors for invalid identifiers (e.g. "test", "test2"),
                // not for valid syntax we hit while skipping (e.g. "}", "//", "part def").
                let is_inside_package = depth > 0;
                let should_report = pe.code.as_deref() == Some("missing_closing_brace")
                    || !is_inside_package
                    || should_report_error_inside_package(pe.found.as_deref().unwrap_or(""));
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

    if input.fragment().is_empty()
        && has_unclosed_brace(bytes)
        && !errors
            .iter()
            .any(|e| e.code.as_deref() == Some("missing_closing_brace"))
    {
        errors.push(missing_closing_brace_error_at_eof(bytes));
    }

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

    errors.extend(collect_recovery_errors(&RootNamespace {
        elements: elements.clone(),
    }));
    errors.sort_by_key(|e| (e.offset.unwrap_or(usize::MAX), e.line.unwrap_or(u32::MAX)));

    ParseResult {
        root: RootNamespace { elements },
        errors,
    }
}
