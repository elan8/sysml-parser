//! Abstract syntax tree types for SysML v2 textual notation.

/// Source location: byte offset, line, column, and length in the source file.
/// Line and column are **1-based**. Use [`Span::to_lsp_range`] for 0-based LSP ranges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub offset: usize,
    pub line: u32,
    pub column: usize,
    pub len: usize,
}

impl Span {
    /// Dummy span for tests or synthetic nodes (offset 0, line 1, column 1, len 0).
    pub fn dummy() -> Self {
        Self {
            offset: 0,
            line: 1,
            column: 1,
            len: 0,
        }
    }

    /// LSP uses 0-based line and 0-based character. Returns (start_line, start_character, end_line, end_character).
    pub fn to_lsp_range(&self) -> (u32, u32, u32, u32) {
        let start_line = self.line.saturating_sub(1);
        let start_char = self.column.saturating_sub(1);
        let end_char = start_char.saturating_add(self.len);
        (start_line, start_char as u32, start_line, end_char as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::Span;

    #[test]
    fn span_dummy() {
        let s = Span::dummy();
        assert_eq!(s.offset, 0);
        assert_eq!(s.line, 1);
        assert_eq!(s.column, 1);
        assert_eq!(s.len, 0);
    }
}

#[derive(Debug, Clone)]
pub struct Node<T> {
    pub span: Span,
    pub value: T,
}

impl<T: PartialEq> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Eq> Eq for Node<T> {}

impl<T> Node<T> {
    pub fn new(span: Span, value: T) -> Self {
        Self { span, value }
    }
}

impl<T> std::ops::Deref for Node<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

/// Trait for generic access to node source span (e.g. visitors).
pub trait AstNode {
    fn span(&self) -> Span;
}

impl<T> AstNode for Node<T> {
    fn span(&self) -> Span {
        self.span.clone()
    }
}

/// Expression: literals, feature refs, member access, index, bracket/unit, etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    LiteralInteger(i64),
    LiteralReal(String),
    LiteralString(String),
    LiteralBoolean(bool),
    /// Single name or qualified name.
    FeatureRef(String),
    /// base.member (e.g. engine.fuelCmdPort).
    MemberAccess(Box<Node<Expression>>, String),
    /// base#(index) e.g. frontWheel#(1).
    Index {
        base: Box<Node<Expression>>,
        index: Box<Node<Expression>>,
    },
    /// [unit] e.g. [kg].
    Bracket(Box<Node<Expression>>),
    /// value [unit] e.g. 1750 [kg].
    LiteralWithUnit {
        value: Box<Node<Expression>>,
        unit: Box<Node<Expression>>,
    },
    /// Binary infix operation e.g. `a >= b * c`, `x / y`.
    BinaryOp {
        op: String,
        left: Box<Node<Expression>>,
        right: Box<Node<Expression>>,
    },
    /// Unary prefix: + - ~ not
    UnaryOp {
        op: String,
        operand: Box<Node<Expression>>,
    },
    /// KerML null or empty sequence ().
    Null,
}

/// KerML top-level element: package, namespace, import, or library package (BNF RootNamespace = PackageBodyElement*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootElement {
    Package(Node<Package>),
    LibraryPackage(Node<LibraryPackage>),
    Namespace(Node<NamespaceDecl>),
    Import(Node<Import>),
}

/// KerML NamespaceDeclaration: `namespace` Identification NamespaceBody (same body structure as Package).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceDecl {
    pub identification: Identification,
    pub body: PackageBody,
}

/// Root of a SysML/KerML document: a sequence of top-level package or namespace elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootNamespace {
    pub elements: Vec<Node<RootElement>>,
}

/// KerML ElementFilterMember: MemberPrefix? 'filter' condition ';'
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterMember {
    pub visibility: Option<Visibility>,
    pub condition: Node<Expression>,
}

/// Placeholder node inserted when resilient parsing skips malformed input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseErrorNode {
    pub message: String,
    pub code: String,
    pub expected: Option<String>,
    pub found: Option<String>,
    pub suggestion: Option<String>,
}

/// Modeled KerML semantic declaration captured as package-level syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KermlSemanticDecl {
    pub bnf_production: String,
    pub text: String,
}

/// Modeled KerML feature declaration family (occurrence/expr/predicate/succession).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KermlFeatureDecl {
    pub bnf_production: String,
    pub text: String,
}

/// Modeled extended SysML/KerML declaration family not yet represented by
/// dedicated concrete nodes (e.g. concern/message style library declarations).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedLibraryDecl {
    pub bnf_production: String,
    pub text: String,
}

/// Top-level element inside a namespace or package body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageBodyElement {
    Error(Node<ParseErrorNode>),
    Doc(Node<DocComment>),
    Comment(Node<CommentAnnotation>),
    TextualRep(Node<TextualRepresentation>),
    Filter(Node<FilterMember>),
    Package(Node<Package>),
    LibraryPackage(Node<LibraryPackage>),
    Import(Node<Import>),
    PartDef(Node<PartDef>),
    PartUsage(Node<PartUsage>),
    PortDef(Node<PortDef>),
    InterfaceDef(Node<InterfaceDef>),
    AliasDef(Node<AliasDef>),
    AttributeDef(Node<AttributeDef>),
    ActionDef(Node<ActionDef>),
    ActionUsage(Node<ActionUsage>),
    RequirementDef(Node<RequirementDef>),
    RequirementUsage(Node<RequirementUsage>),
    Satisfy(Node<Satisfy>),
    UseCaseDef(Node<UseCaseDef>),
    Actor(Node<ActorDecl>),
    StateDef(Node<StateDef>),
    StateUsage(Node<StateUsage>),
    ItemDef(Node<ItemDef>),
    IndividualDef(Node<IndividualDef>),
    ConstraintDef(Node<ConstraintDef>),
    CalcDef(Node<CalcDef>),
    ViewDef(Node<ViewDef>),
    ViewpointDef(Node<ViewpointDef>),
    RenderingDef(Node<RenderingDef>),
    ViewUsage(Node<ViewUsage>),
    ViewpointUsage(Node<ViewpointUsage>),
    RenderingUsage(Node<RenderingUsage>),
    ConnectionDef(Node<ConnectionDef>),
    MetadataDef(Node<MetadataDef>),
    EnumDef(Node<EnumDef>),
    OccurrenceDef(Node<OccurrenceDef>),
    OccurrenceUsage(Node<OccurrenceUsage>),
    Dependency(Node<Dependency>),
    AllocationDef(Node<AllocationDef>),
    AllocationUsage(Node<AllocationUsage>),
    FlowDef(Node<FlowDef>),
    FlowUsage(Node<FlowUsage>),
    ConcernUsage(Node<ConcernUsage>),
    CaseDef(Node<CaseDef>),
    CaseUsage(Node<CaseUsage>),
    AnalysisCaseDef(Node<AnalysisCaseDef>),
    AnalysisCaseUsage(Node<AnalysisCaseUsage>),
    VerificationCaseDef(Node<VerificationCaseDef>),
    VerificationCaseUsage(Node<VerificationCaseUsage>),
    UseCaseUsage(Node<UseCaseUsage>),
    KermlSemanticDecl(Node<KermlSemanticDecl>),
    KermlFeatureDecl(Node<KermlFeatureDecl>),
    ExtendedLibraryDecl(Node<ExtendedLibraryDecl>),
}

/// A package declaration: `package` Identification PackageBody
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub identification: Identification,
    pub body: PackageBody,
}

/// Identification: optional short name in `< >`, optional name.
/// BNF: ( '<' declaredShortName = NAME '>' )? ( declaredName = NAME )?
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identification {
    /// Short name inside `< ... >`, if present.
    pub short_name: Option<String>,
    /// Main declared name (may be quoted, e.g. '1a-Parts Tree').
    pub name: Option<String>,
}

