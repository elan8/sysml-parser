//! Lexer and skip helpers: whitespace, comments, names, qualified names, and body-skip utilities.

use crate::ast::Identification;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until, take_while, take_while1};
use nom::combinator::{map, opt, rest, value};
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
use nom::Parser;
use nom::IResult;

/// Skip optional whitespace (space, tab, newline).
pub(crate) fn ws(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = take_while(|c: u8| c == b' ' || c == b'\t' || c == b'\n' || c == b'\r').parse(input)?;
    Ok((input, ()))
}

/// Skip whitespace and comments (block, single-line). Use between tokens and at body boundaries.
/// Does NOT consume "doc /* ... */" — that is a body element (PackageBodyElement::Doc etc.) and must
/// be parsed explicitly so it appears in the AST. //* ... */ is tried before line_comment so that
/// "//*" starts a block comment, not a line comment.
pub(crate) fn ws_and_comments(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = take_while(|c: u8| c == b' ' || c == b'\t' || c == b'\n' || c == b'\r').parse(input)?;
    let (input, _) = many0(alt((
        block_comment,
        block_comment_slash_star,
        line_comment,
    ))).parse(input)?;
    Ok((input, ()))
}

/// Block comment: /* ... */
fn block_comment(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = tag(&b"/*"[..]).parse(input)?;
    let (input, _) = take_until(&b"*/"[..]).parse(input)?;
    let (input, _) = tag(&b"*/"[..]).parse(input)?;
    let (input, _) = ws(input)?;
    Ok((input, ()))
}

/// Block comment starting with //* ... */ (e.g. in 4a fixture).
fn block_comment_slash_star(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = tag(&b"//*"[..]).parse(input)?;
    let (input, _) = take_until(&b"*/"[..]).parse(input)?;
    let (input, _) = tag(&b"*/"[..]).parse(input)?;
    let (input, _) = ws(input)?;
    Ok((input, ()))
}

/// Single-line comment: // to EOL (consumes the newline).
fn line_comment(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = tag(&b"//"[..]).parse(input)?;
    let (input, _) = take_while(|c: u8| c != b'\n' && c != b'\r').parse(input)?;
    let (input, _) = take_while(|c: u8| c == b'\n' || c == b'\r').parse(input)?;
    Ok((input, ()))
}

/// Parse one or more whitespace characters (consumes at least one).
pub(crate) fn ws1(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = take_while1(|c: u8| c == b' ' || c == b'\t' || c == b'\n' || c == b'\r').parse(input)?;
    Ok((input, ()))
}

