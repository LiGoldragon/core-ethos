//! Strict shared bootstrap reader for Interface, Nexus, and Sema Ethos files.
//!
//! The boundary is deliberately two-phase: [`BootstrapReader::plan`] discovers
//! every declaration occurrence without identity authority, then
//! [`BootstrapReader::seal`] accepts an exact externally allocated assignment set.

mod catalog;
mod error;
mod grammar;
mod model;
mod reader;
mod root;
mod writer;

pub use catalog::{
    BootstrapCatalog, BootstrapPriorIdentities, BootstrapPriorVocabulary, BootstrapVersionPolicy,
    IdentitySchema, IdentitySchemaCatalog, NomosSchema, SchemaRole, TextualMetadataRecord,
    TextualMetadataSnapshot,
};
pub use error::{BootstrapBuildError, BootstrapReadError, BootstrapWriteError};
pub use grammar::BootstrapGrammarIdentities;
pub use model::*;
pub use reader::{
    BootstrapReadPlan, BootstrapReader, DeclarationOccurrence, DeclarationPurpose,
    GeneratedStreamAssignment, GeneratedStreamAssignments, NamingAssignment, NamingAssignments,
    PlannedDeclaration, PlannedScope, PreparedBootstrapTransaction,
};