/// Package body: either `;` or `{` PackageBodyElement* `}`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageBody {
    /// Semicolon form: no body elements.
    Semicolon,
    /// Brace form: list of body elements (may be empty).
    Brace {
        elements: Vec<Node<PackageBodyElement>>,
    },
}

/// Visibility for imports and members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Protected,
}

/// KerML FilterPackageMember: `[` OwnedExpression `]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterPackageMember {
    pub expression: Node<Expression>,
}

/// Import: `private`? `import` `all`? QualifiedName (`::` `*`)? or FilterPackage form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub visibility: Option<Visibility>,
    /// Whether this is a namespace import (QualifiedName::* or FilterPackage) or membership import (single QualifiedName).
    pub is_import_all: bool,
    /// Import target, e.g. "SI::kg" or "Definitions::*".
    pub target: String,
    /// KerML: optional recursive import after :: (e.g. QualifiedName::** or QualifiedName::*::**).
    pub is_recursive: bool,
    /// KerML FilterPackage form: one or more `[ expr ]` members. When present, this is a namespace import of a filter package.
    pub filter_members: Option<Vec<Node<FilterPackageMember>>>,
}

/// Part definition: `part def` Identification (`:>` specializes)? Body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartDef {
    /// Optional `abstract` or `variation` prefix (BNF BasicDefinitionPrefix).
    pub definition_prefix: Option<DefinitionPrefix>,
    pub identification: Identification,
    /// Supertype after `:>`, e.g. Some("Axle") for `part def FrontAxle :> Axle`.
    pub specializes: Option<String>,
    /// Span of the `:> <type>` fragment (for semantic tokens), when present.
    pub specializes_span: Option<Span>,
    pub body: PartDefBody,
}

/// BNF BasicDefinitionPrefix: `abstract` | `variation`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionPrefix {
    Abstract,
    Variation,
}

/// Body of a part definition: `;` or `{` PartDefBodyElement* `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartDefBody {
    Semicolon,
    Brace {
        elements: Vec<Node<PartDefBodyElement>>,
    },
}

/// Element inside a part definition body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartDefBodyElement {
    Error(Node<ParseErrorNode>),
    Doc(Node<DocComment>),
    AttributeDef(Node<AttributeDef>),
    AttributeUsage(Node<AttributeUsage>),
    PortUsage(Node<PortUsage>),
    PartUsage(Box<Node<PartUsage>>),
    InterfaceUsage(Node<InterfaceUsage>),
    Connect(Node<Connect>),
    Perform(Node<Perform>),
    Allocate(Node<Allocate>),
    /// `exhibit state` name `:` type (`;` or body).
    ExhibitState(Node<ExhibitState>),
}

/// Exhibit state usage: `exhibit state` name `:` type (`;` or body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExhibitState {
    pub name: String,
    pub type_name: String,
}

/// Attribute definition: `attribute` name (`:>` type)? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeDef {
    pub name: String,
    /// Type after `:>`, e.g. Some("ISQ::mass").
    pub typing: Option<String>,
    pub body: AttributeBody,
    /// Span of the defined name (for semantic tokens).
    pub name_span: Option<Span>,
    /// Span of the type after `:>`, if present (for semantic tokens).
    pub typing_span: Option<Span>,
}

/// Body of an attribute (def or usage): `;` or `{` ... `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeBody {
    Semicolon,
    Brace,
}

/// Item definition: `item def` Identification body (for events, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDef {
    pub identification: Identification,
    pub body: AttributeBody,
}

/// Individual definition: `individual def` Identification `:>` type body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndividualDef {
    pub identification: Identification,
    pub specializes: Option<String>,
    pub body: AttributeBody,
}

/// Part usage: `part` name `:` type multiplicity? `ordered`? (`redefines`|`:>>`)? value? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartUsage {
    pub name: String,
    /// Type after `:`, e.g. "Vehicle", "AxleAssembly".
    pub type_name: String,
    /// Multiplicity, e.g. Some("[2]").
    pub multiplicity: Option<String>,
    pub ordered: bool,
    /// Optional `subsets` feature and value expression.
    pub subsets: Option<(String, Option<Node<Expression>>)>,
    /// Redefines target, e.g. Some("frontAxleAssembly") or Some("vehicle1::mass").
    pub redefines: Option<String>,
    /// Value expression (= expr, default = expr, := expr).
    pub value: Option<Node<Expression>>,
    pub body: PartUsageBody,
    /// Span of the usage name (for semantic tokens).
    pub name_span: Option<Span>,
    /// Span of the type reference after `:` (for semantic tokens).
    pub type_ref_span: Option<Span>,
}

/// Body of a part usage: `;` or `{` PartUsageBodyElement* `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartUsageBody {
    Semicolon,
    Brace {
        elements: Vec<Node<PartUsageBodyElement>>,
    },
}

/// Metadata annotation on usage: `@` Name (`:` Type)? MetadataBody (e.g. `@Security;` or `@Safety{isMandatory = true;}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataAnnotation {
    pub name: String,
    pub type_name: Option<String>,
    pub body: ConnectBody,
}

/// Element inside a part usage body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartUsageBodyElement {
    Error(Node<ParseErrorNode>),
    Doc(Node<DocComment>),
    AttributeUsage(Node<AttributeUsage>),
    PartUsage(Box<Node<PartUsage>>),
    PortUsage(Node<PortUsage>),
    Bind(Node<Bind>),
    /// `ref` name `:` type body (reference binding in part usage).
    Ref(Node<RefDecl>),
    InterfaceUsage(Node<InterfaceUsage>),
    Connect(Node<Connect>),
    Perform(Node<Perform>),
    Allocate(Node<Allocate>),
    Satisfy(Node<Satisfy>),
    StateUsage(Node<StateUsage>),
    MetadataAnnotation(Node<MetadataAnnotation>),
}

/// Enacted performance: `perform` action_path `{` body `}` inside a part usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Perform {
    /// Qualified action name (e.g. "provide power" or "provide power.generate torque").
    pub action_name: String,
    /// Type after `:` in "perform action name : Type" form.
    pub type_name: Option<String>,
    pub body: PerformBody,
}

/// Body of a perform: `;` or `{` PerformBodyElement* `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PerformBody {
    Semicolon,
    Brace {
        elements: Vec<Node<PerformBodyElement>>,
    },
}

/// Element inside a perform body: doc comment or in/out binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PerformBodyElement {
    Doc(Node<DocComment>),
    InOut(Node<PerformInOutBinding>),
}

/// In/out binding inside a perform body: `in` name `=` expr `;` or `out` name `=` expr `;`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformInOutBinding {
    pub direction: InOut,
    pub name: String,
    pub value: Node<Expression>,
}

/// Attribute usage: `attribute` name `redefines`? value? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeUsage {
    pub name: String,
    /// Redefines target, e.g. Some("Vehicle::mass").
    pub redefines: Option<String>,
    /// Value expression.
    pub value: Option<Node<Expression>>,
    pub body: AttributeBody,
    /// Span of the usage name (for semantic tokens).
    pub name_span: Option<Span>,
    /// Span of the redefines target after `redefines`, if present (for semantic tokens).
    pub redefines_span: Option<Span>,
}

// ---------------------------------------------------------------------------
// Port
// ---------------------------------------------------------------------------

/// Port definition: `port def` Identification body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortDef {
    pub identification: Identification,
    /// Supertype after `:>`, e.g. Some("ClutchPort") for `port def ManualClutchPort :> ClutchPort`.
    pub specializes: Option<String>,
    pub body: PortDefBody,
}

/// Body of a port definition: `;` or `{` PortDefBodyElement* `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortDefBody {
    Semicolon,
    Brace {
        elements: Vec<Node<PortDefBodyElement>>,
    },
}

/// Element inside a port definition body (in/out declarations or nested port usages).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortDefBodyElement {
    InOutDecl(Node<InOutDecl>),
    Doc(Node<DocComment>),
    AttributeDef(Node<AttributeDef>),
    AttributeUsage(Node<AttributeUsage>),
    PortUsage(Node<PortUsage>),
}

