//! Nom-based parser for SysML v2 textual notation.

use crate::ast::{
    AttributeBody, AttributeDef, AttributeUsage, Identification, Import, Package, PackageBody,
    PackageBodyElement, PartDef, PartDefBody, PartDefBodyElement, PartUsage, PartUsageBody,
    PartUsageBodyElement, RootNamespace, Visibility,
};
use crate::error::ParseError;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until, take_while, take_while1};
use nom::combinator::{map, opt};
use nom::multi::many0;
use nom::sequence::{delimited, preceded, tuple};
use nom::IResult;

/// Skip optional whitespace (space, tab, newline).
fn ws(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = take_while(|c: u8| c == b' ' || c == b'\t' || c == b'\n' || c == b'\r')(input)?;
    Ok((input, ()))
}

/// Skip whitespace and comments (block, single-line, doc+block). Use between tokens and at body boundaries.
fn ws_and_comments(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = take_while(|c: u8| c == b' ' || c == b'\t' || c == b'\n' || c == b'\r')(input)?;
    let (input, _) = many0(alt((
        block_comment,
        line_comment,
        doc_then_block_comment,
    )))(input)?;
    Ok((input, ()))
}

/// Block comment: /* ... */
fn block_comment(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = tag("/*")(input)?;
    let (input, _) = take_until("*/")(input)?;
    let (input, _) = tag("*/")(input)?;
    let (input, _) = ws(input)?;
    Ok((input, ()))
}

/// Single-line comment: // to EOL (consumes the newline).
fn line_comment(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = tag("//")(input)?;
    let (input, _) = take_while(|c: u8| c != b'\n' && c != b'\r')(input)?;
    let (input, _) = take_while(|c: u8| c == b'\n' || c == b'\r')(input)?;
    Ok((input, ()))
}

/// Doc keyword followed by optional whitespace and a block comment.
fn doc_then_block_comment(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = tag("doc")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag("/*")(input)?;
    let (input, _) = take_until("*/")(input)?;
    let (input, _) = tag("*/")(input)?;
    let (input, _) = ws(input)?;
    Ok((input, ()))
}

/// Parse one or more whitespace characters (consumes at least one).
fn ws1(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = take_while1(|c: u8| c == b' ' || c == b'\t' || c == b'\n' || c == b'\r')(input)?;
    Ok((input, ()))
}

/// NAME: BASIC_NAME (identifier) or UNRESTRICTED_NAME (single-quoted string).
fn name(input: &[u8]) -> IResult<&[u8], String> {
    alt((quoted_name, basic_name))(input)
}

/// Unquoted identifier: letter or underscore, then alphanumeric or underscore.
fn basic_name(input: &[u8]) -> IResult<&[u8], String> {
    let (input, raw) = take_while1(|c: u8| c.is_ascii_alphanumeric() || c == b'_')(input)?;
    let s = String::from_utf8_lossy(raw).into_owned();
    Ok((input, s))
}

/// Quoted name: '...' (content between single quotes; \' for escape).
fn quoted_name(input: &[u8]) -> IResult<&[u8], String> {
    let (input, _) = tag("'")(input)?;
    let mut s = String::new();
    let mut i = input;
    while !i.is_empty() {
        if i.starts_with(b"\\'") {
            s.push('\'');
            i = &i[2..];
        } else if i[0] == b'\'' {
            i = &i[1..];
            break;
        } else {
            s.push(i[0] as char);
            i = &i[1..];
        }
    }
    Ok((i, s))
}

/// QualifiedName: ( '$' '::' )? ( NAME '::' )* NAME. Returns string like "SI::kg" or "ISQ::mass".
fn qualified_name(input: &[u8]) -> IResult<&[u8], String> {
    let (input, _) = ws_and_comments(input)?;
    let (input, opt_dollar) = opt(tag("$"))(input)?;
    let (input, _) = opt(preceded(tag("::"), ws_and_comments))(input)?;
    let (input, first) = name(input)?;
    let (input, rest_segments) = many0(preceded(
        preceded(ws_and_comments, tag("::")),
        preceded(ws_and_comments, name),
    ))(input)?;
    let mut segments = Vec::new();
    if opt_dollar.is_some() {
        segments.push("$".to_string());
    }
    segments.push(first);
    segments.extend(rest_segments);
    let s = segments.join("::");
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
        preceded(ws_and_comments, tag("<")),
        preceded(ws_and_comments, name),
        preceded(ws_and_comments, tag(">")),
    ))(input)?;
    let (input, decl_name) = opt(preceded(ws_and_comments, name))(input)?;
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
        map(preceded(ws_and_comments, tag(";")), |_| PackageBody::Semicolon),
        map(
            delimited(
                preceded(ws_and_comments, tag("{")),
                preceded(ws_and_comments, many0(preceded(ws_and_comments, package_body_element))),
                preceded(ws_and_comments, tag("}")),
            ),
            |elements| PackageBody::Brace { elements },
        ),
    ))(input)
}

