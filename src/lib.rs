//! SysML v2 textual notation parser.
//!
//! Reusable library for parsing SysML v2 textual syntax into an AST.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod ast;
pub mod error;
pub mod parser;

pub use ast::{
    ActionDef, ActionDefBody, ActionDefBodyElement, ActionUsage, ActionUsageBody,
    ActionUsageBodyElement, AliasBody, AliasDef, AllocationDef, AllocationUsage, AnalysisCaseDef,
    AnalysisCaseUsage, Annotation, AstNode, AttributeBody, AttributeDef, AttributeUsage, Bind,
    CaseDef, CaseUsage, CommentAnnotation, Connect, ConnectBody, ConnectStmt, ConnectionDef,
    ConnectionDefBody, ConnectionDefBodyElement, DocComment, EndDecl, Expression, FilterMember,
    FilterPackageMember, FirstMergeBody, FirstStmt, Flow, FlowDef, FlowUsage, Identification,
    Import, InOut, InOutDecl, InterfaceDef, InterfaceDefBody, InterfaceDefBodyElement,
    InterfaceUsage, InterfaceUsageBodyElement, MergeStmt, NamespaceDecl, Node,
    OccurrenceBodyElement, OccurrenceUsage, OccurrenceUsageBody, Package, PackageBody,
    PackageBodyElement, ParseErrorNode, PartDef, PartDefBody, PartDefBodyElement, PartUsage,
    PartUsageBody, PartUsageBodyElement, Perform, PerformBody, PerformBodyElement,
    PerformInOutBinding, PortBody, PortDef, PortDefBody, PortDefBodyElement, PortUsage, RefBody,
    RefDecl, RequireConstraint, RequireConstraintBody, RequirementDef, RequirementDefBody,
    RequirementDefBodyElement, RequirementUsage, RootElement, RootNamespace, Span,
    TextualRepresentation, VerificationCaseDef, VerificationCaseUsage, Visibility,
};
pub use error::{DiagnosticSeverity, ParseError};
pub use parser::{parse_root, parse_with_diagnostics, ParseResult};

/// Parse a SysML v2 textual input into a root namespace AST.
///
/// Returns an error if the input is not valid SysML or if not all input is consumed.
#[allow(clippy::result_large_err)]
pub fn parse(input: &str) -> Result<RootNamespace, ParseError> {
    parse_root(input)
}

/// Parse for editor/LSP use: returns a partial AST plus diagnostics.
///
/// Prefer this over [`parse_root`] when you want IDE features (outline/hover/semantic tokens) to
/// keep working even when the file contains syntax errors.
pub fn parse_for_editor(input: &str) -> ParseResult {
    parse_with_diagnostics(input)
}