/// Port usage: `port` name `:` type multiplicity? `:>` subsets? `redefines`? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub multiplicity: Option<String>,
    /// Subsets feature and optional value expression.
    pub subsets: Option<(String, Option<Node<Expression>>)>,
    pub redefines: Option<String>,
    pub body: PortBody,
    /// Span of the usage name (for semantic tokens).
    pub name_span: Option<Span>,
    /// Span of the type reference after `:`, if present (for semantic tokens).
    pub type_ref_span: Option<Span>,
}

/// Body of a port usage: `;` or `{` PortUsage* `}` (nested ports).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortBody {
    Semicolon,
    Brace,
    /// Brace with nested port usages (e.g. port vehicleToRoadPort redefines ... { port left...; port right...; }).
    BraceWithPorts {
        elements: Vec<Node<PortUsage>>,
    },
}

// ---------------------------------------------------------------------------
// Interface
// ---------------------------------------------------------------------------

/// Interface definition: `interface def` Identification body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceDef {
    pub identification: Identification,
    pub body: InterfaceDefBody,
}

/// Body of an interface definition: `;` or `{` InterfaceDefBodyElement* `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceDefBody {
    Semicolon,
    Brace {
        elements: Vec<Node<InterfaceDefBodyElement>>,
    },
}

/// Element inside an interface definition body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceDefBodyElement {
    Doc(Node<DocComment>),
    EndDecl(Node<EndDecl>),
    RefDecl(Node<RefDecl>),
    ConnectStmt(Node<ConnectStmt>),
}

/// End declaration in interface def: `end` name `:` type `;`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndDecl {
    pub name: String,
    pub type_name: String,
    /// Span of the name (for semantic tokens).
    pub name_span: Option<Span>,
    /// Span of the type after `:` (for semantic tokens).
    pub type_ref_span: Option<Span>,
}

/// Ref declaration in interface def: `ref` name `:` type body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefDecl {
    pub name: String,
    pub type_name: String,
    /// Optional binding value: `= expr` (SysML shorthand binding for references).
    pub value: Option<Node<Expression>>,
    pub body: RefBody,
    /// Span of the name (for semantic tokens).
    pub name_span: Option<Span>,
    /// Span of the type after `:` (for semantic tokens).
    pub type_ref_span: Option<Span>,
}

/// Body of a ref declaration: `;` or `{` ... `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefBody {
    Semicolon,
    Brace,
}

// ---------------------------------------------------------------------------
// Connection (Phase 2)
// ---------------------------------------------------------------------------

/// Connection definition: `connection def` Identification body (BNF ConnectionDefinition).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionDef {
    pub identification: Identification,
    pub body: ConnectionDefBody,
}

/// Body of a connection definition: `;` or `{` end/ref/connect* `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionDefBody {
    Semicolon,
    Brace {
        elements: Vec<Node<ConnectionDefBodyElement>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionDefBodyElement {
    EndDecl(Node<EndDecl>),
    RefDecl(Node<RefDecl>),
    ConnectStmt(Node<ConnectStmt>),
}

// ---------------------------------------------------------------------------
// Metadata (Phase 2)
// ---------------------------------------------------------------------------

/// Metadata definition: `metadata def` Identification body (BNF MetadataDefinition).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataDef {
    pub is_abstract: bool,
    pub identification: Identification,
    pub body: DefinitionBody,
}

// ---------------------------------------------------------------------------
// Enumeration (Phase 2)
// ---------------------------------------------------------------------------

/// Enumeration definition: `enum def` Identification EnumerationBody (BNF EnumerationDefinition).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDef {
    pub identification: Identification,
    pub body: EnumerationBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnumerationBody {
    Semicolon,
    Brace { values: Vec<String> },
}

// ---------------------------------------------------------------------------
// Occurrence (Phase 2)
// ---------------------------------------------------------------------------

/// Occurrence definition: `occurrence def` Identification body (BNF OccurrenceDefinition).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OccurrenceDef {
    pub is_abstract: bool,
    pub identification: Identification,
    pub body: DefinitionBody,
}

/// Occurrence usage: `occurrence` name (`:` type)? body, with optional individual/portion modifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OccurrenceUsage {
    pub is_individual: bool,
    pub portion_kind: Option<String>,
    pub name: String,
    pub type_name: Option<String>,
    pub body: DefinitionBody,
}

// ---------------------------------------------------------------------------
// Library Package (Phase 2)
// ---------------------------------------------------------------------------

/// Library package: `library` (optional `standard`) `package` Identification PackageBody (BNF LibraryPackage).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryPackage {
    pub is_standard: bool,
    pub identification: Identification,
    pub body: PackageBody,
}

/// Generic definition body: `;` or `{` ... `}` (skip content for metadata/occurrence).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionBody {
    Semicolon,
    Brace,
}

/// Connect statement in interface def or usage: `connect` from `to` to body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectStmt {
    pub from: Node<Expression>,
    pub to: Node<Expression>,
    pub body: ConnectBody,
}

/// Body of a connect statement: `;` or `{` ... `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectBody {
    Semicolon,
    Brace,
}

// ---------------------------------------------------------------------------
// Part usage body: bind, interface usage, connect
// ---------------------------------------------------------------------------

/// Bind: `bind` left `=` right (`;` or `{ }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bind {
    pub left: Node<Expression>,
    pub right: Node<Expression>,
    /// Optional body after the bind (semicolon or brace); 3a fixture uses `bind x = y { }`.
    pub body: Option<ConnectBody>,
}

/// Interface usage: typed+connect or connection form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceUsage {
    /// `interface` `:Type`? `connect` from `to` to body; optional body with ref redefs.
    TypedConnect {
        interface_type: Option<String>,
        from: Node<Expression>,
        to: Node<Expression>,
        body: ConnectBody,
        body_elements: Vec<Node<InterfaceUsageBodyElement>>,
    },
    /// `interface` from `to` to body.
    Connection {
        from: Node<Expression>,
        to: Node<Expression>,
        body_elements: Vec<Node<InterfaceUsageBodyElement>>,
    },
}

/// Element in interface usage body (e.g. ref redefinition).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceUsageBodyElement {
    /// `ref` `:>>` name `=` value body.
    RefRedef {
        name: String,
        value: Node<Expression>,
        body: RefBody,
    },
}

/// Connect at part usage level: `connect` from `to` to body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connect {
    pub from: Node<Expression>,
    pub to: Node<Expression>,
    pub body: ConnectBody,
}

// ---------------------------------------------------------------------------
// Alias
// ---------------------------------------------------------------------------

/// Alias definition: `alias` Identification `for` qualified_name body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasDef {
    pub identification: Identification,
    pub target: String,
    pub body: AliasBody,
}

/// Body of an alias definition: `;` or `{` ... `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AliasBody {
    Semicolon,
    Brace,
}

// ---------------------------------------------------------------------------
// Action (function-based behavior)
// ---------------------------------------------------------------------------

/// Action definition: `action def` Identification body (in/out params).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionDef {
    pub identification: Identification,
    pub body: ActionDefBody,
}

/// Body of an action definition: `;` or `{` ActionDefBodyElement* `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionDefBody {
    Semicolon,
    Brace {
        elements: Vec<Node<ActionDefBodyElement>>,
    },
}

/// Element inside an action definition body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionDefBodyElement {
    Error(Node<ParseErrorNode>),
    InOutDecl(Node<InOutDecl>),
    Doc(Node<DocComment>),
    Perform(Node<Perform>),
    Bind(Node<Bind>),
    Flow(Node<Flow>),
    FirstStmt(Node<FirstStmt>),
    MergeStmt(Node<MergeStmt>),
    ActionUsage(Box<Node<ActionUsage>>),
}