/// Skip to the next sync point (next line start after newline and ws/comments), or to end of input.
/// Used for error recovery so parsing can continue after a failed top-level element.
pub(crate) fn skip_to_next_sync_point(input: Input<'_>) -> IResult<Input<'_>, ()> {
    alt((
        map(
            (
                take_until(&b"\n"[..]),
                opt(tag(&b"\n"[..])),
                ws_and_comments,
            ),
            |_| (),
        ),
        value((), rest),
    ))
    .parse(input)
}

/// NAME: BASIC_NAME (identifier) or UNRESTRICTED_NAME (single-quoted string).
pub(crate) fn name(input: Input<'_>) -> IResult<Input<'_>, String> {
    alt((quoted_name, basic_name)).parse(input)
}

/// Unquoted identifier: letter or underscore, then alphanumeric or underscore.
fn basic_name(input: Input<'_>) -> IResult<Input<'_>, String> {
    let (input, raw) = take_while1(|c: u8| c.is_ascii_alphanumeric() || c == b'_').parse(input)?;
    let s = String::from_utf8_lossy(raw.fragment()).into_owned();
    Ok((input, s))
}

/// Quoted name: '...' (content between single quotes; \' for escape).
fn quoted_name(input: Input<'_>) -> IResult<Input<'_>, String> {
    let (input, _) = tag(&b"'"[..]).parse(input)?;
    let frag = input.fragment();
    let mut s = String::new();
    let mut count = 0usize;
    while count < frag.len() {
        if frag[count] == b'\\' && count + 1 < frag.len() && frag[count + 1] == b'\'' {
            s.push('\'');
            count += 2;
        } else if frag[count] == b'\'' {
            count += 1;
            break;
        } else {
            s.push(frag[count] as char);
            count += 1;
        }
    }
    let (input, _) = nom::bytes::complete::take(count).parse(input)?;
    Ok((input, s))
}

/// QualifiedName: ( '$' '::' )? ( NAME '::' )* NAME. Returns string like "SI::kg" or "ISQ::mass".
pub(crate) fn qualified_name(input: Input<'_>) -> IResult<Input<'_>, String> {
    let (input, _) = ws_and_comments(input)?;
    let (input, opt_dollar) = opt(tag(&b"$"[..])).parse(input)?;
    let (input, _) = opt(preceded(tag(&b"::"[..]), ws_and_comments)).parse(input)?;
    let (input, first) = name(input)?;
    let (input, rest_segments) = many0(preceded(
        preceded(ws_and_comments, tag(&b"::"[..])),
        preceded(ws_and_comments, name),
    ))
    .parse(input)?;
    let mut segments = Vec::new();
    if opt_dollar.is_some() {
        segments.push("$".to_string());
    }
    segments.push(first);
    segments.extend(rest_segments);
    let s = segments.join("::");
    Ok((input, s))
}

/// Skip any content until we see '}' at the same brace level (tracks nesting, skips comments).
pub(crate) fn skip_until_brace_end(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let frag = input.fragment();
    let mut depth = 1u32;
    let mut pos = 0usize;
    while depth > 0 && pos < frag.len() {
        if pos + 2 <= frag.len() && frag[pos..].starts_with(b"/*") {
            if let Some(rel) = find_subslice(&frag[pos..], b"*/") {
                pos += rel + 2;
                continue;
            }
            break;
        }
        if pos + 2 <= frag.len() && frag[pos..].starts_with(b"//") {
            let mut j = pos + 2;
            while j < frag.len() && frag[j] != b'\n' && frag[j] != b'\r' {
                j += 1;
            }
            while j < frag.len() && (frag[j] == b'\n' || frag[j] == b'\r') {
                j += 1;
            }
            pos = j;
            continue;
        }
        if frag[pos] == b'{' {
            depth += 1;
        } else if frag[pos] == b'}' {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        pos += 1;
    }
    let (input, _) = nom::bytes::complete::take(pos).parse(input)?;
    Ok((input, ()))
}

pub(crate) fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Identification: ( '<' ShortName '>' )? ( Name )?
pub(crate) fn identification(input: Input<'_>) -> IResult<Input<'_>, Identification> {
    let (input, short_name) = opt(delimited(
        preceded(ws_and_comments, tag(&b"<"[..])),
        preceded(ws_and_comments, name),
        preceded(ws_and_comments, tag(&b">"[..])),
    ))
    .parse(input)?;
    let (input, decl_name) = opt(preceded(ws_and_comments, name)).parse(input)?;
    Ok((
        input,
        Identification {
            short_name,
            name: decl_name,
        },
    ))
}

/// Take input until we hit one of the terminator bytes (e.g. '{' or ';'), return as string (trimmed).
pub(crate) fn take_until_terminator<'a>(input: Input<'a>, terminators: &'a [u8]) -> IResult<Input<'a>, String> {
    let frag = input.fragment();
    let mut i = 0;
    while i < frag.len() {
        if terminators.contains(&frag[i]) {
            let s = String::from_utf8_lossy(&frag[..i]).trim().to_string();
            let (input, _) = nom::bytes::complete::take(i).parse(input)?;
            return Ok((input, s));
        }
        if frag[i] == b'/' && i + 1 < frag.len() && (frag[i + 1] == b'*' || frag[i + 1] == b'/') {
            break;
        }
        i += 1;
    }
    let s = String::from_utf8_lossy(&frag[..i]).trim().to_string();
    let (input, _) = nom::bytes::complete::take(i).parse(input)?;
    Ok((input, s))
}
