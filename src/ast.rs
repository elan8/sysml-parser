//! Abstract syntax tree types for SysML v2 textual notation.

/// Root of a SysML document: a sequence of package-level elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootNamespace {
    pub elements: Vec<PackageBodyElement>,
}

/// Top-level element inside a namespace or package body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageBodyElement {
    Package(Package),
    Import(Import),
    PartDef(PartDef),
    PartUsage(PartUsage),
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
        elements: Vec<PackageBodyElement>,
    },
}

/// Visibility for imports and members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
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
        elements: Vec<PartDefBodyElement>,
    },
}

/// Element inside a part definition body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartDefBodyElement {
    AttributeDef(AttributeDef),
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
    /// Optional `subsets` feature and value, e.g. ("frontWheel", "frontWheel#(1)").
    pub subsets: Option<(String, Option<String>)>,
    pub body: PartUsageBody,
}

/// Body of a part usage: `;` or `{` PartUsageBodyElement* `}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartUsageBody {
    Semicolon,
    Brace {
        elements: Vec<PartUsageBodyElement>,
    },
}

/// Element inside a part usage body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartUsageBodyElement {
    AttributeUsage(AttributeUsage),
    PartUsage(Box<PartUsage>),
}

/// Attribute usage: `attribute` name `redefines`? value? body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeUsage {
    pub name: String,
    /// Redefines target, e.g. Some("Vehicle::mass").
    pub redefines: Option<String>,
    /// Value expression, e.g. Some("1750 [kg]").
    pub value: Option<String>,
    pub body: AttributeBody,
}