/// In/out parameter in action def: `in` name `:` type `;` or `out` name `:` type `;`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InOutDecl {
    pub direction: InOut,
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InOut {
    In,
    Out,
}

/// Action usage: `action` name `:` type_name (`accept` param_name `:` param_type)? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionUsage {
    pub name: String,
    pub type_name: String,
    /// For accept form: (param_name, param_type).
    pub accept: Option<(String, String)>,
    pub body: ActionUsageBody,
    /// Span of the usage name (for semantic tokens).
    pub name_span: Option<Span>,
    /// Span of the type reference after `:` (for semantic tokens).
    pub type_ref_span: Option<Span>,
}

/// Body of an action usage: `;` or `{` ActionUsageBodyElement* `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionUsageBody {
    Semicolon,
    Brace {
        elements: Vec<Node<ActionUsageBodyElement>>,
    },
}

/// Element inside an action usage body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionUsageBodyElement {
    Error(Node<ParseErrorNode>),
    InOutDecl(Node<InOutDecl>),
    Bind(Node<Bind>),
    Flow(Node<Flow>),
    FirstStmt(Node<FirstStmt>),
    MergeStmt(Node<MergeStmt>),
    ActionUsage(Box<Node<ActionUsage>>),
}

/// Flow: `flow` from `to` to body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flow {
    pub from: Node<Expression>,
    pub to: Node<Expression>,
    pub body: ConnectBody,
}

/// Flow definition: `flow def` Identification body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowDef {
    pub identification: Identification,
    pub body: DefinitionBody,
}

/// Flow usage: `flow` name (`:` type)? [`from` expr `to` expr]? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub from: Option<Node<Expression>>,
    pub to: Option<Node<Expression>>,
    pub body: DefinitionBody,
}

/// First/then control flow: `first` expr `then` expr body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstStmt {
    pub first: Node<Expression>,
    pub then: Node<Expression>,
    pub body: FirstMergeBody,
}

/// Merge: `merge` expr body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeStmt {
    pub merge: Node<Expression>,
    pub body: FirstMergeBody,
}

/// Body of first/merge: `;` or `{` ... `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstMergeBody {
    Semicolon,
    Brace,
}

// ---------------------------------------------------------------------------
// Allocation
// ---------------------------------------------------------------------------

/// Allocate statement at part usage level: `allocate` from `to` to body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allocate {
    pub source: Node<Expression>,
    pub target: Node<Expression>,
    pub body: ConnectBody,
}

/// Allocation definition: `allocation def` Identification body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationDef {
    pub identification: Identification,
    pub body: DefinitionBody,
}

/// Allocation usage: `allocation` name (`:` type)? [`allocate` source `to` target]? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub source: Option<Node<Expression>>,
    pub target: Option<Node<Expression>>,
    pub body: DefinitionBody,
}

// ---------------------------------------------------------------------------
// Requirements
// ---------------------------------------------------------------------------

/// Requirement definition: `requirement def` Identification body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementDef {
    pub identification: Identification,
    pub body: RequirementDefBody,
}

/// Body of an requirement definition: `;` or `{` RequirementDefBodyElement* `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequirementDefBody {
    Semicolon,
    Brace {
        elements: Vec<Node<RequirementDefBodyElement>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequirementDefBodyElement {
    Error(Node<ParseErrorNode>),
    Import(Node<Import>),
    SubjectDecl(Node<SubjectDecl>),
    RequireConstraint(Node<RequireConstraint>),
    Frame(Node<FrameMember>),
    Doc(Node<DocComment>),
}

/// Subject declaration: `subject` name `:` type `;`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectDecl {
    pub name: String,
    pub type_name: String,
}

/// Require constraint: `require constraint { ... }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequireConstraint {
    pub body: ConstraintBody,
}

/// Requirement usage / Satisfy. Example: `satisfy EnduranceReq by droneInstance;`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Satisfy {
    pub source: Node<Expression>,
    pub target: Node<Expression>,
    pub body: ConnectBody,
}

/// Bare requirement Usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub body: RequirementDefBody,
}

/// Dependency: `dependency` (Identification `from`)? client(s) `to` supplier(s) RelationshipBody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub identification: Option<Identification>,
    pub clients: Vec<String>,
    pub suppliers: Vec<String>,
    pub body: ConnectBody,
}

/// Framed concern member in requirement body: `frame` name (`;` or body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameMember {
    pub name: String,
    pub body: RequirementDefBody,
}

/// Concern usage at package level: `concern` name (`:` type)? RequirementBody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcernUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub body: RequirementDefBody,
}

/// Case definition: `case def` Identification body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseDef {
    pub identification: Identification,
    pub body: UseCaseDefBody,
}

/// Case usage: `case` name (`:` type)? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub body: UseCaseDefBody,
}

/// Analysis case definition: `analysis def` Identification body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisCaseDef {
    pub identification: Identification,
    pub body: UseCaseDefBody,
}

/// Analysis case usage: `analysis` name (`:` type)? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisCaseUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub body: UseCaseDefBody,
}

/// Verification case definition: `verification def` Identification body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationCaseDef {
    pub identification: Identification,
    pub body: UseCaseDefBody,
}

/// Verification case usage: `verification` name (`:` type)? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationCaseUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub body: UseCaseDefBody,
}

/// Use case usage at package level: `use case` name (`:` type)? CaseBody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseCaseUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub body: UseCaseDefBody,
}

// ---------------------------------------------------------------------------
// Use Cases
// ---------------------------------------------------------------------------

/// Actor declaration: `actor` Identification `;`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorDecl {
    pub identification: Identification,
}

/// Use Case definition: `use case def` Identification body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseCaseDef {
    pub identification: Identification,
    pub body: UseCaseDefBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UseCaseDefBody {
    Semicolon,
    Brace {
        elements: Vec<Node<UseCaseDefBodyElement>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UseCaseDefBodyElement {
    Error(Node<ParseErrorNode>),
    Doc(Node<DocComment>),
    SubjectDecl(Node<SubjectDecl>),
    ActorUsage(Node<ActorUsage>),
    Objective(Node<Objective>),
}

/// actor usage `actor pilot : Operator;`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorUsage {
    pub name: String,
    pub type_name: String,
}

/// Objective `objective { doc ... }`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Objective {
    pub body: ConstraintBody,
}

// ---------------------------------------------------------------------------
// State Machine
// ---------------------------------------------------------------------------

/// State definition: `state def` Identification body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDef {
    pub identification: Identification,
    pub body: StateDefBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateDefBody {
    Semicolon,
    Brace {
        elements: Vec<Node<StateDefBodyElement>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateDefBodyElement {
    Error(Node<ParseErrorNode>),
    Doc(Node<DocComment>),
    /// `entry` (`;` or body) - entry action.
    Entry(Node<EntryAction>),
    /// `then` name `;` - initial state.
    Then(Node<ThenStmt>),
    /// `ref` name `:` type body – reference binding in state.
    Ref(Node<RefDecl>),
    StateUsage(Node<StateUsage>),
    Transition(Node<Transition>),
}

/// Entry action: `entry` (`;` or body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryAction {
    /// For `entry action name body` form; None for plain `entry` body.
    pub action_name: Option<String>,
    pub body: StateDefBody,
}

/// Then (initial state): `then` name `;`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThenStmt {
    pub state_name: String,
}

/// State usage: `state` name (`:` type)? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub body: StateDefBody,
}

/// Transition: `transition` name [`first` source] [`if` guard] [`do` effect] `then` target body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    pub name: String,
    /// If omitted, form is `transition name then target;`.
    pub source: Option<Node<Expression>>,
    pub guard: Option<Node<Expression>>,
    pub effect: Option<Node<Expression>>,
    pub target: Node<Expression>,
    pub body: ConnectBody,
}

// ---------------------------------------------------------------------------
// Constraints & Calculations
// ---------------------------------------------------------------------------

