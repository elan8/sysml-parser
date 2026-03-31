//! Parser input type and span extraction for source locations.

use crate::ast::{Node, Span};
use nom_locate::LocatedSpan;

/// Parser input: bytes with location tracking (offset, line, column).
pub type Input<'a> = LocatedSpan<&'a [u8]>;

/// Build a Span from the start and rest inputs (the consumed region).
pub fn span_from_to(start: Input<'_>, rest: Input<'_>) -> Span {
    let len = start.fragment().len().saturating_sub(rest.fragment().len());
    Span {
        offset: start.location_offset(),
        line: start.location_line(),
        column: start.get_column(),
        len,
    }
}

/// Wrap a parsed value in a Node using the span from start to rest.
pub fn node_from_to<T>(start: Input<'_>, rest: Input<'_>, value: T) -> Node<T> {
    Node::new(span_from_to(start, rest), value)
}

/// Run a parser and return the value together with the span of the consumed input.
/// Use this to capture sub-spans for semantic tokens (e.g. name, type reference).
pub fn with_span<'a, F, O, E>(
    mut f: F,
) -> impl FnMut(Input<'a>) -> nom::IResult<Input<'a>, (Span, O), E>
where
    E: nom::error::ParseError<Input<'a>>,
    F: FnMut(Input<'a>) -> nom::IResult<Input<'a>, O, E>,
{
    move |input: Input<'a>| {
        let start = input;
        let (rest, value) = f(input)?;
        Ok((rest, (span_from_to(start, rest), value)))
    }
}

#[cfg(test)]
mod tests {
    use super::{span_from_to, Input};
    use crate::ast::Span;
    use nom::bytes::complete::tag;
    use nom::error::Error;
    use nom::Parser;
    use nom_locate::LocatedSpan;

    #[test]
    fn span_from_to_consumed_region() {
        let bytes = b"package Foo;" as &[u8];
        let start = LocatedSpan::new(bytes);
        let (rest, _) = tag::<_, Input<'_>, Error<Input<'_>>>(&b"package"[..])
            .parse(start)
            .unwrap();
        let span = span_from_to(start, rest);
        assert_eq!(
            span,
            Span {
                offset: 0,
                line: 1,
                column: 1,
                len: 7,
            },
            "span should cover consumed 'package' (7 bytes)"
        );
    }
}
