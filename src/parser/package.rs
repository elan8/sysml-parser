//! Package and root namespace parsing.

use crate::ast::{
    ClassifierDecl, ExtendedLibraryDecl, FeatureDecl, FilterMember, KermlFeatureDecl,
    KermlSemanticDecl, LibraryPackage, NamespaceDecl, Node, Package, PackageBody,
    PackageBodyElement, RootElement, RootNamespace, Visibility,
};
use crate::parser::action::{action_def, action_usage};
use crate::parser::alias::alias_def;
use crate::parser::allocation::{allocate_usage, allocation_def, allocation_usage};
use crate::parser::attribute::attribute_def;
use crate::parser::build_recovery_error_node_from_span;
use crate::parser::case::{
    analysis_case_def, analysis_case_usage, case_def, case_usage, verification_case_def,
    verification_case_usage,
};
use crate::parser::connection::connection_def;
use crate::parser::constraint::{calc_def, constraint_def};
use crate::parser::dependency::dependency;
use crate::parser::enumeration::enum_def;
use crate::parser::expr::expression;
use crate::parser::flow::{flow_def, flow_usage};
use crate::parser::import::import_;
use crate::parser::individual::individual_def;
use crate::parser::interface::interface_def;
use crate::parser::item::item_def;
use crate::parser::lex::{
    identification, recover_body_element, skip_statement_or_block, starts_with_any_keyword,
    starts_with_keyword, ws1, ws_and_comments, PACKAGE_BODY_STARTERS,
};
use crate::parser::metadata::metadata_def;
use crate::parser::node_from_to;
use crate::parser::occurrence::{
    individual_usage, occurrence_def, occurrence_usage, snapshot_usage, timeslice_usage,
};
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
use nom::IResult;
use nom::Parser;

/// Keyword "package" with following whitespace.
fn keyword_package(input: Input<'_>) -> IResult<Input<'_>, ()> {
    let (input, _) = tag(&b"package"[..]).parse(input)?;
    let (input, _) = ws1(input)?;
    Ok((input, ()))
}

fn required_identification(input: Input<'_>) -> IResult<Input<'_>, crate::ast::Identification> {
    let (input, identification) = identification(input)?;
    if identification.short_name.is_some() || identification.name.is_some() {
        Ok((input, identification))
    } else {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )))
    }
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
    let (input, identification) = required_identification(input)?;
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
    let (input, identification) = required_identification(input)?;
    let (input, body) = package_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            Package {
                identification,
                body,
            },
        ),
    ))
}

/// KerML namespace Identification NamespaceBody
fn namespace_decl(input: Input<'_>) -> IResult<Input<'_>, Node<NamespaceDecl>> {
    let start = input;
    let (input, _) = preceded(ws_and_comments, tag(&b"namespace"[..])).parse(input)?;
    let (input, _) = ws1(input)?;
    let (input, identification) = required_identification(input)?;
    let (input, body) = package_body(input)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            NamespaceDecl {
                identification,
                body,
            },
        ),
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
        map(preceded(ws_and_comments, tag(&b";"[..])), |_| {
            PackageBody::Semicolon
        }),
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

fn modeled_decl_text(start: Input<'_>, end: Input<'_>) -> String {
    let delta = end
        .location_offset()
        .saturating_sub(start.location_offset());
    let bytes = start.fragment();
    let take = delta.min(bytes.len());
    String::from_utf8_lossy(&bytes[..take]).trim().to_string()
}

fn starts_with_visibility_prefix(fragment: &[u8]) -> Option<usize> {
    for prefix in [
        b"public".as_slice(),
        b"private".as_slice(),
        b"protected".as_slice(),
    ] {
        if starts_with_keyword(fragment, prefix) {
            return Some(prefix.len());
        }
    }
    None
}

fn strip_common_decl_prefixes(fragment: &[u8]) -> &[u8] {
    let mut frag = fragment;
    if let Some(len) = starts_with_visibility_prefix(frag) {
        frag = &frag[len..];
        let mut i = 0usize;
        while i < frag.len() && frag[i].is_ascii_whitespace() {
            i += 1;
        }
        frag = &frag[i..];
    }
    if starts_with_keyword(frag, b"abstract") || starts_with_keyword(frag, b"variation") {
        let cut = if starts_with_keyword(frag, b"abstract") {
            8
        } else {
            9
        };
        frag = &frag[cut..];
        let mut i = 0usize;
        while i < frag.len() && frag[i].is_ascii_whitespace() {
            i += 1;
        }
        frag = &frag[i..];
    }
    frag
}

fn is_modeled_decl_start(fragment: &[u8], starters: &[&[u8]]) -> bool {
    if fragment.starts_with(b"#") {
        return false;
    }
    if starts_with_keyword(fragment, b"package")
        || starts_with_keyword(fragment, b"library")
        || starts_with_keyword(fragment, b"namespace")
        || starts_with_keyword(fragment, b"import")
        || starts_with_keyword(fragment, b"doc")
        || starts_with_keyword(fragment, b"comment")
        || starts_with_keyword(fragment, b"filter")
    {
        return false;
    }
    let frag = strip_common_decl_prefixes(fragment);
    starts_with_any_keyword(frag, starters)
}

fn parse_modeled_decl<'a>(
    input: Input<'a>,
    starters: &'a [&'a [u8]],
) -> IResult<Input<'a>, (String, String)> {
    let (input, _) = ws_and_comments(input)?;
    if input.fragment().is_empty() || input.fragment().starts_with(b"}") {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }
    if !is_modeled_decl_start(input.fragment(), starters) {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }
    let raw_start = input;
    let stripped = strip_common_decl_prefixes(input.fragment());
    let bnf_production = starters
        .iter()
        .find(|kw| starts_with_keyword(stripped, kw))
        .map(|kw| String::from_utf8_lossy(kw).to_string())
        .unwrap_or_else(|| "declaration".to_string());
    let (input, _) = skip_statement_or_block(input)?;
    Ok((input, (bnf_production, modeled_decl_text(raw_start, input))))
}