/// Constraint definition: `constraint def` Identification body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintDef {
    pub identification: Identification,
    pub body: ConstraintDefBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintDefBody {
    Semicolon,
    Brace {
        elements: Vec<Node<ConstraintDefBodyElement>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintDefBodyElement {
    Doc(Node<DocComment>),
    InOutDecl(Node<InOutDecl>),
    Expression(Node<Expression>), // e.g. totalThrust >= totalWeight * margin
}

/// constraint body {}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintBody {
    Semicolon,
    Brace, // Often contains docs or block of expressions
}

/// KerML Documentation: 'doc' Identification? ( 'locale' STRING_VALUE )? body = REGULAR_COMMENT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocComment {
    /// Optional identification after 'doc'.
    pub identification: Option<Identification>,
    /// Optional locale string (e.g. "en").
    pub locale: Option<String>,
    /// Body text (content between /* and */).
    pub text: String,
}

/// KerML Comment: ( 'comment' Identification? )? ( 'locale' STRING_VALUE )? body = REGULAR_COMMENT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentAnnotation {
    pub identification: Option<Identification>,
    pub locale: Option<String>,
    pub text: String,
}

/// KerML TextualRepresentation: ( 'rep' Identification )? 'language' STRING_VALUE body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextualRepresentation {
    pub rep_identification: Option<Identification>,
    pub language: String,
    pub text: String,
}

/// Calc definition: `calc def` Identification body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalcDef {
    pub identification: Identification,
    pub body: CalcDefBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalcDefBody {
    Semicolon,
    Brace {
        elements: Vec<Node<CalcDefBodyElement>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalcDefBodyElement {
    Doc(Node<DocComment>),
    InOutDecl(Node<InOutDecl>),
    ReturnDecl(Node<ReturnDecl>),
    Expression(Node<Expression>), // formula
}

/// Return declaration: `return` name `:` type `;`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnDecl {
    pub name: String,
    pub type_name: String,
}

// ---------------------------------------------------------------------------
// Views and Viewpoints (SysML v2 Clause 8.2.2.26)
// ---------------------------------------------------------------------------

/// View definition: `view def` Identification ViewDefinitionBody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewDef {
    pub identification: Identification,
    pub body: ViewDefBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewDefBody {
    Semicolon,
    Brace {
        elements: Vec<Node<ViewDefBodyElement>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewDefBodyElement {
    Doc(Node<DocComment>),
    Filter(Node<FilterMember>),
    ViewRendering(Node<ViewRenderingUsage>),
}

/// View rendering usage: `render` name `:` type (`;` or body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewRenderingUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub body: ConnectBody,
}

/// Viewpoint definition: `viewpoint def` Identification RequirementBody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewpointDef {
    pub identification: Identification,
    pub body: RequirementDefBody,
}

/// Rendering definition: `rendering def` Definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderingDef {
    pub identification: Identification,
    pub body: RenderingDefBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderingDefBody {
    Semicolon,
    Brace,
}

/// View usage: `view` name `:` type? ViewBody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub body: ViewBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewBody {
    Semicolon,
    Brace {
        elements: Vec<Node<ViewBodyElement>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewBodyElement {
    Doc(Node<DocComment>),
    Filter(Node<FilterMember>),
    ViewRendering(Node<ViewRenderingUsage>),
    Expose(Node<ExposeMember>),
    Satisfy(Node<SatisfyViewMember>),
}

/// Expose in view body: `expose` (MembershipImport | NamespaceImport) RelationshipBody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExposeMember {
    /// Full target path (e.g. vehicle, vehicle::*, vehicle::*::**, SystemModel::vehicle::**).
    pub target: String,
    pub body: ConnectBody,
}

/// Satisfy in view body: `satisfy` QualifiedName RelationshipBody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SatisfyViewMember {
    pub viewpoint_ref: String,
    pub body: ConnectBody,
}

/// Viewpoint usage: `viewpoint` ConstraintUsageDeclaration RequirementBody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewpointUsage {
    pub name: String,
    pub type_name: String,
    pub body: RequirementDefBody,
}

/// Rendering usage: `rendering` Usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderingUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub body: ConnectBody,
}

// ---------------------------------------------------------------------------
// Normalization for test comparison (strips optional spans so parsed == expected)
// ---------------------------------------------------------------------------

impl RootNamespace {
    /// Returns a copy with all optional source spans set to `None` and all `Node` spans set to
    /// `Span::dummy()`. Use when comparing parser output to hand-built expected AST in tests.
    pub fn normalize_for_test_comparison(&self) -> Self {
        RootNamespace {
            elements: self
                .elements
                .iter()
                .map(normalize_root_element_node)
                .collect(),
        }
    }
}

fn dummy_node<T: Clone>(_n: &Node<T>, value: T) -> Node<T> {
    Node::new(Span::dummy(), value)
}

fn normalize_root_element_node(el: &Node<RootElement>) -> Node<RootElement> {
    let value = match &el.value {
        RootElement::Package(p) => RootElement::Package(dummy_node(p, normalize_package(&p.value))),
        RootElement::LibraryPackage(lp) => {
            RootElement::LibraryPackage(dummy_node(lp, normalize_library_package(&lp.value)))
        }
        RootElement::Namespace(n) => {
            RootElement::Namespace(dummy_node(n, normalize_namespace_decl(&n.value)))
        }
        RootElement::Import(n) => RootElement::Import(dummy_node(n, n.value.clone())),
    };
    dummy_node(el, value)
}

fn normalize_library_package(lp: &LibraryPackage) -> LibraryPackage {
    LibraryPackage {
        is_standard: lp.is_standard,
        identification: lp.identification.clone(),
        body: normalize_package_body(&lp.body),
    }
}

fn normalize_namespace_decl(n: &NamespaceDecl) -> NamespaceDecl {
    NamespaceDecl {
        identification: n.identification.clone(),
        body: normalize_package_body(&n.body),
    }
}

fn normalize_package(p: &Package) -> Package {
    Package {
        identification: p.identification.clone(),
        body: normalize_package_body(&p.body),
    }
}

fn normalize_package_body(b: &PackageBody) -> PackageBody {
    match b {
        PackageBody::Semicolon => PackageBody::Semicolon,
        PackageBody::Brace { elements } => PackageBody::Brace {
            elements: elements
                .iter()
                .map(normalize_package_body_element_node)
                .collect(),
        },
    }
}

