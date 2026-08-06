//! Typed bootstrap-reader failures.

use signal_sema_translator::VocabularyEncodedId;

use super::catalog::SchemaRole;
use super::model::{EthosKind, EthosVersion};

/// Failure to build the shared structural reader.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapBuildError {
    #[error("bootstrap grammar identity {position} must use the Universal vocabulary root")]
    NonUniversalGrammarIdentity { position: &'static str },
    #[error("bootstrap document and syntax grammar identities must be distinct")]
    DuplicateGrammarIdentity,
    #[error("bootstrap token profile is invalid: {0}")]
    TokenProfile(#[from] raw_discovery::TokenProfileError),
    #[error("bootstrap structural grammar is invalid: {0}")]
    Authoring(#[from] structural_codec::AuthoringError),
    #[error("bootstrap structural table is invalid: {0}")]
    Table(Box<structural_codec::TableError<signal_sema_translator::VocabularyRoot>>),
}

/// Failure in catalog construction, discovery, planning, or semantic sealing.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapReadError {
    #[error("structural discovery or evaluation failed: {0}")]
    Structural(Box<structural_codec::DecodeError<signal_sema_translator::VocabularyRoot>>),
    #[error("expected {expected} at byte {start}, found {found}")]
    UnexpectedStructure {
        expected: &'static str,
        found: &'static str,
        start: usize,
    },
    #[error(
        "header name {0:?} is not one of the metadata projections of the three file-kind priors"
    )]
    UnknownFileKind(String),
    #[error("Ethos version {found:?} is unsupported; supported versions are {supported:?}")]
    UnsupportedVersion {
        found: EthosVersion,
        supported: Vec<EthosVersion>,
    },
    #[error("header kind {header:?} does not match semantic body {body:?}")]
    HeaderBodyMismatch { header: EthosKind, body: EthosKind },
    #[error("version component {0:?} is not canonical nonnegative decimal")]
    InvalidVersionComponent(String),
    #[error("module path {0:?} is invalid")]
    InvalidModulePath(Vec<String>),
    #[error("visible name {0:?} is not a safe bare projection")]
    InvalidVisibleName(String),
    #[error("import selector vector must be nonempty")]
    EmptyImportSelectors,
    #[error("textual snapshot contains more than one record for identity {0:?}")]
    DuplicateMetadataIdentity(VocabularyEncodedId),
    #[error("textual snapshot is missing identity {0:?}")]
    MissingMetadataIdentity(VocabularyEncodedId),
    #[error(
        "textual snapshot record for identity {identity:?} does not equal the required module/name projection"
    )]
    MetadataProjectionMismatch { identity: VocabularyEncodedId },
    #[error(
        "the sealing snapshot does not preserve the reader catalog's record for identity {0:?}"
    )]
    MetadataSnapshotDoesNotExtend(VocabularyEncodedId),
    #[error("the sealing snapshot adds unrelated identity {0:?}")]
    ExtraMetadataIdentity(VocabularyEncodedId),
    #[error("textual lookup {module_path:?}:{name} has no identity")]
    MissingTextualLookup {
        module_path: Vec<String>,
        name: String,
    },
    #[error("reference {name:?} is ambiguous before schema validation: {identities:?}")]
    AmbiguousReference {
        name: String,
        identities: Vec<VocabularyEncodedId>,
    },
    #[error("reference {name:?} has no identity in the visible environment")]
    UnresolvedReference { name: String },
    #[error("identity {0:?} has no schema catalog entry")]
    MissingSchema(VocabularyEncodedId),
    #[error("identity {identity:?} lacks required schema role {required:?}")]
    WrongSchemaRole {
        identity: VocabularyEncodedId,
        required: SchemaRole,
    },
    #[error("identity {identity:?} registers incompatible roles {first:?} and {second:?}")]
    ConflictingSchemaRoles {
        identity: VocabularyEncodedId,
        first: SchemaRole,
        second: SchemaRole,
    },
    #[error("schema catalog contains identity {0:?} more than once")]
    DuplicateSchemaIdentity(VocabularyEncodedId),
    #[error("schema identity {0:?} must use the Universal vocabulary root")]
    NonUniversalSchemaIdentity(VocabularyEncodedId),
    #[error("prior {position} must use a Universal identity")]
    NonUniversalPrior { position: &'static str },
    #[error("prior {position} points to identity {identity:?} without required role {required:?}")]
    InvalidPriorRole {
        position: &'static str,
        identity: VocabularyEncodedId,
        required: SchemaRole,
    },
    #[error("file-kind prior projections are not distinct")]
    DuplicateFileKindProjection,
    #[error("name {name:?} is declared more than once in {scope}")]
    DuplicateDeclaration { name: String, scope: String },
    #[error("assignment for occurrence {0} appears more than once")]
    DuplicateAssignment(u32),
    #[error("assignment refers to an occurrence outside this plan")]
    ExtraAssignment,
    #[error("declaration occurrence {0} has no naming-authority assignment")]
    MissingAssignment(u32),
    #[error("assignment for occurrence {occurrence} uses non-Universal identity {identity:?}")]
    NonUniversalAssignment {
        occurrence: u32,
        identity: VocabularyEncodedId,
    },
    #[error("assigned identity {identity:?} collides with existing or newly assigned object")]
    AssignedIdentityCollision { identity: VocabularyEncodedId },
    #[error("generated Stream assignment for occurrence {0} appears more than once")]
    DuplicateGeneratedStreamAssignment(u32),
    #[error("Stream occurrence {0} has no exact generated initiation/termination assignment")]
    MissingGeneratedStreamAssignment(u32),
    #[error("generated Stream assignment refers to a non-Stream or foreign occurrence")]
    ExtraGeneratedStreamAssignment,
    #[error("Shape identity {identity:?} requires {expected} arguments, found {found}")]
    ShapeArity {
        identity: VocabularyEncodedId,
        expected: u16,
        found: usize,
    },
    #[error("Nomos identity {identity:?} requires {expected} arguments, found {found}")]
    NomosArity {
        identity: VocabularyEncodedId,
        expected: u16,
        found: usize,
    },
    #[error("local parameter {name:?} is reused with incompatible Trait requirements")]
    ConflictingNamedParameter { name: String },
    #[error("one Trait requirement repeats Trait identity {0:?}")]
    DuplicateTrait(VocabularyEncodedId),
    #[error("one Trait requirement repeats visible Trait name {0:?}")]
    DuplicateTraitProjection(String),
    #[error("authored Stream declarations are admitted only in Interface support Types")]
    StreamOutsideInterfaceTypes,
    #[error("Sema persistent declarations admit only plain nominal types")]
    NonPersistentDeclaration,
    #[error("prepared transaction model is invalid: {0}")]
    InvalidPreparedModel(&'static str),
}

impl From<structural_codec::DecodeError<signal_sema_translator::VocabularyRoot>>
    for BootstrapReadError
{
    fn from(error: structural_codec::DecodeError<signal_sema_translator::VocabularyRoot>) -> Self {
        Self::Structural(Box::new(error))
    }
}

/// Failure to render a canonical textual projection.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapWriteError {
    #[error("prepared semantic model is invalid: {0}")]
    InvalidModel(#[from] BootstrapReadError),
    #[error("no visible spelling is available for encoded identity {0:?}")]
    MissingSpelling(VocabularyEncodedId),
}