/// RelationshipBody: ';' or '{' ... '}'. For '{' we skip content until matching '}'.
fn relationship_body(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(";"), |_| ()),
        map(
            delimited(
                tag("{"),
                skip_until_brace_end,
                preceded(ws_and_comments, tag("}")),
            ),
            |_| (),
        ),
    ))(input)
}

/// Skip any content until we see '}' at the same brace level (no nesting).
fn skip_until_brace_end(input: &[u8]) -> IResult<&[u8], ()> {
    let mut depth = 1u32;
    let mut i = input;
    while depth > 0 && !i.is_empty() {
        if i.starts_with(b"/*") {
            if let Some(pos) = find_subslice(i, b"*/") {
                i = &i[pos + 2..];
                continue;
            }
            break;
        }
        if i.starts_with(b"//") {
            let mut j = 2;
            while j < i.len() && i[j] != b'\n' && i[j] != b'\r' {
                j += 1;
            }
            while j < i.len() && (i[j] == b'\n' || i[j] == b'\r') {
                j += 1;
            }
            i = &i[j..];
            continue;
        }
        if i[0] == b'{' {
            depth += 1;
        } else if i[0] == b'}' {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        i = &i[1..];
    }
    Ok((i, ()))
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Import: visibility? 'import' isImportAll? (QualifiedName | QualifiedName '::' '*') RelationshipBody
fn import_(input: &[u8]) -> IResult<&[u8], Import> {
    let (input, _) = ws_and_comments(input)?;
    let (input, visibility) = opt(alt((
        map(preceded(tag("public"), ws1), |_| Visibility::Public),
        map(preceded(tag("private"), ws1), |_| Visibility::Private),
        map(preceded(tag("protected"), ws1), |_| Visibility::Protected),
    )))(input)?;
    let (input, _) = tag("import")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = opt(preceded(tag("all"), ws1))(input)?;
    let (input, (target, is_import_all)) = alt((
        map(
            tuple((qualified_name, preceded(ws_and_comments, tag("::")), preceded(ws_and_comments, tag("*")))),
            |(q, _, _)| (format!("{}::*", q), true),
        ),
        map(qualified_name, |q| (q, false)),
    ))(input)?;
    let (input, _) = relationship_body(input)?;
    Ok((
        input,
        Import {
            visibility,
            is_import_all,
            target,
        },
    ))
}

/// Attribute body: ';' or '{' ... '}' (skip content inside braces)
fn attribute_body(input: &[u8]) -> IResult<&[u8], AttributeBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(";"), |_| AttributeBody::Semicolon),
        map(
            delimited(tag("{"), skip_until_brace_end, preceded(ws_and_comments, tag("}"))),
            |_| AttributeBody::Brace,
        ),
    ))(input)
}

/// Attribute definition: 'attribute' name ( ':>' | ':' )? qualified_name? body
fn attribute_def(input: &[u8]) -> IResult<&[u8], AttributeDef> {
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag("attribute")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = opt(preceded(tag("def"), ws1))(input)?;
    let (input, name_str) = name(input)?;
    let (input, typing) = opt(alt((
        preceded(preceded(ws_and_comments, tag(":>")), preceded(ws_and_comments, qualified_name)),
        preceded(preceded(ws_and_comments, tag(":")), preceded(ws_and_comments, qualified_name)),
    )))(input)?;
    let (input, body) = attribute_body(input)?;
    Ok((
        input,
        AttributeDef {
            name: name_str,
            typing,
            body,
        },
    ))
}

/// Part def body: ';' or '{' PartDefBodyElement* '}'
fn part_def_body(input: &[u8]) -> IResult<&[u8], PartDefBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(";"), |_| PartDefBody::Semicolon),
        map(
            delimited(
                tag("{"),
                preceded(
                    ws_and_comments,
                    many0(preceded(ws_and_comments, part_def_body_element)),
                ),
                preceded(ws_and_comments, tag("}")),
            ),
            |elements| PartDefBody::Brace { elements },
        ),
    ))(input)
}

fn part_def_body_element(input: &[u8]) -> IResult<&[u8], PartDefBodyElement> {
    map(attribute_def, PartDefBodyElement::AttributeDef)(input)
}

/// Part definition: 'part' 'def' Identification ( ':>' qualified_name )? body
fn part_def(input: &[u8]) -> IResult<&[u8], PartDef> {
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag("part")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = tag("def")(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, specializes) = opt(preceded(
        preceded(ws_and_comments, tag(":>")),
        preceded(ws_and_comments, qualified_name),
    ))(input)?;
    let (input, body) = part_def_body(input)?;
    Ok((
        input,
        PartDef {
            identification,
            specializes,
            body,
        },
    ))
}

