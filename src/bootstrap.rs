//! Strict shared bootstrap reader for Interface, Nexus, and Sema Ethos files.
//!
//! The boundary is deliberately two-phase: [`BootstrapReader::plan`] discovers
//! every declaration occurrence without identity authority, then
//! [`BootstrapReader::seal`] consumes exact authority dispositions and an explicit
//! before-to-after textual metadata transition.

mod catalog;
mod error;
mod grammar;
mod model;
mod reader;
mod root;
mod writer;

pub use catalog::{
    BootstrapCatalog, BootstrapPriorIdentities, BootstrapPriorVocabulary, BootstrapVersionPolicy,
    CanonicalIdentityOrder, IdentitySchema, IdentitySchemaCatalog, NomosSchema, SchemaRole,
    TextualMetadataRecord, TextualMetadataSnapshot, TextualMetadataTransition,
    TextualProjectionAddress,
};
pub use error::{BootstrapBuildError, BootstrapReadError, BootstrapWriteError};
pub use grammar::BootstrapGrammarIdentities;
pub use model::*;
pub use reader::{
    AssignedIdentity, BootstrapReadPlan, BootstrapReader, DeclarationOccurrence,
    DeclarationPurpose, GeneratedStreamAssignment, GeneratedStreamAssignments, IdentityDisposition,
    NamingAssignment, NamingAssignments, PlannedDeclaration, PlannedScope, PreparedBootstrapDraft,
    PreparedBootstrapTransaction,
};
pub use root::BootstrapSectionSchema;
