//! Purpose-built semantic carriers for the bootstrap Ethos file kinds.

use std::marker::PhantomData;

use name_table::{EncodedName, TrueNamed};

use super::catalog::CanonicalIdentityOrder;

/// One compatibility version written as three canonical decimal components.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
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
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum EthosKind {
    Interface,
    Nexus,
    Sema,
}

/// Compile-time marker for the bootstrap grammar's structural vocabulary.
///
/// It carries no identity or hierarchy: every semantic reference is an
/// opaque [`EncodedName`].
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct BootstrapLanguage;

/// Retained compatibility metadata for one decoded file.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct EthosHeader {
    pub kind: EthosKind,
    pub version: EthosVersion,
}

/// A source-only import path and its nonempty selector vector.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct ImportEntry {
    pub module_path: Vec<String>,
    pub imported_names: Vec<String>,
}

/// Source-only information needed to reproduce the authored projection.
#[derive(
    rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Default, Eq, PartialEq,
)]
pub struct BootstrapSourceProjection {
    pub imports: Vec<ImportEntry>,
}

/// A semantically decoded bootstrap document plus source-only projection data.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct DecodedBootstrap {
    pub document: BootstrapDocument,
    pub source: BootstrapSourceProjection,
}

/// The common envelope with a typed, kind-selected body.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct BootstrapDocument {
    pub header: EthosHeader,
    pub body: BootstrapBody,
}

/// The three provisional root schemas. The enum is an implementation boundary,
/// not a language-ontology claim.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
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
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum InterfaceRole {
    Input,
    Output,
    Refusal,
}

/// A role position either declares a plain nominal type inline or references
/// one. Authored Stream/Nomos declarations deliberately remain exclusive to
/// Interface support Types: applying a Stream initiation declaration directly
/// as Input/Output/Refusal has no coherent role-position semantics.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum RoleEntry {
    Declaration(TypeDeclaration),
    Reference(EncodedName),
}

impl RoleEntry {
    pub(crate) fn target(&self) -> &EncodedName {
        match self {
            Self::Declaration(declaration) => &declaration.name,
            Self::Reference(reference) => reference,
        }
    }
}

/// A relation owned by the Interface root; it never mutates the target type.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct InterfaceRoleMembership {
    pub role: InterfaceRole,
    pub target: EncodedName,
}

/// Roles first, support declarations last.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
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
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct NexusBody {
    pub traits: Vec<TraitDeclaration>,
    pub types: Vec<Declaration>,
}

/// Persistent nominal declarations followed by their keyed tables.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct SemaBody {
    pub record_types: Vec<TypeDeclaration>,
    pub tables: Vec<TableDeclaration>,
}

// These are the strict filled values occupied by individual authority seats.
// Each hashes its own portable rkyv archive directly, with no serializer or
// aggregate-document substitute.
impl TrueNamed for TypeDeclaration {}
impl TrueNamed for VariantDeclaration {}
impl TrueNamed for TraitDeclaration {}
impl TrueNamed for MethodDeclaration {}
impl TrueNamed for TableDeclaration {}

/// The deliberately closed bootstrap authored declaration algebra.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum Declaration {
    Type(TypeDeclaration),
    Nomos(NomosDeclaration),
}

/// Audited purpose-built Nomos alternatives.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum NomosDeclaration {
    StreamInitiation(StreamInitiationDeclaration),
}

/// The authored meaning of `Name.Stream.(Query Event)`.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct StreamInitiationDeclaration {
    /// The authored name designates the eventual direct Stream output identity.
    pub name: EncodedName,
    pub query: TypeExpression,
    pub event: TypeExpression,
}

/// A named plain nominal declaration.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TypeDeclaration {
    pub name: EncodedName,
    pub body: TypeBody,
}

