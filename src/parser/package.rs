//! Package and root namespace parsing.

use crate::ast::{
    FilterMember, LibraryPackage, NamespaceDecl, Node, Package, PackageBody, PackageBodyElement,
    ParseErrorNode, RootElement, RootNamespace, Visibility,
};
use crate::parser::action::{action_def, action_usage};
use crate::parser::alias::alias_def;
use crate::parser::attribute::attribute_def;
use crate::parser::connection::connection_def;
use crate::parser::dependency::dependency;
use crate::parser::constraint::{calc_def, constraint_def};
use crate::parser::enumeration::enum_def;
use crate::parser::individual::individual_def;
use crate::parser::item::item_def;
use crate::parser::metadata::metadata_def;
use crate::parser::occurrence::occurrence_def;
use crate::parser::expr::expression;
use crate::parser::import::import_;
use crate::parser::interface::interface_def;
use crate::parser::lex::{
    identification, recover_body_element, starts_with_any_keyword, ws1, ws_and_comments,
    PACKAGE_BODY_STARTERS,
};
use crate::parser::node_from_to;
use crate::parser::part::{part_def_or_usage, PartDefOrUsage};
use crate::parser::port::port_def;
use crate::parser::requirement::{
    comment_annotation, concern_usage, doc_comment, requirement_def, requirement_usage, satisfy,
    textual_representation,
};
use crate::parser::state::state_def;
use crate::parser::state::state_usage;
use crate::parser::usecase::{actor_decl, use_case_def, use_case_usage};
use crate::parser::view::{
    rendering_def, rendering_usage, view_def, view_usage, viewpoint_def, viewpoint_usage,
};
use crate::parser::Input;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{map, opt};
use nom::multi::many0;
use nom::sequence::preceded;
use nom::Parser;
use nom::IResult;

fn recovery_found_snippet(input: Input<'_>) -> Option<String> {
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

/// Keyword "package" with following whitespace.
fn keyword_package(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = tag(&b"package"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    Ok((input, ()))
}

/// library (optional standard) package Identification PackageBody (BNF LibraryPackage)
fn library_package_(input: Input<'_>) -> IResult<Input<'_>, Node<LibraryPackage>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"library"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, is_standard) = opt(preceded(tag(&b"standard"[..]), ws1))
        .parse(input)
        .map(|(i, o)| (i, o.is_some()))?;
    let (input, _) = keyword_package(input)?;
    let (input, identification) = identification(input)?;
    let (input, body) = package_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            LibraryPackage {
                is_standard,
                identification,
                body,
            },
        ),
    ))
}

/// package Identification PackageBody
fn package_(input: Input<'_>) -> IResult<Input<'_>, Node<Package>> {
    let start = input;
    let (input, _) = keyword_package(input)?;
    let (input, identification) = identification(input)?;
    let (input, body) = package_body(input)?;
    Ok((
        input,
        node_from_to(start, input, Package {
            identification,
            body,
        }),
    ))
}

/// KerML namespace Identification NamespaceBody
fn namespace_decl(input: Input<'_>) -> IResult<Input<'_>, Node<NamespaceDecl>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"namespace"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = identification(input)?;
    let (input, body) = package_body(input)?;
    Ok((
        input,
        node_from_to(start, input, NamespaceDecl {
            identification,
            body,
        }),
    ))
}

/// One root-level element: import, package, or namespace (BNF PackageBodyElement* at root).
pub(crate) fn root_element(input: Input<'_>) -> IResult<Input<'_>, Node<RootElement>> {
    let (input, _) = ws_and_comments(input)?;
    let start = input;
    let (input, elem) = alt((
        map(import_, RootElement::Import),
        map(namespace_decl, RootElement::Namespace),
        map(library_package_, RootElement::LibraryPackage),
        map(package_, RootElement::Package),
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

/// PackageBody: ';' | '{' PackageBodyElement* '}'
/// Brace form is tried first so that ws before '{' is not consumed by the semicolon branch.
pub(crate) fn package_body(input: Input<'_>) -> IResult<Input<'_>, PackageBody> {
    alt((
        package_body_brace,
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| PackageBody::Semicolon),
    ))
    .parse(input)
}

fn package_body_brace(input: Input<'_>) -> IResult<Input<'_>, PackageBody> {
    let (mut input, _) = preceded(ws_and_comments, tag(&b"{"[..])).parse(input)?;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().is_empty() {
            return Ok((input, PackageBody::Brace { elements }));
        }
        if input.fragment().starts_with(b"}") {
            let (input, _) = preceded(ws_and_comments, tag(&b"}"[..])).parse(input)?;
            return Ok((input, PackageBody::Brace { elements }));
        }
        match package_body_element(input) {
            Ok((next, element)) => {
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                elements.push(element);
                input = next;
            }
            Err(_) if starts_with_any_keyword(input.fragment(), PACKAGE_BODY_STARTERS) => {
                let (next, _) = recover_body_element(input, PACKAGE_BODY_STARTERS)?;
                if next.location_offset() == input.location_offset() {
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::Many0,
                    )));
                }
                elements.push(node_from_to(
                    input,
                    next,
                    PackageBodyElement::Error(Node::new(
                        crate::ast::Span::dummy(),
                        ParseErrorNode {
                            message: "recovered package body element".to_string(),
                            code: "recovered_package_body_element".to_string(),
                            expected: Some("valid package body element".to_string()),
                            found: recovery_found_snippet(input),
                            suggestion: None,
                        },
                    )),
                ));
                input = next;
            }
            Err(_) => {
                return Err(nom::Err::Failure(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Tag,
                )));
            }
        }
    }
}

/// KerML ElementFilterMember: MemberPrefix? 'filter' condition = OwnedExpression ';'
pub(crate) fn filter_member(input: Input<'_>) -> IResult<Input<'_>, Node<FilterMember>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    let (input, visibility) = opt(alt((
        map(preceded(tag(&b"public"[..]), ws1), |_| Visibility::Public),
        map(preceded(tag(&b"private"[..]), ws1), |_| Visibility::Private),
        map(preceded(tag(&b"protected"[..]), ws1), |_| Visibility::Protected),
    )))
    .parse(input)?;
    let (input, _) = tag(&b"filter"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, condition) = expression(input)?;
    let (input, _) = preceded(ws_and_comments, tag(&b";"[..])).parse(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            FilterMember {
                visibility,
                condition,
            },
        ),
    ))
}

