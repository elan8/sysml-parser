//! Package and root namespace parsing.

use crate::ast::{
    FilterMember, LibraryPackage, NamespaceDecl, Node, Package, PackageBody, PackageBodyElement,
    ParseErrorNode, RootElement, RootNamespace, Visibility,
};
use crate::parser::action::{action_def, action_usage};
use crate::parser::allocation::{allocate_usage, allocation_def, allocation_usage};
use crate::parser::alias::alias_def;
use crate::parser::attribute::attribute_def;
use crate::parser::case::{
    analysis_case_def, analysis_case_usage, case_def, case_usage, verification_case_def,
    verification_case_usage,
};
use crate::parser::connection::connection_def;
use crate::parser::dependency::dependency;
use crate::parser::constraint::{calc_def, constraint_def};
use crate::parser::enumeration::enum_def;
use crate::parser::individual::individual_def;
use crate::parser::item::item_def;
use crate::parser::metadata::metadata_def;
use crate::parser::occurrence::{
    individual_usage, occurrence_def, occurrence_usage, snapshot_usage, timeslice_usage,
};
use crate::parser::flow::{flow_def, flow_usage};
use crate::parser::expr::expression;
use crate::parser::import::import_;
use crate::parser::interface::interface_def;
use crate::parser::lex::{
    identification, recover_body_element, starts_with_any_keyword, starts_with_keyword, ws1, ws_and_comments,
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

/// [standard] library package Identification PackageBody (BNF LibraryPackage)
fn library_package_(input: Input<'_>) -> IResult<Input<'_>, Node<LibraryPackage>> {
    let start = input;
    let (input, _) = ws_and_comments(input)?;
    // Accept both `standard library package` (current SysML v2 stdlib)
    // and legacy `library standard package`.
    let (input, is_standard) = if input.fragment().starts_with(b"standard") {
        let (input, _) = tag(&b"standard"[..]).parse(input)?;
        let (input, _) = ws1(input)?;
        let (input, _) = tag(&b"library"[..]).parse(input)?;
        let (input, _) = ws1(input)?;
        (input, true)
    } else {
        let (input, _) = tag(&b"library"[..]).parse(input)?;
        let (input, _) = ws1(input)?;
        let (input, is_standard) = opt(preceded(tag(&b"standard"[..]), ws1))
            .parse(input)
            .map(|(i, o)| (i, o.is_some()))?;
        (input, is_standard)
    };
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

fn package_body_element_fallback(input: Input<'_>) -> IResult<Input<'_>, Node<PackageBodyElement>> {
    let (input, _) = ws_and_comments(input)?;
    let frag = input.fragment();

    if starts_with_keyword(frag, b"part")
        || starts_with_keyword(frag, b"abstract")
        || starts_with_keyword(frag, b"variation")
    {
        let start = input;
        let (input, parsed) = part_def_or_usage(input)?;
        let value = match parsed {
            PartDefOrUsage::Def(n) => PackageBodyElement::PartDef(n),
            PartDefOrUsage::Usage(n) => PackageBodyElement::PartUsage(n),
        };
        return Ok((input, node_from_to(start, input, value)));
    }

    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Tag,
    )))
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
                if let Ok((next, element)) = package_body_element_fallback(input) {
                    if next.location_offset() == input.location_offset() {
                        return Err(nom::Err::Failure(nom::error::Error::new(
                            input,
                            nom::error::ErrorKind::Many0,
                        )));
                    }
                    elements.push(element);
                    input = next;
                    continue;
                }
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
    if let Ok((input, elem)) = map(doc_comment, PackageBodyElement::Doc).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(comment_annotation, PackageBodyElement::Comment).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(textual_representation, PackageBodyElement::TextualRep).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(filter_member, PackageBodyElement::Filter).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(attribute_def, PackageBodyElement::AttributeDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(library_package_, PackageBodyElement::LibraryPackage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(package_, PackageBodyElement::Package).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(import_, PackageBodyElement::Import).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(part_def_or_usage, |p| match p {
        PartDefOrUsage::Def(n) => PackageBodyElement::PartDef(n),
        PartDefOrUsage::Usage(n) => PackageBodyElement::PartUsage(n),
    })
    .parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(port_def, PackageBodyElement::PortDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(interface_def, PackageBodyElement::InterfaceDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(connection_def, PackageBodyElement::ConnectionDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(dependency, PackageBodyElement::Dependency).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(metadata_def, PackageBodyElement::MetadataDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(enum_def, PackageBodyElement::EnumDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(occurrence_def, PackageBodyElement::OccurrenceDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(occurrence_usage, PackageBodyElement::OccurrenceUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(individual_usage, PackageBodyElement::OccurrenceUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(snapshot_usage, PackageBodyElement::OccurrenceUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(timeslice_usage, PackageBodyElement::OccurrenceUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(allocation_def, PackageBodyElement::AllocationDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(allocation_usage, PackageBodyElement::AllocationUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(allocate_usage, PackageBodyElement::AllocationUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(flow_def, PackageBodyElement::FlowDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(flow_usage, PackageBodyElement::FlowUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(alias_def, PackageBodyElement::AliasDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(action_def, PackageBodyElement::ActionDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(action_usage, PackageBodyElement::ActionUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(requirement_def, PackageBodyElement::RequirementDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(requirement_usage, PackageBodyElement::RequirementUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(satisfy, PackageBodyElement::Satisfy).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(use_case_def, PackageBodyElement::UseCaseDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(use_case_usage, PackageBodyElement::UseCaseUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(case_def, PackageBodyElement::CaseDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(case_usage, PackageBodyElement::CaseUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(analysis_case_def, PackageBodyElement::AnalysisCaseDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(analysis_case_usage, PackageBodyElement::AnalysisCaseUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(verification_case_def, PackageBodyElement::VerificationCaseDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(verification_case_usage, PackageBodyElement::VerificationCaseUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(concern_usage, PackageBodyElement::ConcernUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(actor_decl, PackageBodyElement::Actor).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(state_def, PackageBodyElement::StateDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(state_usage, PackageBodyElement::StateUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(item_def, PackageBodyElement::ItemDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(individual_def, PackageBodyElement::IndividualDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(constraint_def, PackageBodyElement::ConstraintDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(calc_def, PackageBodyElement::CalcDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(view_def, PackageBodyElement::ViewDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(viewpoint_def, PackageBodyElement::ViewpointDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(rendering_def, PackageBodyElement::RenderingDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(view_usage, PackageBodyElement::ViewUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(viewpoint_usage, PackageBodyElement::ViewpointUsage).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    let (input, elem) = map(rendering_usage, PackageBodyElement::RenderingUsage).parse(input)?;
    Ok((input, node_from_to(start, input, elem)))
}

/// Root: (package | namespace)*
pub(crate) fn root_namespace(input: Input<'_>) -> IResult<Input<'_>, RootNamespace> {
    let (input, _) = ws_and_comments(input)?;
    let (input, elements) = many0(preceded(ws_and_comments, root_element)).parse(input)?;
    let (input, _) = ws_and_comments(input)?;
    Ok((input, RootNamespace { elements }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom_locate::LocatedSpan;

    #[test]
    fn kitchen_timer_display_tail_parses_as_package_body_element() {
        let input = include_str!("../../tests/fixtures/KitchenTimer.sysml")
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        let start = input
            .find("\tpart def Display {")
            .expect("fixture should contain Display part");
        let tail = &input.as_bytes()[start..];
        let located = LocatedSpan::new(tail);

        let result = package_body_element(located);
        assert!(
            result.is_ok(),
            "package_body_element should parse Display tail, got {:?}",
            result
        );
    }

    #[test]
    fn kitchen_timer_display_tail_parses_as_part_directly() {
        let input = include_str!("../../tests/fixtures/KitchenTimer.sysml")
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        let start = input
            .find("\tpart def Display {")
            .expect("fixture should contain Display part");
        let tail = &input.as_bytes()[start..];
        let located = LocatedSpan::new(tail);
        let (located, _) = ws_and_comments(located).expect("leading ws");

        let result = part_def_or_usage(located);
        assert!(
            result.is_ok(),
            "part_def_or_usage should parse Display tail directly, got {:?}",
            result
        );
    }
}