/// Structural alternatives selected by the authored delimiter.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
#[rkyv(
    serialize_bounds(__S: rkyv::ser::Writer + rkyv::ser::Allocator, __S::Error: rkyv::rancor::Source),
    deserialize_bounds(__D::Error: rkyv::rancor::Source),
    bytecheck(bounds(__C: rkyv::validation::ArchiveContext, __C::Error: rkyv::rancor::Source)),
)]
pub enum TypeBody {
    Newtype(#[rkyv(omit_bounds)] TypeExpression),
    Struct(#[rkyv(omit_bounds)] Vec<TypeExpression>),
    Enum(#[rkyv(omit_bounds)] Vec<VariantDeclaration>),
}

/// A declaration scoped by its owning enum.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct VariantDeclaration {
    pub name: EncodedName,
    pub body: VariantBody,
}

/// The strict unit, unary, and nonempty product variant alternatives.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
#[rkyv(
    serialize_bounds(__S: rkyv::ser::Writer + rkyv::ser::Allocator, __S::Error: rkyv::rancor::Source),
    deserialize_bounds(__D::Error: rkyv::rancor::Source),
    bytecheck(bounds(__C: rkyv::validation::ArchiveContext, __C::Error: rkyv::rancor::Source)),
)]
pub enum VariantBody {
    Unit,
    Unary(#[rkyv(omit_bounds)] TypeExpression),
    Product(#[rkyv(omit_bounds)] Vec<TypeExpression>),
}

/// A parameter identity local to one containing type or method declaration.
#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct LocalParameter {
    pub owner: EncodedName,
    pub ordinal: u32,
}

/// The binder form is semantic and cannot be omitted from named requirements.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
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
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
#[rkyv(
    serialize_bounds(__S: rkyv::ser::Writer + rkyv::ser::Allocator, __S::Error: rkyv::rancor::Source),
    deserialize_bounds(__D::Error: rkyv::rancor::Source),
    bytecheck(bounds(__C: rkyv::validation::ArchiveContext, __C::Error: rkyv::rancor::Source)),
)]
pub enum TypeExpression {
    Reference(EncodedName),
    ShapeApplication(#[rkyv(omit_bounds)] ShapeApplication),
    TraitRequirement(TraitRequirement),
}

/// An identity registered as a Shape, applied at its catalog-defined arity.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
#[rkyv(
    serialize_bounds(__S: rkyv::ser::Writer + rkyv::ser::Allocator, __S::Error: rkyv::rancor::Source),
    deserialize_bounds(__D::Error: rkyv::rancor::Source),
    bytecheck(bounds(__C: rkyv::validation::ArchiveContext, __C::Error: rkyv::rancor::Source)),
)]
pub struct ShapeApplication {
    pub shape: EncodedName,
    #[rkyv(omit_bounds)]
    pub arguments: Vec<TypeExpression>,
}

/// One local parameter constrained by a normalized, nonempty Trait vector.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TraitRequirement {
    pub(crate) binder: ParameterBinder,
    pub(crate) required_traits: Vec<EncodedName>,
}

impl TraitRequirement {
    pub fn binder(&self) -> &ParameterBinder {
        &self.binder
    }

    pub fn required_traits(&self) -> &[EncodedName] {
        &self.required_traits
    }
}

/// The prepared generated declaration whose value is the query value.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct StreamInitiationInterfaceDeclaration {
    pub name: EncodedName,
    pub query: TypeExpression,
}

/// The prepared direct `Stream<Event>` Output declaration.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct StreamInterfaceDeclaration {
    pub name: EncodedName,
    pub stream_of_event: ShapeApplication,
}

/// The prepared termination Input referencing the direct Stream Output.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct StreamTerminationInterfaceDeclaration {
    pub name: EncodedName,
    pub stream_handle: EncodedName,
}

/// Atomic Stream declarations and Interface-owned role relations prepared for
/// an external store to commit or reject as one transaction.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct PreparedStreamGeneration {
    pub initiation: StreamInitiationInterfaceDeclaration,
    pub output: StreamInterfaceDeclaration,
    pub termination: StreamTerminationInterfaceDeclaration,
    pub role_relations: [InterfaceRoleMembership; 3],
}

/// One behavioral Trait with zero or more scoped methods.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TraitDeclaration {
    pub name: EncodedName,
    pub methods: Vec<MethodDeclaration>,
}