/// PackageBodyElement: Package | Import | PartDef | PartUsage | PortDef | InterfaceDef | AliasDef | ActionDef | ActionUsage
pub(crate) fn package_body_element(input: Input<'_>) -> IResult<Input<'_>, Node<PackageBodyElement>> {
    let (input, _) = ws_and_comments(input)?;
    let start = input;
    // Doc first so "doc /* ... */" at start of package body parses before other elements.
    // Annotation parsers grouped to help type inference.
    // Split into groups to avoid nom Choice type explosion with too many alts.
    let annotation_parser = alt((
        map(doc_comment, PackageBodyElement::Doc),
        map(comment_annotation, PackageBodyElement::Comment),
        map(textual_representation, PackageBodyElement::TextualRep),
    ));
    let structural_parser = alt((
        map(filter_member, PackageBodyElement::Filter),
        map(attribute_def, PackageBodyElement::AttributeDef),
        map(library_package_, PackageBodyElement::LibraryPackage),
        map(package_, PackageBodyElement::Package),
        map(import_, PackageBodyElement::Import),
        map(part_def_or_usage, |p| match p {
            PartDefOrUsage::Def(n) => PackageBodyElement::PartDef(n),
            PartDefOrUsage::Usage(n) => PackageBodyElement::PartUsage(n),
        }),
        map(port_def, PackageBodyElement::PortDef),
        map(interface_def, PackageBodyElement::InterfaceDef),
        map(connection_def, PackageBodyElement::ConnectionDef),
        map(dependency, PackageBodyElement::Dependency),
        map(metadata_def, PackageBodyElement::MetadataDef),
        map(enum_def, PackageBodyElement::EnumDef),
        map(occurrence_def, PackageBodyElement::OccurrenceDef),
        map(alias_def, PackageBodyElement::AliasDef),
        map(action_def, PackageBodyElement::ActionDef),
        map(action_usage, PackageBodyElement::ActionUsage),
    ));
    let req_parser = alt((
        map(requirement_def, PackageBodyElement::RequirementDef),
        map(requirement_usage, PackageBodyElement::RequirementUsage),
        map(satisfy, PackageBodyElement::Satisfy),
        map(use_case_def, PackageBodyElement::UseCaseDef),
        map(use_case_usage, PackageBodyElement::UseCaseUsage),
        map(concern_usage, PackageBodyElement::ConcernUsage),
        map(actor_decl, PackageBodyElement::Actor),
        map(state_def, PackageBodyElement::StateDef),
        map(state_usage, PackageBodyElement::StateUsage),
    ));
    let other_parser = alt((
        map(item_def, PackageBodyElement::ItemDef),
        map(individual_def, PackageBodyElement::IndividualDef),
        map(constraint_def, PackageBodyElement::ConstraintDef),
        map(calc_def, PackageBodyElement::CalcDef),
        map(view_def, PackageBodyElement::ViewDef),
        map(viewpoint_def, PackageBodyElement::ViewpointDef),
        map(rendering_def, PackageBodyElement::RenderingDef),
        map(view_usage, PackageBodyElement::ViewUsage),
        map(viewpoint_usage, PackageBodyElement::ViewpointUsage),
        map(rendering_usage, PackageBodyElement::RenderingUsage),
    ));
    let (input, elem) = alt((
        annotation_parser,
        structural_parser,
        req_parser,
        other_parser,
    ))
    .parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

/// Root: (package | namespace)*
pub(crate) fn root_namespace(input: Input<'_>) -> IResult<Input<'_>, RootNamespace> {
    let (input, _) = ws_and_comments(input)?;
    let (input, elements) = many0(preceded(ws_and_comments, root_element)).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    Ok((input, RootNamespace { elements }))
}
