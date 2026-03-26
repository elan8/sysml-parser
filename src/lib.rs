//! SysML v2 textual notation parser.
//!
//! Reusable library for parsing SysML v2 textual syntax into an AST.

pub mod ast;
pub mod error;
pub mod parser;

pub use ast::{
    AllocationDef, AllocationUsage,
    ActionDef, ActionDefBody, ActionDefBodyElement, ActionUsage, ActionUsageBody, ActionUsageBodyElement, AliasBody,
    AliasDef, AnalysisCaseDef, AnalysisCaseUsage, AstNode, AttributeBody, AttributeDef, AttributeUsage, Bind, CaseDef,
    CaseUsage, Connect, ConnectBody,
    CommentAnnotation, ConnectStmt, DocComment, EndDecl, Expression, FilterMember, FilterPackageMember, FirstMergeBody, FirstStmt, Flow,
    FlowDef, FlowUsage,
    Identification, InOut,
    InOutDecl, InterfaceDef, InterfaceDefBody, InterfaceDefBodyElement, InterfaceUsage,
    InterfaceUsageBodyElement, Import, MergeStmt, NamespaceDecl, Node, Package, PackageBody, PackageBodyElement,
    OccurrenceUsage,
    ParseErrorNode,
    PartDef, PartDefBody, PartDefBodyElement, PartUsage, PartUsageBody, PartUsageBodyElement,
    Perform, PerformBody, PerformBodyElement, PerformInOutBinding, PortBody, PortDef, PortDefBody, PortDefBodyElement,
    PortUsage, RefBody, RefDecl, RootElement, RootNamespace, Span, TextualRepresentation, VerificationCaseDef,
    VerificationCaseUsage, Visibility,
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
