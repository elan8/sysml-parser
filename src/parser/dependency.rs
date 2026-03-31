//! Dependency parsing (BNF Dependency, DependencyDeclaration).

use crate::ast::{ConnectBody, Dependency, Identification, Node};
use crate::parser::lex::{qualified_name, ws1, ws_and_comments};
use crate::parser::node_from_to;
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::multi::separated_list1;
use nom::sequence::preceded;
use nom::IResult;
use nom::Parser;

/// dependency DependencyDeclaration RelationshipBody
/// DependencyDeclaration = (Identification 'from')? client(s) 'to' supplier(s)
/// Parses: first token may be Identification (if "from" follows) or first client.
pub(crate) fn dependency(input: Input<'_>) -> IResult<Input<'_>, Node<Dependency>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"dependency"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, first) = preceded(ws_and_comments, qualified_name).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    let (input, (ident, clients, suppliers)) = alt((
        // Identification 'from' client_list 'to' supplier_list
        map(
            (
                preceded(tag(&b"from"[..]), ws1),
                separated_list1(
                    preceded(ws_and_comments, tag(&b","[..])),
                    preceded(ws_and_comments, qualified_name),
                ),
                preceded(ws_and_comments, tag(&b"to"[..])),
                ws1,
                separated_list1(
                    preceded(ws_and_comments, tag(&b","[..])),
                    preceded(ws_and_comments, qualified_name),
                ),
            ),
            |(_, clients, _, _, suppliers)| {
                let ident = Identification {
                    short_name: None,
                    name: Some(first.clone()),
                };
                (Some(ident), clients, suppliers)
            },
        ),
        // client_list 'to' supplier_list (first is first client)
        map(
            (
                separated_list1(
                    preceded(ws_and_comments, tag(&b","[..])),
                    preceded(ws_and_comments, qualified_name),
                ),
                preceded(ws_and_comments, tag(&b"to"[..])),
                ws1,
                separated_list1(
                    preceded(ws_and_comments, tag(&b","[..])),
                    preceded(ws_and_comments, qualified_name),
                ),
            ),
            |(rest_clients, _, _, suppliers)| {
                let mut clients = vec![first.clone()];
                clients.extend(rest_clients);
                (None, clients, suppliers)
            },
        ),
        // single client 'to' supplier_list
        map(
            (
                preceded(ws_and_comments, tag(&b"to"[..])),
                ws1,
                separated_list1(
                    preceded(ws_and_comments, tag(&b","[..])),
                    preceded(ws_and_comments, qualified_name),
                ),
            ),
            |(_, _, suppliers)| (None, vec![first.clone()], suppliers),
        ),
    ))
    .parse(input)?;
    let (input, body) = relationship_body_connect(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            Dependency {
                identification: ident,
                clients,
                suppliers,
                body,
            },
        ),
    ))
}

fn relationship_body_connect(input: Input<'_>) -> IResult<Input<'_>, ConnectBody> {
    let (input, _) = ws_and_comments(input)?;
    let (input, body) = alt((
        map(tag(&b";"[..]), |_| ConnectBody::Semicolon),
        map(
            nom::sequence::delimited(
                tag(&b"{"[..]),
                crate::parser::lex::skip_until_brace_end,
                preceded(ws_and_comments, tag(&b"}"[..])),
            ),
            |_| ConnectBody::Brace,
        ),
    ))
    .parse(input)?;
    Ok((input, body))
}