fn kerml_semantic_decl(input: Input<'_>) -> IResult<Input<'_>, Node<KermlSemanticDecl>> {
    let start = input;
    let starters: &[&[u8]] = &[
        b"behavior",
        b"bool",
        b"function",
        b"interaction",
        b"datatype",
        b"inv",
        b"multiplicity",
        b"assoc",
        b"metaclass",
        b"step",
    ];
    let (input, (bnf_production, text)) = parse_modeled_decl(input, starters)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            KermlSemanticDecl {
                bnf_production,
                text,
            },
        ),
    ))
}

fn kerml_feature_decl(input: Input<'_>) -> IResult<Input<'_>, Node<KermlFeatureDecl>> {
    let start = input;
    let starters: &[&[u8]] = &[b"occurrence", b"expr", b"predicate", b"succession"];
    let (input, (bnf_production, text)) = parse_modeled_decl(input, starters)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            KermlFeatureDecl {
                bnf_production,
                text,
            },
        ),
    ))
}

fn feature_decl(input: Input<'_>) -> IResult<Input<'_>, Node<FeatureDecl>> {
    let start = input;
    let starters: &[&[u8]] = &[b"feature"];
    let (input, (keyword, text)) = parse_modeled_decl(input, starters)?;
    Ok((
        input,
        node_from_to(start, input, FeatureDecl { keyword, text }),
    ))
}

fn classifier_decl(input: Input<'_>) -> IResult<Input<'_>, Node<ClassifierDecl>> {
    let start = input;
    let starters: &[&[u8]] = &[b"class", b"classifier", b"struct", b"subclassifier"];
    let (input, (keyword, text)) = parse_modeled_decl(input, starters)?;
    Ok((
        input,
        node_from_to(start, input, ClassifierDecl { keyword, text }),
    ))
}

fn extended_library_decl(input: Input<'_>) -> IResult<Input<'_>, Node<ExtendedLibraryDecl>> {
    let start = input;
    let starters: &[&[u8]] = &[
        b"action",
        b"allocation",
        b"analysis",
        b"attribute",
        b"case",
        b"calc",
        b"connection",
        b"constraint",
        b"flow",
        b"interface",
        b"item",
        b"metadata",
        b"requirement",
        b"state",
        b"use",
        b"verification",
        b"view",
        b"viewpoint",
        b"rendering",
        b"enum",
        b"message",
        b"concern",
        b"part",
        b"port",
    ];
    let (input, (bnf_production, text)) = parse_modeled_decl(input, starters)?;
    Ok((
        input,
        node_from_to(
            start,
            input,
            ExtendedLibraryDecl {
                bnf_production,
                text,
            },
        ),
    ))
}

fn package_body_brace(input: Input<'_>) -> IResult<Input<'_>, PackageBody> {
    let (mut input, _) = preceded(ws_and_comments, tag(&b"{"[..])).parse(input)?;
    let mut elements = Vec::new();
    loop {
        let (next, _) = ws_and_comments(input)?;
        input = next;
        if input.fragment().is_empty() {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Eof,
            )));
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
            Err(_)
                if starts_with_any_keyword(input.fragment(), PACKAGE_BODY_STARTERS)
                    || starts_with_any_keyword(
                        strip_common_decl_prefixes(input.fragment()),
                        PACKAGE_BODY_STARTERS,
                    ) =>
            {
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
                // If we couldn't parse a dedicated node but the line still looks like a modeled
                // library declaration (including `abstract`/visibility prefixes), preserve it as
                // an `ExtendedLibraryDecl` instead of aborting the entire package.
                if let Ok((next, ext)) = map(
                    extended_library_decl,
                    PackageBodyElement::ExtendedLibraryDecl,
                )
                .parse(input)
                {
                    if next.location_offset() == input.location_offset() {
                        return Err(nom::Err::Failure(nom::error::Error::new(
                            input,
                            nom::error::ErrorKind::Many0,
                        )));
                    }
                    elements.push(node_from_to(input, next, ext));
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
                        build_recovery_error_node_from_span(
                            input,
                            next,
                            PACKAGE_BODY_STARTERS,
                            "package body",
                            "recovered_package_body_element",
                        ),
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
        map(preceded(tag(&b"protected"[..]), ws1), |_| {
            Visibility::Protected
        }),
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
pub(crate) fn package_body_element(
    input: Input<'_>,
) -> IResult<Input<'_>, Node<PackageBodyElement>> {
    let (input, _) = ws_and_comments(input)?;
    let start = input;
    if let Ok((input, elem)) = map(doc_comment, PackageBodyElement::Doc).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(comment_annotation, PackageBodyElement::Comment).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) =
        map(textual_representation, PackageBodyElement::TextualRep).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(filter_member, PackageBodyElement::Filter).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(attribute_def, PackageBodyElement::AttributeDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) =
        map(library_package_, PackageBodyElement::LibraryPackage).parse(input)
    {
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
    if let Ok((input, elem)) =
        map(occurrence_usage, PackageBodyElement::OccurrenceUsage).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) =
        map(individual_usage, PackageBodyElement::OccurrenceUsage).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(snapshot_usage, PackageBodyElement::OccurrenceUsage).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) =
        map(timeslice_usage, PackageBodyElement::OccurrenceUsage).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(allocation_def, PackageBodyElement::AllocationDef).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) =
        map(allocation_usage, PackageBodyElement::AllocationUsage).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(allocate_usage, PackageBodyElement::AllocationUsage).parse(input)
    {
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
    if let Ok((input, elem)) = map(requirement_def, PackageBodyElement::RequirementDef).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) =
        map(requirement_usage, PackageBodyElement::RequirementUsage).parse(input)
    {
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
    if let Ok((input, elem)) =
        map(analysis_case_def, PackageBodyElement::AnalysisCaseDef).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) =
        map(analysis_case_usage, PackageBodyElement::AnalysisCaseUsage).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(
        verification_case_def,
        PackageBodyElement::VerificationCaseDef,
    )
    .parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(
        verification_case_usage,
        PackageBodyElement::VerificationCaseUsage,
    )
    .parse(input)
    {
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
    if let Ok((input, elem)) = map(viewpoint_usage, PackageBodyElement::ViewpointUsage).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(rendering_usage, PackageBodyElement::RenderingUsage).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(feature_decl, PackageBodyElement::FeatureDecl).parse(input) {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) = map(classifier_decl, PackageBodyElement::ClassifierDecl).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) =
        map(kerml_semantic_decl, PackageBodyElement::KermlSemanticDecl).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    if let Ok((input, elem)) =
        map(kerml_feature_decl, PackageBodyElement::KermlFeatureDecl).parse(input)
    {
        return Ok((input, node_from_to(start, input, elem)));
    }
    let (input, elem) = map(
        extended_library_decl,
        PackageBodyElement::ExtendedLibraryDecl,
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use nom_locate::LocatedSpan;
    use std::path::PathBuf;

    fn sysml_v2_release_root() -> PathBuf {
        std::env::var_os("SYSML_V2_RELEASE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sysml-v2-release"))
    }

    fn primitive_data_types_fixture() -> Option<String> {
        let path = sysml_v2_release_root()
            .join("sysml")
            .join("src")
            .join("validation")
            .join("15-Properties-Values-Expressions")
            .join("15_10-Primitive Data Types.sysml");
        std::fs::read_to_string(path).ok()
    }

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

    #[test]
    fn primitive_data_types_validation_fixture_package_parses_directly() {
        let Some(input) = primitive_data_types_fixture() else {
            return;
        };
        let located = LocatedSpan::new(input.as_bytes());
        let result = package_(located);
        assert!(
            result.is_ok(),
            "package_ should parse fixture, got {:?}",
            result
        );
    }

    #[test]
    fn primitive_data_types_validation_fixture_package_body_parses_directly() {
        let Some(input) = primitive_data_types_fixture() else {
            return;
        };
        let start = input
            .find('{')
            .expect("fixture should contain package body");
        let located = LocatedSpan::new(&input.as_bytes()[start..]);
        let result = package_body_brace(located);
        assert!(
            result.is_ok(),
            "package_body_brace should parse fixture body, got {:?}",
            result
        );
    }
}
