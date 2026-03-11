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
}

/// Root of a SysML document: a sequence of package-level elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootNamespace {
    pub elements: Vec<Node<PackageBodyElement>>,
}

/// Top-level element inside a namespace or package body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageBodyElement {
    Doc(Node<DocComment>),
    Package(Node<Package>),
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
    ConstraintDef(Node<ConstraintDef>),
    CalcDef(Node<CalcDef>),
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

/// Import: `private`? `import` `all`? QualifiedName (`::` `*`)? etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub visibility: Option<Visibility>,
    /// Whether this is a namespace import (Definitions::*) or membership (SI::kg).
    pub is_import_all: bool,
    /// Import target, e.g. "SI::kg" or "Definitions::*".
    pub target: String,
}

/// Part definition: `part def` Identification (`:>` specializes)? Body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartDef {
    pub identification: Identification,
    /// Supertype after `:>`, e.g. Some("Axle") for `part def FrontAxle :> Axle`.
    pub specializes: Option<String>,
    pub body: PartDefBody,
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
    Doc(Node<DocComment>),
    AttributeDef(Node<AttributeDef>),
    AttributeUsage(Node<AttributeUsage>),
    PortUsage(Node<PortUsage>),
    PartUsage(Box<Node<PartUsage>>),
    Connect(Node<Connect>),
    Perform(Node<Perform>),
    Allocate(Node<Allocate>),
}

/// Attribute definition: `attribute` name (`:>` type)? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeDef {
    pub name: String,
    /// Type after `:>`, e.g. Some("ISQ::mass").
    pub typing: Option<String>,
    pub body: AttributeBody,
}

/// Body of an attribute (def or usage): `;` or `{` ... `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeBody {
    Semicolon,
    Brace,
}

/// Part usage: `part` name `:` type multiplicity? `ordered`? body.
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
    pub body: PartUsageBody,
}

/// Body of a part usage: `;` or `{` PartUsageBodyElement* `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartUsageBody {
    Semicolon,
    Brace {
        elements: Vec<Node<PartUsageBodyElement>>,
    },
}

/// Element inside a part usage body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartUsageBodyElement {
    Doc(Node<DocComment>),
    AttributeUsage(Node<AttributeUsage>),
    PartUsage(Box<Node<PartUsage>>),
    PortUsage(Node<PortUsage>),
    Bind(Node<Bind>),
    InterfaceUsage(Node<InterfaceUsage>),
    Connect(Node<Connect>),
    Perform(Node<Perform>),
    Allocate(Node<Allocate>),
    Satisfy(Node<Satisfy>),
    StateUsage(Node<StateUsage>),
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

/// In/out binding inside a perform body: `in` name `=` expr `;` or `out` name `=` expr `;`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformBodyElement {
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
}

// ---------------------------------------------------------------------------
// Port
// ---------------------------------------------------------------------------

/// Port definition: `port def` Identification body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortDef {
    pub identification: Identification,
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
}

/// Body of a port usage: `;` or `{` PortUsage* `}` (nested ports).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortBody {
    Semicolon,
    Brace,
    /// Brace with nested port usages (e.g. port vehicleToRoadPort redefines ... { port left...; port right...; }).
    BraceWithPorts { elements: Vec<Node<PortUsage>> },
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
    EndDecl(Node<EndDecl>),
    RefDecl(Node<RefDecl>),
    ConnectStmt(Node<ConnectStmt>),
}

/// End declaration in interface def: `end` name `:` type `;`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndDecl {
    pub name: String,
    pub type_name: String,
}

/// Ref declaration in interface def: `ref` name `:` type body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefDecl {
    pub name: String,
    pub type_name: String,
    pub body: RefBody,
}

/// Body of a ref declaration: `;` or `{` ... `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefBody {
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
    InOutDecl(Node<InOutDecl>),
    Doc(Node<DocComment>),
    Perform(Node<Perform>),
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
    SubjectDecl(Node<SubjectDecl>),
    RequireConstraint(Node<RequireConstraint>),
    Doc(Node<DocComment>), // Just keeping it simple for now, or maybe DocComment is not in AST, wait.
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
    pub type_name: String,
    pub body: RequirementDefBody,
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
    StateUsage(Node<StateUsage>),
    Transition(Node<Transition>),
}

/// State usage: `state` name (`:` type)? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub body: StateDefBody,
}

/// Transition: `transition` name `first` source `then` target body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    pub name: String,
    pub source: Node<Expression>,
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

/// Doc comment: `doc /* ... */`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocComment {
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
