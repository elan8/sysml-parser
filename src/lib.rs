//! SysML v2 textual notation parser.
//!
//! Reusable library for parsing SysML v2 textual syntax into an AST.

pub mod ast;
pub mod error;
pub mod parser;

pub use ast::{
    ActionDef, ActionDefBody, ActionUsage, ActionUsageBody, ActionUsageBodyElement, AliasBody,
    AliasDef, AstNode, AttributeBody, AttributeDef, AttributeUsage, Bind, Connect, ConnectBody,
    ConnectStmt, EndDecl, Expression, FirstMergeBody, FirstStmt, Flow, Identification, InOut,
    InOutDecl, InterfaceDef, InterfaceDefBody, InterfaceDefBodyElement, InterfaceUsage,
    InterfaceUsageBodyElement, Import, MergeStmt, Node, Package, PackageBody, PackageBodyElement,
    PartDef, PartDefBody, PartDefBodyElement, PartUsage, PartUsageBody, PartUsageBodyElement,
    Perform, PerformBody, PerformBodyElement, PortBody, PortDef, PortDefBody, PortDefBodyElement,
    PortUsage, RefBody, RefDecl, RootNamespace, Span, Visibility,
};
pub use error::ParseError;
pub use parser::parse_root;

/// Parse a SysML v2 textual input into a root namespace AST.
///
/// Returns an error if the input is not valid SysML or if not all input is consumed.
pub fn parse(input: &str) -> Result<RootNamespace, ParseError> {
    parse_root(input)
}
