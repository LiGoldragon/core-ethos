//! Purpose-built semantic carriers for the bootstrap Ethos file kinds.

use signal_sema_translator::VocabularyEncodedId;

/// One compatibility version written as three canonical decimal components.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EthosVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl EthosVersion {
    /// Construct an exact compatibility version.
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

/// The closed set of provisional bootstrap roots.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EthosKind {
    Interface,
    Nexus,
    Sema,
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

/// Source-only information needed to reproduce the authored projection.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BootstrapSourceProjection {
    pub imports: Vec<ImportEntry>,
}

/// A semantically decoded bootstrap document plus source-only projection data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedBootstrap {
    pub document: BootstrapDocument,
    pub source: BootstrapSourceProjection,
}

/// The common envelope with a typed, kind-selected body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapDocument {
    pub header: EthosHeader,
    pub body: BootstrapBody,
}

/// The three provisional root schemas. The enum is an implementation boundary,
/// not a language-ontology claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BootstrapBody {
    Interface(InterfaceBody),
    Nexus(NexusBody),
    Sema(SemaBody),
}

impl BootstrapBody {
    pub(crate) const fn kind(&self) -> EthosKind {
        match self {
            Self::Interface(_) => EthosKind::Interface,
            Self::Nexus(_) => EthosKind::Nexus,
            Self::Sema(_) => EthosKind::Sema,
        }
    }
}

/// One Interface-owned universal role.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
    /// Relations produced only by the three authored role sections. Generated
    /// Stream relations live in the prepared transaction until commit.
    pub memberships: Vec<InterfaceRoleMembership>,
}

/// Traits first, signature-supporting declarations last.
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

/// The deliberately closed bootstrap authored declaration algebra.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Declaration {
    Type(TypeDeclaration),
    Nomos(NomosDeclaration),
}

/// Audited purpose-built Nomos alternatives.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NomosDeclaration {
    StreamInitiation(StreamInitiationDeclaration),
}

/// The authored meaning of `Name.Stream.(Query Event)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamInitiationDeclaration {
    /// The authored name designates the eventual direct Stream output identity.
    pub name: VocabularyEncodedId,
    pub query: TypeExpression,
    pub event: TypeExpression,
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

/// The binder form is semantic and cannot be omitted from named requirements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParameterBinder {
    Inferred(LocalParameter),
    Named {
        parameter: LocalParameter,
        local_name: String,
    },
}

impl ParameterBinder {
    pub fn parameter(&self) -> &LocalParameter {
        match self {
            Self::Inferred(parameter) | Self::Named { parameter, .. } => parameter,
        }
    }
}

/// The strict recursive type-expression algebra.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeExpression {
    Reference(VocabularyEncodedId),
    ShapeApplication(ShapeApplication),
    TraitRequirement(TraitRequirement),
}

/// An identity registered as a Shape, applied at its catalog-defined arity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapeApplication {
    pub shape: VocabularyEncodedId,
    pub arguments: Vec<TypeExpression>,
}

/// One local parameter constrained by a normalized, nonempty Trait vector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraitRequirement {
    pub(crate) binder: ParameterBinder,
    pub(crate) required_traits: Vec<VocabularyEncodedId>,
}

impl TraitRequirement {
    pub fn binder(&self) -> &ParameterBinder {
        &self.binder
    }

    pub fn required_traits(&self) -> &[VocabularyEncodedId] {
        &self.required_traits
    }
}

/// The prepared generated declaration whose value is the query value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamInitiationInterfaceDeclaration {
    pub name: VocabularyEncodedId,
    pub query: TypeExpression,
}

/// The prepared direct `Stream<Event>` Output declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamInterfaceDeclaration {
    pub name: VocabularyEncodedId,
    pub stream_of_event: ShapeApplication,
}

/// The prepared termination Input referencing the direct Stream Output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamTerminationInterfaceDeclaration {
    pub name: VocabularyEncodedId,
    pub stream_handle: VocabularyEncodedId,
}

/// Atomic Stream declarations and Interface-owned role relations prepared for
/// an external store to commit or reject as one transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedStreamGeneration {
    pub initiation: StreamInitiationInterfaceDeclaration,
    pub output: StreamInterfaceDeclaration,
    pub termination: StreamTerminationInterfaceDeclaration,
    pub role_relations: [InterfaceRoleMembership; 3],
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

/// Runtime Stream value anatomy remains a catalog contract at this stage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStreamSchemaContract {
    pub stream_shape: VocabularyEncodedId,
    pub stream_identity_shape: VocabularyEncodedId,
    pub stream_shape_arity: u16,
    pub stream_identity_shape_arity: u16,
}

/// Archiving is deliberately deferred until the random EncodedName substrate is
/// stable enough that an archive layout would not freeze today's chain carrier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapArchiveStatus {
    NotYetArchived,
}