fn normalize_package_body_element_node(el: &Node<PackageBodyElement>) -> Node<PackageBodyElement> {
    let value = match &el.value {
        PackageBodyElement::Error(n) => PackageBodyElement::Error(dummy_node(n, n.value.clone())),
        PackageBodyElement::Doc(n) => PackageBodyElement::Doc(dummy_node(n, n.value.clone())),
        PackageBodyElement::Comment(n) => {
            PackageBodyElement::Comment(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::TextualRep(n) => {
            PackageBodyElement::TextualRep(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::Filter(n) => PackageBodyElement::Filter(dummy_node(n, n.value.clone())),
        PackageBodyElement::Package(n) => {
            PackageBodyElement::Package(dummy_node(n, normalize_package(&n.value)))
        }
        PackageBodyElement::LibraryPackage(n) => {
            PackageBodyElement::LibraryPackage(dummy_node(n, normalize_library_package(&n.value)))
        }
        PackageBodyElement::Import(n) => PackageBodyElement::Import(dummy_node(n, n.value.clone())),
        PackageBodyElement::PartDef(n) => {
            PackageBodyElement::PartDef(dummy_node(n, normalize_part_def(&n.value)))
        }
        PackageBodyElement::PartUsage(n) => {
            PackageBodyElement::PartUsage(dummy_node(n, normalize_part_usage(&n.value)))
        }
        PackageBodyElement::PortDef(n) => {
            PackageBodyElement::PortDef(dummy_node(n, normalize_port_def(&n.value)))
        }
        PackageBodyElement::InterfaceDef(n) => {
            PackageBodyElement::InterfaceDef(dummy_node(n, normalize_interface_def(&n.value)))
        }
        PackageBodyElement::ConnectionDef(n) => {
            PackageBodyElement::ConnectionDef(dummy_node(n, normalize_connection_def(&n.value)))
        }
        PackageBodyElement::MetadataDef(n) => {
            PackageBodyElement::MetadataDef(dummy_node(n, normalize_metadata_def(&n.value)))
        }
        PackageBodyElement::EnumDef(n) => {
            PackageBodyElement::EnumDef(dummy_node(n, normalize_enum_def(&n.value)))
        }
        PackageBodyElement::OccurrenceDef(n) => {
            PackageBodyElement::OccurrenceDef(dummy_node(n, normalize_occurrence_def(&n.value)))
        }
        PackageBodyElement::OccurrenceUsage(n) => {
            PackageBodyElement::OccurrenceUsage(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::AliasDef(n) => {
            PackageBodyElement::AliasDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::AttributeDef(n) => {
            PackageBodyElement::AttributeDef(dummy_node(n, normalize_attribute_def(&n.value)))
        }
        PackageBodyElement::ActionDef(n) => {
            PackageBodyElement::ActionDef(dummy_node(n, normalize_action_def(&n.value)))
        }
        PackageBodyElement::ActionUsage(n) => {
            PackageBodyElement::ActionUsage(dummy_node(n, normalize_action_usage(&n.value)))
        }
        PackageBodyElement::RequirementDef(n) => {
            PackageBodyElement::RequirementDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::RequirementUsage(n) => {
            PackageBodyElement::RequirementUsage(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::Satisfy(n) => {
            PackageBodyElement::Satisfy(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::UseCaseDef(n) => {
            PackageBodyElement::UseCaseDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::Actor(n) => PackageBodyElement::Actor(dummy_node(n, n.value.clone())),
        PackageBodyElement::StateDef(n) => {
            PackageBodyElement::StateDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::StateUsage(n) => {
            PackageBodyElement::StateUsage(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::ItemDef(n) => {
            PackageBodyElement::ItemDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::IndividualDef(n) => {
            PackageBodyElement::IndividualDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::ConstraintDef(n) => {
            PackageBodyElement::ConstraintDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::CalcDef(n) => {
            PackageBodyElement::CalcDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::ViewDef(n) => {
            PackageBodyElement::ViewDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::ViewpointDef(n) => {
            PackageBodyElement::ViewpointDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::RenderingDef(n) => {
            PackageBodyElement::RenderingDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::ViewUsage(n) => {
            PackageBodyElement::ViewUsage(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::ViewpointUsage(n) => {
            PackageBodyElement::ViewpointUsage(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::RenderingUsage(n) => {
            PackageBodyElement::RenderingUsage(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::Dependency(n) => {
            PackageBodyElement::Dependency(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::AllocationDef(n) => {
            PackageBodyElement::AllocationDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::AllocationUsage(n) => {
            PackageBodyElement::AllocationUsage(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::FlowDef(n) => {
            PackageBodyElement::FlowDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::FlowUsage(n) => {
            PackageBodyElement::FlowUsage(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::ConcernUsage(n) => {
            PackageBodyElement::ConcernUsage(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::CaseDef(n) => {
            PackageBodyElement::CaseDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::CaseUsage(n) => {
            PackageBodyElement::CaseUsage(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::AnalysisCaseDef(n) => {
            PackageBodyElement::AnalysisCaseDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::AnalysisCaseUsage(n) => {
            PackageBodyElement::AnalysisCaseUsage(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::VerificationCaseDef(n) => {
            PackageBodyElement::VerificationCaseDef(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::VerificationCaseUsage(n) => {
            PackageBodyElement::VerificationCaseUsage(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::UseCaseUsage(n) => {
            PackageBodyElement::UseCaseUsage(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::KermlSemanticDecl(n) => {
            PackageBodyElement::KermlSemanticDecl(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::KermlFeatureDecl(n) => {
            PackageBodyElement::KermlFeatureDecl(dummy_node(n, n.value.clone()))
        }
        PackageBodyElement::ExtendedLibraryDecl(n) => {
            PackageBodyElement::ExtendedLibraryDecl(dummy_node(n, n.value.clone()))
        }
    };
    dummy_node(el, value)
}

fn normalize_attribute_def(a: &AttributeDef) -> AttributeDef {
    AttributeDef {
        name: a.name.clone(),
        typing: a.typing.clone(),
        body: a.body.clone(),
        name_span: None,
        typing_span: None,
    }
}

fn normalize_part_def(p: &PartDef) -> PartDef {
    PartDef {
        definition_prefix: p.definition_prefix.clone(),
        identification: p.identification.clone(),
        specializes: p.specializes.clone(),
        specializes_span: None,
        body: normalize_part_def_body(&p.body),
    }
}

fn normalize_part_def_body(b: &PartDefBody) -> PartDefBody {
    match b {
        PartDefBody::Semicolon => PartDefBody::Semicolon,
        PartDefBody::Brace { elements } => PartDefBody::Brace {
            elements: elements
                .iter()
                .map(normalize_part_def_body_element_node)
                .collect(),
        },
    }
}

fn normalize_part_def_body_element_node(el: &Node<PartDefBodyElement>) -> Node<PartDefBodyElement> {
    let value = match &el.value {
        PartDefBodyElement::Error(n) => PartDefBodyElement::Error(dummy_node(n, n.value.clone())),
        PartDefBodyElement::Doc(n) => PartDefBodyElement::Doc(dummy_node(n, n.value.clone())),
        PartDefBodyElement::AttributeDef(n) => {
            PartDefBodyElement::AttributeDef(dummy_node(n, normalize_attribute_def(&n.value)))
        }
        PartDefBodyElement::AttributeUsage(n) => {
            PartDefBodyElement::AttributeUsage(dummy_node(n, normalize_attribute_usage(&n.value)))
        }
        PartDefBodyElement::PortUsage(n) => {
            PartDefBodyElement::PortUsage(dummy_node(n, normalize_port_usage(&n.value)))
        }
        PartDefBodyElement::PartUsage(n) => {
            PartDefBodyElement::PartUsage(Box::new(dummy_node(n, normalize_part_usage(&n.value))))
        }
        PartDefBodyElement::InterfaceUsage(n) => {
            PartDefBodyElement::InterfaceUsage(dummy_node(n, n.value.clone()))
        }
        PartDefBodyElement::Connect(n) => {
            PartDefBodyElement::Connect(dummy_node(n, n.value.clone()))
        }
        PartDefBodyElement::Perform(n) => {
            PartDefBodyElement::Perform(dummy_node(n, n.value.clone()))
        }
        PartDefBodyElement::Allocate(n) => {
            PartDefBodyElement::Allocate(dummy_node(n, n.value.clone()))
        }
        PartDefBodyElement::ExhibitState(n) => {
            PartDefBodyElement::ExhibitState(dummy_node(n, n.value.clone()))
        }
    };
    dummy_node(el, value)
}

fn normalize_attribute_usage(a: &AttributeUsage) -> AttributeUsage {
    AttributeUsage {
        name: a.name.clone(),
        redefines: a.redefines.clone(),
        value: a.value.clone(),
        body: a.body.clone(),
        name_span: None,
        redefines_span: None,
    }
}

fn normalize_part_usage(p: &PartUsage) -> PartUsage {
    PartUsage {
        name: p.name.clone(),
        type_name: p.type_name.clone(),
        multiplicity: p.multiplicity.clone(),
        ordered: p.ordered,
        subsets: p.subsets.clone(),
        redefines: p.redefines.clone(),
        value: p.value.clone(),
        body: normalize_part_usage_body(&p.body),
        name_span: None,
        type_ref_span: None,
    }
}

fn normalize_part_usage_body(b: &PartUsageBody) -> PartUsageBody {
    match b {
        PartUsageBody::Semicolon => PartUsageBody::Semicolon,
        PartUsageBody::Brace { elements } => PartUsageBody::Brace {
            elements: elements
                .iter()
                .map(normalize_part_usage_body_element_node)
                .collect(),
        },
    }
}

fn normalize_perform(p: &Perform) -> Perform {
    Perform {
        action_name: p.action_name.clone(),
        type_name: p.type_name.clone(),
        body: normalize_perform_body(&p.body),
    }
}

fn normalize_perform_body(b: &PerformBody) -> PerformBody {
    match b {
        PerformBody::Semicolon => PerformBody::Semicolon,
        PerformBody::Brace { elements } => PerformBody::Brace {
            elements: elements
                .iter()
                .map(normalize_perform_body_element_node)
                .collect(),
        },
    }
}

fn normalize_perform_body_element_node(el: &Node<PerformBodyElement>) -> Node<PerformBodyElement> {
    let value = match &el.value {
        PerformBodyElement::Doc(n) => PerformBodyElement::Doc(dummy_node(n, n.value.clone())),
        PerformBodyElement::InOut(n) => PerformBodyElement::InOut(dummy_node(
            n,
            PerformInOutBinding {
                direction: n.value.direction,
                name: n.value.name.clone(),
                value: normalize_expression_node(&n.value.value),
            },
        )),
    };
    dummy_node(el, value)
}

fn normalize_expression_node(node: &Node<Expression>) -> Node<Expression> {
    let value = match &node.value {
        Expression::LiteralInteger(x) => Expression::LiteralInteger(*x),
        Expression::LiteralReal(s) => Expression::LiteralReal(s.clone()),
        Expression::LiteralString(s) => Expression::LiteralString(s.clone()),
        Expression::LiteralBoolean(b) => Expression::LiteralBoolean(*b),
        Expression::FeatureRef(s) => Expression::FeatureRef(s.clone()),
        Expression::MemberAccess(base, member) => {
            Expression::MemberAccess(Box::new(normalize_expression_node(base)), member.clone())
        }
        Expression::Index { base, index } => Expression::Index {
            base: Box::new(normalize_expression_node(base)),
            index: Box::new(normalize_expression_node(index)),
        },
        Expression::Bracket(inner) => {
            Expression::Bracket(Box::new(normalize_expression_node(inner)))
        }
        Expression::LiteralWithUnit { value: v, unit } => Expression::LiteralWithUnit {
            value: Box::new(normalize_expression_node(v)),
            unit: Box::new(normalize_expression_node(unit)),
        },
        Expression::BinaryOp { op, left, right } => Expression::BinaryOp {
            op: op.clone(),
            left: Box::new(normalize_expression_node(left)),
            right: Box::new(normalize_expression_node(right)),
        },
        Expression::UnaryOp { op, operand } => Expression::UnaryOp {
            op: op.clone(),
            operand: Box::new(normalize_expression_node(operand)),
        },
        Expression::Null => Expression::Null,
    };
    Node::new(Span::dummy(), value)
}

fn normalize_part_usage_body_element_node(
    el: &Node<PartUsageBodyElement>,
) -> Node<PartUsageBodyElement> {
    let value = match &el.value {
        PartUsageBodyElement::Error(n) => {
            PartUsageBodyElement::Error(dummy_node(n, n.value.clone()))
        }
        PartUsageBodyElement::Doc(n) => PartUsageBodyElement::Doc(dummy_node(n, n.value.clone())),
        PartUsageBodyElement::AttributeUsage(n) => {
            PartUsageBodyElement::AttributeUsage(dummy_node(n, normalize_attribute_usage(&n.value)))
        }
        PartUsageBodyElement::PartUsage(n) => {
            PartUsageBodyElement::PartUsage(Box::new(dummy_node(n, normalize_part_usage(&n.value))))
        }
        PartUsageBodyElement::PortUsage(n) => {
            PartUsageBodyElement::PortUsage(dummy_node(n, normalize_port_usage(&n.value)))
        }
        PartUsageBodyElement::Ref(n) => {
            PartUsageBodyElement::Ref(dummy_node(n, normalize_ref_decl(&n.value)))
        }
        PartUsageBodyElement::Bind(n) => PartUsageBodyElement::Bind(dummy_node(n, n.value.clone())),
        PartUsageBodyElement::InterfaceUsage(n) => {
            PartUsageBodyElement::InterfaceUsage(dummy_node(n, n.value.clone()))
        }
        PartUsageBodyElement::Connect(n) => {
            PartUsageBodyElement::Connect(dummy_node(n, n.value.clone()))
        }
        PartUsageBodyElement::Perform(n) => {
            PartUsageBodyElement::Perform(dummy_node(n, normalize_perform(&n.value)))
        }
        PartUsageBodyElement::Allocate(n) => {
            PartUsageBodyElement::Allocate(dummy_node(n, n.value.clone()))
        }
        PartUsageBodyElement::Satisfy(n) => {
            PartUsageBodyElement::Satisfy(dummy_node(n, n.value.clone()))
        }
        PartUsageBodyElement::StateUsage(n) => {
            PartUsageBodyElement::StateUsage(dummy_node(n, n.value.clone()))
        }
        PartUsageBodyElement::MetadataAnnotation(n) => {
            PartUsageBodyElement::MetadataAnnotation(dummy_node(n, n.value.clone()))
        }
    };
    dummy_node(el, value)
}

fn normalize_port_usage(p: &PortUsage) -> PortUsage {
    PortUsage {
        name: p.name.clone(),
        type_name: p.type_name.clone(),
        multiplicity: p.multiplicity.clone(),
        subsets: p.subsets.clone(),
        redefines: p.redefines.clone(),
        body: normalize_port_body(&p.body),
        name_span: None,
        type_ref_span: None,
    }
}

fn normalize_port_body(b: &PortBody) -> PortBody {
    match b {
        PortBody::Semicolon => PortBody::Semicolon,
        PortBody::Brace => PortBody::Brace,
        PortBody::BraceWithPorts { elements } => PortBody::BraceWithPorts {
            elements: elements
                .iter()
                .map(|n| dummy_node(n, normalize_port_usage(&n.value)))
                .collect(),
        },
    }
}

fn normalize_port_def(p: &PortDef) -> PortDef {
    PortDef {
        identification: p.identification.clone(),
        specializes: p.specializes.clone(),
        body: normalize_port_def_body(&p.body),
    }
}

fn normalize_port_def_body(b: &PortDefBody) -> PortDefBody {
    match b {
        PortDefBody::Semicolon => PortDefBody::Semicolon,
        PortDefBody::Brace { elements } => PortDefBody::Brace {
            elements: elements
                .iter()
                .map(normalize_port_def_body_element_node)
                .collect(),
        },
    }
}

fn normalize_port_def_body_element_node(el: &Node<PortDefBodyElement>) -> Node<PortDefBodyElement> {
    let value = match &el.value {
        PortDefBodyElement::InOutDecl(n) => {
            PortDefBodyElement::InOutDecl(dummy_node(n, n.value.clone()))
        }
        PortDefBodyElement::Doc(n) => PortDefBodyElement::Doc(dummy_node(n, n.value.clone())),
        PortDefBodyElement::AttributeDef(n) => {
            PortDefBodyElement::AttributeDef(dummy_node(n, normalize_attribute_def(&n.value)))
        }
        PortDefBodyElement::AttributeUsage(n) => {
            PortDefBodyElement::AttributeUsage(dummy_node(n, normalize_attribute_usage(&n.value)))
        }
        PortDefBodyElement::PortUsage(n) => {
            PortDefBodyElement::PortUsage(dummy_node(n, normalize_port_usage(&n.value)))
        }
    };
    dummy_node(el, value)
}

fn normalize_interface_def(i: &InterfaceDef) -> InterfaceDef {
    InterfaceDef {
        identification: i.identification.clone(),
        body: normalize_interface_def_body(&i.body),
    }
}

fn normalize_connection_def(c: &ConnectionDef) -> ConnectionDef {
    ConnectionDef {
        identification: c.identification.clone(),
        body: normalize_connection_def_body(&c.body),
    }
}

fn normalize_connection_def_body(b: &ConnectionDefBody) -> ConnectionDefBody {
    match b {
        ConnectionDefBody::Semicolon => ConnectionDefBody::Semicolon,
        ConnectionDefBody::Brace { elements } => ConnectionDefBody::Brace {
            elements: elements
                .iter()
                .map(normalize_connection_def_body_element_node)
                .collect(),
        },
    }
}

fn normalize_connection_def_body_element_node(
    el: &Node<ConnectionDefBodyElement>,
) -> Node<ConnectionDefBodyElement> {
    let value = match &el.value {
        ConnectionDefBodyElement::EndDecl(n) => {
            ConnectionDefBodyElement::EndDecl(dummy_node(n, normalize_end_decl(&n.value)))
        }
        ConnectionDefBodyElement::RefDecl(n) => {
            ConnectionDefBodyElement::RefDecl(dummy_node(n, normalize_ref_decl(&n.value)))
        }
        ConnectionDefBodyElement::ConnectStmt(n) => {
            ConnectionDefBodyElement::ConnectStmt(dummy_node(n, n.value.clone()))
        }
    };
    dummy_node(el, value)
}

fn normalize_metadata_def(m: &MetadataDef) -> MetadataDef {
    MetadataDef {
        is_abstract: m.is_abstract,
        identification: m.identification.clone(),
        body: m.body.clone(),
    }
}

fn normalize_enum_def(e: &EnumDef) -> EnumDef {
    EnumDef {
        identification: e.identification.clone(),
        body: e.body.clone(),
    }
}

fn normalize_occurrence_def(o: &OccurrenceDef) -> OccurrenceDef {
    OccurrenceDef {
        is_abstract: o.is_abstract,
        identification: o.identification.clone(),
        body: o.body.clone(),
    }
}

fn normalize_interface_def_body(b: &InterfaceDefBody) -> InterfaceDefBody {
    match b {
        InterfaceDefBody::Semicolon => InterfaceDefBody::Semicolon,
        InterfaceDefBody::Brace { elements } => InterfaceDefBody::Brace {
            elements: elements
                .iter()
                .map(normalize_interface_def_body_element_node)
                .collect(),
        },
    }
}

fn normalize_interface_def_body_element_node(
    el: &Node<InterfaceDefBodyElement>,
) -> Node<InterfaceDefBodyElement> {
    let value = match &el.value {
        InterfaceDefBodyElement::Doc(n) => {
            InterfaceDefBodyElement::Doc(dummy_node(n, n.value.clone()))
        }
        InterfaceDefBodyElement::EndDecl(n) => {
            InterfaceDefBodyElement::EndDecl(dummy_node(n, normalize_end_decl(&n.value)))
        }
        InterfaceDefBodyElement::RefDecl(n) => {
            InterfaceDefBodyElement::RefDecl(dummy_node(n, normalize_ref_decl(&n.value)))
        }
        InterfaceDefBodyElement::ConnectStmt(n) => {
            InterfaceDefBodyElement::ConnectStmt(dummy_node(n, n.value.clone()))
        }
    };
    dummy_node(el, value)
}

fn normalize_end_decl(e: &EndDecl) -> EndDecl {
    EndDecl {
        name: e.name.clone(),
        type_name: e.type_name.clone(),
        name_span: None,
        type_ref_span: None,
    }
}

fn normalize_ref_decl(r: &RefDecl) -> RefDecl {
    RefDecl {
        name: r.name.clone(),
        type_name: r.type_name.clone(),
        value: r.value.clone(),
        body: r.body.clone(),
        name_span: None,
        type_ref_span: None,
    }
}

fn normalize_action_def(a: &ActionDef) -> ActionDef {
    ActionDef {
        identification: a.identification.clone(),
        body: normalize_action_def_body(&a.body),
    }
}

fn normalize_action_def_body(b: &ActionDefBody) -> ActionDefBody {
    match b {
        ActionDefBody::Semicolon => ActionDefBody::Semicolon,
        ActionDefBody::Brace { elements } => ActionDefBody::Brace {
            elements: elements
                .iter()
                .map(normalize_action_def_body_element_node)
                .collect(),
        },
    }
}

fn normalize_action_def_body_element_node(
    el: &Node<ActionDefBodyElement>,
) -> Node<ActionDefBodyElement> {
    let value = match &el.value {
        ActionDefBodyElement::Error(n) => {
            ActionDefBodyElement::Error(dummy_node(n, n.value.clone()))
        }
        ActionDefBodyElement::InOutDecl(n) => {
            ActionDefBodyElement::InOutDecl(dummy_node(n, n.value.clone()))
        }
        ActionDefBodyElement::Doc(n) => ActionDefBodyElement::Doc(dummy_node(n, n.value.clone())),
        ActionDefBodyElement::Perform(n) => {
            ActionDefBodyElement::Perform(dummy_node(n, normalize_perform(&n.value)))
        }
        ActionDefBodyElement::Bind(n) => ActionDefBodyElement::Bind(dummy_node(n, n.value.clone())),
        ActionDefBodyElement::Flow(n) => ActionDefBodyElement::Flow(dummy_node(n, n.value.clone())),
        ActionDefBodyElement::FirstStmt(n) => {
            ActionDefBodyElement::FirstStmt(dummy_node(n, n.value.clone()))
        }
        ActionDefBodyElement::MergeStmt(n) => {
            ActionDefBodyElement::MergeStmt(dummy_node(n, n.value.clone()))
        }
        ActionDefBodyElement::ActionUsage(n) => ActionDefBodyElement::ActionUsage(Box::new(
            dummy_node(n, normalize_action_usage(&n.value)),
        )),
    };
    dummy_node(el, value)
}

fn normalize_action_usage(a: &ActionUsage) -> ActionUsage {
    ActionUsage {
        name: a.name.clone(),
        type_name: a.type_name.clone(),
        accept: a.accept.clone(),
        body: normalize_action_usage_body(&a.body),
        name_span: None,
        type_ref_span: None,
    }
}

fn normalize_action_usage_body(b: &ActionUsageBody) -> ActionUsageBody {
    match b {
        ActionUsageBody::Semicolon => ActionUsageBody::Semicolon,
        ActionUsageBody::Brace { elements } => ActionUsageBody::Brace {
            elements: elements
                .iter()
                .map(normalize_action_usage_body_element_node)
                .collect(),
        },
    }
}

fn normalize_action_usage_body_element_node(
    el: &Node<ActionUsageBodyElement>,
) -> Node<ActionUsageBodyElement> {
    let value = match &el.value {
        ActionUsageBodyElement::Error(n) => {
            ActionUsageBodyElement::Error(dummy_node(n, n.value.clone()))
        }
        ActionUsageBodyElement::InOutDecl(n) => {
            ActionUsageBodyElement::InOutDecl(dummy_node(n, n.value.clone()))
        }
        ActionUsageBodyElement::Bind(n) => {
            ActionUsageBodyElement::Bind(dummy_node(n, n.value.clone()))
        }
        ActionUsageBodyElement::Flow(n) => {
            ActionUsageBodyElement::Flow(dummy_node(n, n.value.clone()))
        }
        ActionUsageBodyElement::FirstStmt(n) => {
            ActionUsageBodyElement::FirstStmt(dummy_node(n, n.value.clone()))
        }
        ActionUsageBodyElement::MergeStmt(n) => {
            ActionUsageBodyElement::MergeStmt(dummy_node(n, n.value.clone()))
        }
        ActionUsageBodyElement::ActionUsage(n) => ActionUsageBodyElement::ActionUsage(Box::new(
            dummy_node(n, normalize_action_usage(&n.value)),
        )),
    };
    dummy_node(el, value)
}
