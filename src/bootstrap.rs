//! Strict shared bootstrap reader for Interface, Nexus, and Sema Ethos files.
//!
//! The boundary is deliberately two-phase: [`BootstrapReader::plan`] discovers
//! every declaration occurrence without identity authority, then
//! [`BootstrapReader::seal`] prepares the exact transaction draft and obtains a
//! configuration-specific receipt from the injected naming authority.

mod catalog;
mod error;
mod grammar;
mod model;
mod reader;
mod root;
mod writer;

pub use catalog::{
    BootstrapCatalog, BootstrapPriorDefinition, BootstrapPriorIdentities, BootstrapPriorRole,
    BootstrapPriorSlot, BootstrapPriorVocabulary, BootstrapVersionPolicy, CanonicalIdentityOrder,
    IdentitySchema, IdentitySchemaCatalog, SchemaRole, TextualMetadataRecord,
    TextualMetadataSnapshot, TextualMetadataTransition, TextualProjectionAddress,
    bootstrap_prior_definitions,
};
pub use error::{BootstrapBuildError, BootstrapReadError, BootstrapWriteError};
pub use grammar::BootstrapGrammarIdentities;
pub use model::*;
pub use reader::{
    AdmittedToCoreEthosRawConstruction, BootstrapCatalogMetadata, BootstrapNamingAuthority,
    BootstrapNamingAuthorityRequest, BootstrapReadPlan, BootstrapReader, DeclarationOccurrence,
    DeclarationPurpose, IdentityDisposition, NamingAssignment, NamingAssignments,
    PlannedDeclaration, PlannedScope, PreparedBootstrapDraft, PreparedBootstrapTransaction,
    SemaTransactionAssembler,
};
pub use root::BootstrapSectionSchema;