/// Take input until we hit one of the terminator bytes (e.g. '{' or ';'), return as string (trimmed).
fn take_until_terminator<'a>(input: &'a [u8], terminators: &[u8]) -> IResult<&'a [u8], String> {
    let mut i = 0;
    while i < input.len() {
        if terminators.contains(&input[i]) {
            let s = String::from_utf8_lossy(&input[..i]).trim().to_string();
            return Ok((&input[i..], s));
        }
        if input[i] == b'/' && i + 1 < input.len() && (input[i + 1] == b'*' || input[i + 1] == b'/') {
            break;
        }
        i += 1;
    }
    let s = String::from_utf8_lossy(&input[..i]).trim().to_string();
    Ok((&input[i..], s))
}

/// Attribute usage: 'attribute' name ( 'redefines' qualified_name )? ( '=' value )? body
fn attribute_usage(input: &[u8]) -> IResult<&[u8], AttributeUsage> {
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag("attribute")(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, redefines) = opt(preceded(
        preceded(ws_and_comments, tag("redefines")),
        preceded(ws1, qualified_name),
    ))(input)?;
    let (input, value) = opt(preceded(
        preceded(ws_and_comments, tag("=")),
        preceded(ws_and_comments, |i| take_until_terminator(i, b"{;")),
    ))(input)?;
    let (input, body) = attribute_body(input)?;
    Ok((
        input,
        AttributeUsage {
            name: name_str,
            redefines,
            value: value.filter(|s| !s.is_empty()),
            body,
        },
    ))
}

/// Part usage body: ';' or '{' PartUsageBodyElement* '}'
fn part_usage_body(input: &[u8]) -> IResult<&[u8], PartUsageBody> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(tag(";"), |_| PartUsageBody::Semicolon),
        map(
            delimited(
                tag("{"),
                preceded(
                    ws_and_comments,
                    many0(preceded(ws_and_comments, part_usage_body_element)),
                ),
                preceded(ws_and_comments, tag("}")),
            ),
            |elements| PartUsageBody::Brace { elements },
        ),
    ))(input)
}

fn part_usage_body_element(input: &[u8]) -> IResult<&[u8], PartUsageBodyElement> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(attribute_usage, PartUsageBodyElement::AttributeUsage),
        map(part_usage, |p| PartUsageBodyElement::PartUsage(Box::new(p))),
    ))(input)
}

/// Multiplicity: '[' ... ']' as string
fn multiplicity(input: &[u8]) -> IResult<&[u8], String> {
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag("[")(input)?;
    let (input, content) = take_until("]")(input)?;
    let (input, _) = tag("]")(input)?;
    let s = format!("[{}]", String::from_utf8_lossy(content).trim());
    Ok((input, s))
}

/// Part usage: 'part' name ':' type_name multiplicity? 'ordered'? ( 'subsets' name '=' value )? body
fn part_usage(input: &[u8]) -> IResult<&[u8], PartUsage> {
    let (input, _) = ws_and_comments(input)?;
    let (input, _) = tag("part")(input)?;
    let (input, _) = ws1(input)?;
    let (input, name_str) = name(input)?;
    let (input, type_name) = opt(preceded(
        preceded(ws_and_comments, tag(":")),
        preceded(ws_and_comments, qualified_name),
    ))(input)?;
    let (input, multiplicity_opt) = opt(multiplicity)(input)?;
    let (input, ordered) = opt(preceded(ws_and_comments, tag("ordered")))(input)?;
    let (input, subsets) = opt(preceded(
        preceded(ws_and_comments, tag("subsets")),
        preceded(ws1, tuple((
            name,
            opt(preceded(
                preceded(ws_and_comments, tag("=")),
                preceded(ws_and_comments, |i| take_until_terminator(i, b"{;")),
            )),
        ))),
    ))(input)?;
    let (input, body) = part_usage_body(input)?;
    let subsets = subsets.map(|(feat, val)| (feat, val.filter(|s| !s.is_empty())));
    Ok((
        input,
        PartUsage {
            name: name_str,
            type_name: type_name.unwrap_or_else(|| String::new()),
            multiplicity: multiplicity_opt,
            ordered: ordered.is_some(),
            subsets,
            body,
        },
    ))
}

/// PackageBodyElement: Package | Import | PartDef | PartUsage
fn package_body_element(input: &[u8]) -> IResult<&[u8], PackageBodyElement> {
    let (input, _) = ws_and_comments(input)?;
    alt((
        map(package_, PackageBodyElement::Package),
        map(import_, PackageBodyElement::Import),
        map(part_def, PackageBodyElement::PartDef),
        map(part_usage, PackageBodyElement::PartUsage),
    ))(input)
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
    let (input, _) = ws_and_comments(input)?;
    let (input, elements) = many0(preceded(ws_and_comments, package_body_element))(input)?;
    let (input, _) = ws_and_comments(input)?;
    Ok((input, RootNamespace { elements }))
}

/// Parse full input; must consume entire input. Strips UTF-8 BOM if present.
pub fn parse_root(input: &str) -> Result<RootNamespace, ParseError> {
    let bytes = input.strip_prefix('\u{FEFF}').map(str::as_bytes).unwrap_or_else(|| input.as_bytes());
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
