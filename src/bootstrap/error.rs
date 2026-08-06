//! Typed bootstrap-reader failures.

use signal_sema_translator::VocabularyEncodedId;

use super::model::EthosKind;

/// The semantic class registered for one resolved identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameClass {
    Nominal,
    PersistentNominal,
    Shape,
    Trait,
    NomosHead,
    Table,
    Method,
}

/// A source position's required reference class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpectedNameClass {
    Nominal,
    PersistentNominal,
    Shape,
    Trait,
    StreamNomosHead,
}

/// Failure to build the shared structural reader.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapBuildError {
    #[error("bootstrap grammar identity must use the Universal vocabulary root")]
    NonUniversalGrammarIdentity,
    #[error("bootstrap token profile is invalid: {0}")]
    TokenProfile(#[from] raw_discovery::TokenProfileError),
    #[error("bootstrap structural grammar is invalid: {0}")]
    Authoring(#[from] structural_codec::AuthoringError),
    #[error("bootstrap structural table is invalid: {0}")]
    Table(Box<structural_codec::TableError<signal_sema_translator::VocabularyRoot>>),
}

/// Failure in discovery, planning, exact assignment, or semantic sealing.
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
    #[error("unknown Ethos file kind {0:?}")]
    UnknownFileKind(String),
    #[error("header kind {header:?} does not match selected body {body:?}")]
    HeaderBodyMismatch { header: EthosKind, body: EthosKind },
    #[error("version component {0:?} is not canonical nonnegative decimal")]
    InvalidVersionComponent(String),
    #[error("import module path {0:?} is invalid")]
    InvalidModulePath(String),
    #[error("import selector vector must be nonempty")]
    EmptyImportSelectors,
    #[error("name {name:?} is declared more than once in {scope}")]
    DuplicateDeclaration { name: String, scope: String },
    #[error("assignment for occurrence {0} appears more than once")]
    DuplicateAssignment(u32),
    #[error("assignment refers to unknown occurrence {0}")]
    ExtraAssignment(u32),
    #[error("declaration occurrence {0} has no naming-authority assignment")]
    MissingAssignment(u32),
    #[error(
        "the same encoded identity was assigned to declaration occurrences {first} and {second}"
    )]
    DuplicateAssignedIdentity { first: u32, second: u32 },
    #[error("reference {name:?} is absent from local declarations, imports, and priors")]
    UnresolvedReference { name: String },
    #[error("reference {name:?} is ambiguous in the textual environment")]
    AmbiguousReference { name: String },
    #[error("reference {name:?} resolved as {actual:?}, expected {expected:?}")]
    WrongReferenceClass {
        name: String,
        actual: NameClass,
        expected: ExpectedNameClass,
    },
    #[error("catalog contains duplicate path/name entry {module_path:?}:{name}")]
    DuplicateCatalogEntry {
        module_path: Vec<String>,
        name: String,
    },
    #[error("catalog prior {position} must use a Universal identity")]
    NonUniversalPrior { position: &'static str },
    #[error("catalog entry {name:?} must use a Universal identity")]
    NonUniversalCatalogIdentity { name: String },
    #[error("local parameter {name:?} is reused with incompatible Trait requirements")]
    ConflictingNamedParameter { name: String },
    #[error("one Trait requirement repeats Trait identity {0:?}")]
    DuplicateTrait(VocabularyEncodedId),
    #[error("authored Stream declarations are admitted only in Interface support Types")]
    StreamOutsideInterfaceTypes,
    #[error("Sema persistent declarations admit only plain nominal types")]
    NonPersistentDeclaration,
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
    #[error("no visible spelling is available for encoded identity {0:?}")]
    MissingSpelling(VocabularyEncodedId),
    #[error("decoded bootstrap model violates writer invariant: {0}")]
    InvalidModel(&'static str),
}