/// A method signature has zero or more parameters and one mandatory return.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct MethodDeclaration {
    pub name: EncodedName,
    pub parameters: Vec<TypeExpression>,
    pub return_type: TypeExpression,
}

/// A keyed persistent table. Both leaves are strict persistent nominal refs.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct TableDeclaration {
    pub name: EncodedName,
    pub record_type: EncodedName,
    pub key_type: EncodedName,
}

/// Catalog identities underlying the validated runtime Stream carriers.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStreamSchemaContract {
    pub stream_shape: EncodedName,
    pub stream_identity_shape: EncodedName,
    pub stream_shape_arity: u16,
    pub stream_identity_shape_arity: u16,
}

/// A runtime Stream handle indexed by its Event type.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStreamIdentity<Event> {
    encoded_name: EncodedName,
    event: PhantomData<fn() -> Event>,
}

impl<Event> RuntimeStreamIdentity<Event> {
    pub fn new(
        encoded_name: EncodedName,
        identities: &CanonicalIdentityOrder,
    ) -> Result<Self, RuntimeStreamValueError> {
        if !identities.contains(&encoded_name) {
            return Err(RuntimeStreamValueError::UnrecognizedHandle(encoded_name));
        }
        Ok(Self {
            encoded_name,
            event: PhantomData,
        })
    }

    pub const fn encoded_name(&self) -> &EncodedName {
        &self.encoded_name
    }
}

/// The query value consumed by Stream initiation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStreamInitiation<Query> {
    query: Query,
}

impl<Query> RuntimeStreamInitiation<Query> {
    pub const fn new(query: Query) -> Self {
        Self { query }
    }

    pub const fn query(&self) -> &Query {
        &self.query
    }
}

/// A `Stream<Event>` value is exactly its typed identity handle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStream<Event> {
    identity: RuntimeStreamIdentity<Event>,
}

impl<Event> RuntimeStream<Event> {
    pub const fn new(identity: RuntimeStreamIdentity<Event>) -> Self {
        Self { identity }
    }

    pub const fn identity(&self) -> &RuntimeStreamIdentity<Event> {
        &self.identity
    }
}

/// Termination carries the exact same typed Stream value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStreamTermination<Event> {
    stream: RuntimeStream<Event>,
}

impl<Event> RuntimeStreamTermination<Event> {
    pub const fn new(stream: RuntimeStream<Event>) -> Self {
        Self { stream }
    }

    pub const fn stream(&self) -> &RuntimeStream<Event> {
        &self.stream
    }
}

/// One validated initiation/output/termination runtime value family.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStreamValues<Query, Event> {
    initiation: RuntimeStreamInitiation<Query>,
    stream: RuntimeStream<Event>,
    termination: RuntimeStreamTermination<Event>,
}

impl<Query, Event> RuntimeStreamValues<Query, Event> {
    pub fn new(
        initiation: RuntimeStreamInitiation<Query>,
        stream: RuntimeStream<Event>,
        termination: RuntimeStreamTermination<Event>,
    ) -> Result<Self, RuntimeStreamValueError> {
        if stream.identity.encoded_name != termination.stream.identity.encoded_name {
            return Err(RuntimeStreamValueError::MismatchedTerminationStream {
                stream: stream.identity.encoded_name.clone(),
                termination: termination.stream.identity.encoded_name.clone(),
            });
        }
        Ok(Self {
            initiation,
            stream,
            termination,
        })
    }

    pub const fn initiation(&self) -> &RuntimeStreamInitiation<Query> {
        &self.initiation
    }

    pub const fn stream(&self) -> &RuntimeStream<Event> {
        &self.stream
    }

    pub const fn termination(&self) -> &RuntimeStreamTermination<Event> {
        &self.termination
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RuntimeStreamValueError {
    #[error("runtime Stream handle {0:?} is absent from the canonical identity registry")]
    UnrecognizedHandle(EncodedName),
    #[error("termination Stream {termination:?} differs from Stream value {stream:?}")]
    MismatchedTerminationStream {
        stream: EncodedName,
        termination: EncodedName,
    },
}
