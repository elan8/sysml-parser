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
mod expr;
mod import;
mod interface;
mod lex;
mod package;
mod part;
mod port;
mod span;

pub(crate) use span::{node_from_to, Input};

use crate::ast::RootNamespace;
use crate::error::ParseError;
use nom::InputLength;
use nom_locate::LocatedSpan;

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
                let offset = located.location_offset() + located.input_len() - rest.input_len();
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
                Err(ParseError::new("expected end of input").with_location(offset, rest.location_line(), rest.get_column()))
            }
        }
        Err(nom::Err::Error(e)) => {
            let offset = e.input.location_offset();
            let line = e.input.location_line();
            let column = e.input.get_column();
            Err(ParseError::new(format!("parse error: {:?}", e.code)).with_location(offset, line, column))
        }
        Err(nom::Err::Failure(e)) => {
            let offset = e.input.location_offset();
            let line = e.input.location_line();
            let column = e.input.get_column();
            Err(ParseError::new(format!("parse error: {:?}", e.code)).with_location(offset, line, column))
        }
        Err(nom::Err::Incomplete(_)) => Err(ParseError::new("unexpected end of input")),
    }
}
