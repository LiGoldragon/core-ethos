//! Purpose-built semantic carriers for the bootstrap Ethos file kinds.

use std::collections::BTreeMap;

use signal_sema_translator::VocabularyEncodedId;

/// One compatibility version written as three canonical decimal components.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EthosVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

/// The closed set of bootstrap roots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EthosKind {
    Interface,
    Nexus,
    Sema,
}

impl EthosKind {
    pub(crate) const fn spelling(self) -> &'static str {
        match self {
            Self::Interface => "Interface",
            Self::Nexus => "Nexus",
            Self::Sema => "Sema",
        }
    }
}

/// Retained compatibility metadata for one decoded file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EthosHeader {
    pub kind: EthosKind,
    pub version: EthosVersion,
}

/// A source-only import path and its nonempty selector vector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportEntry {
    pub module_path: Vec<String>,
    pub imported_names: Vec<String>,
}

/// Source-only information required to write an equivalent textual projection.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BootstrapSourceMetadata {
    pub imports: Vec<ImportEntry>,
    pub(crate) named_parameters: BTreeMap<LocalParameter, String>,
}

/// A semantically decoded bootstrap document plus non-semantic textual metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedBootstrap {
    pub document: BootstrapDocument,
    pub source: BootstrapSourceMetadata,
}

/// The common envelope with a typed, kind-selected body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapDocument {
    pub header: EthosHeader,
    pub body: BootstrapBody,
}

/// The three provisional root schemas. This enum is an implementation boundary,
/// not a claim that the language ontology is a Rust sum.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BootstrapBody {
    Interface(InterfaceBody),
    Nexus(NexusBody),
    Sema(SemaBody),
}

/// One Interface-owned universal role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterfaceRole {
    Input,
    Output,
    Refusal,
}

/// A role position either declares its nominal type inline or references one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoleEntry {
    Declaration(TypeDeclaration),
    Reference(VocabularyEncodedId),
}

impl RoleEntry {
    pub(crate) fn target(&self) -> &VocabularyEncodedId {
        match self {
            Self::Declaration(declaration) => &declaration.name,
            Self::Reference(reference) => reference,
        }
    }
}

/// A relation owned by the Interface root; it never mutates the target type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterfaceRoleMembership {
    pub role: InterfaceRole,
    pub target: VocabularyEncodedId,
}

/// Roles first, support declarations last.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterfaceBody {
    pub inputs: Vec<RoleEntry>,
    pub outputs: Vec<RoleEntry>,
    pub refusals: Vec<RoleEntry>,
    pub types: Vec<Declaration>,
    pub memberships: Vec<InterfaceRoleMembership>,
}

/// Traits first, signature-supporting types last.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NexusBody {
    pub traits: Vec<TraitDeclaration>,
    pub types: Vec<Declaration>,
}

/// Persistent nominal declarations followed by their keyed tables.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemaBody {
    pub record_types: Vec<TypeDeclaration>,
    pub tables: Vec<TableDeclaration>,
}

/// The deliberately closed bootstrap declaration algebra.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Declaration {
    Type(TypeDeclaration),
    Stream(GeneratedStreamDeclarations),
}

/// A named plain nominal declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeDeclaration {
    pub name: VocabularyEncodedId,
    pub body: TypeBody,
}

/// Structural alternatives selected by the authored delimiter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeBody {
    Newtype(TypeExpression),
    Struct(Vec<TypeExpression>),
    Enum(Vec<VariantDeclaration>),
}

/// A declaration scoped by its owning enum.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariantDeclaration {
    pub name: VocabularyEncodedId,
    pub body: VariantBody,
}

/// The strict unit, unary, and nonempty product variant alternatives.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VariantBody {
    Unit,
    Unary(TypeExpression),
    Product(Vec<TypeExpression>),
}

/// A parameter identity local to one containing type or method declaration.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalParameter {
    pub owner: VocabularyEncodedId,
    pub ordinal: u32,
}

/// The strict recursive type-expression algebra.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeExpression {
    Reference(VocabularyEncodedId),
    ShapeApplication(ShapeApplication),
    TraitRequirement(TraitRequirement),
}

/// A prior-vocabulary Shape applied to one or more arguments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapeApplication {
    pub shape: VocabularyEncodedId,
    pub arguments: Vec<TypeExpression>,
}

/// One local parameter constrained by a normalized, nonempty Trait vector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraitRequirement {
    pub parameter: LocalParameter,
    pub required_traits: Vec<VocabularyEncodedId>,
}

/// The one audited bootstrap Nomos arm, resolved atomically into three types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedStreamDeclarations {
    pub initiation: StreamInitiationInterfaceDeclaration,
    pub output: StreamInterfaceDeclaration,
    pub termination: StreamTerminationInterfaceDeclaration,
}

/// The generated Input whose value is the query value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamInitiationInterfaceDeclaration {
    pub name: VocabularyEncodedId,
    pub query: TypeExpression,
}

/// The generated Output whose body is directly `Stream<Event>`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamInterfaceDeclaration {
    pub name: VocabularyEncodedId,
    pub stream_of_event: ShapeApplication,
}

/// The generated Input whose body references the Output declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamTerminationInterfaceDeclaration {
    pub name: VocabularyEncodedId,
    pub stream_handle: VocabularyEncodedId,
}

/// One behavioral Trait with zero or more scoped methods.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraitDeclaration {
    pub name: VocabularyEncodedId,
    pub methods: Vec<MethodDeclaration>,
}

/// A method signature has zero or more parameters and one mandatory return.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MethodDeclaration {
    pub name: VocabularyEncodedId,
    pub parameters: Vec<TypeExpression>,
    pub return_type: TypeExpression,
}

/// A keyed persistent table. Both leaves are strict persistent nominal refs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableDeclaration {
    pub name: VocabularyEncodedId,
    pub record_type: VocabularyEncodedId,
    pub key_type: VocabularyEncodedId,
}
