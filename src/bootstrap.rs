//! Strict shared bootstrap reader for Interface, Nexus, and Sema Ethos files.
//!
//! The boundary is deliberately two-phase: [`BootstrapReader::plan`] discovers
//! every declaration occurrence without identity authority, then
//! [`BootstrapReader::seal`] accepts an exact externally allocated assignment set.

mod error;
mod grammar;
mod model;
mod reader;
mod writer;

pub use error::{
    BootstrapBuildError, BootstrapReadError, BootstrapWriteError, ExpectedNameClass, NameClass,
};
pub use grammar::BootstrapGrammarIdentity;
pub use model::*;
pub use reader::{
    BootstrapCatalog, BootstrapPriorIdentities, BootstrapPriorVocabulary, BootstrapReadPlan,
    BootstrapReader, DeclarationOccurrence, DeclarationPurpose, NamingAssignment,
    NamingAssignments, PlannedDeclaration, PlannedScope, TextualMetadataEntry,
};
