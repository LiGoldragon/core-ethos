//! Typed errors for the retained sealed declaration algebra.

use content_identity::ArchiveError;
use name_table::Identifier;

/// Computing a stringless-Encoded value's content identity failed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EncodedIdentityError {
    #[error(transparent)]
    Archive(#[from] ArchiveError),
}

/// Which encoded reference in a streaming relation failed validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamingRelationReference {
    Token,
    Event,
    CloseToken,
}

impl std::fmt::Display for StreamingRelationReference {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Token => formatter.write_str("token"),
            Self::Event => formatter.write_str("event"),
            Self::CloseToken => formatter.write_str("close-token"),
        }
    }
}

/// The non-plain reference form rejected at a streaming value position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamingReferenceForm {
    Scalar,
    BytesLength,
    SingleTypeApplication,
    MultiTypeApplication,
}

impl std::fmt::Display for StreamingReferenceForm {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scalar => formatter.write_str("scalar"),
            Self::BytesLength => formatter.write_str("Bytes length"),
            Self::SingleTypeApplication => formatter.write_str("single-type generic application"),
            Self::MultiTypeApplication => formatter.write_str("multi-type generic application"),
        }
    }
}

/// An EncodedEthos relation or its Ethos-local identifiers did not meet the encoded
/// Ethos contract.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EncodedEthosError {
    #[error("EncodedEthos requires Schema identifiers, not {0}")]
    NonEthosIdentifier(Identifier),
    #[error("a streaming relation requires an input interface enumeration")]
    MissingInputInterface,
    #[error("a streaming relation requires an output interface enumeration")]
    MissingOutputInterface,
    #[error("the {0:?} interface root must be an enumeration")]
    InterfaceRootNotEnumeration(crate::declaration::DeclarationRole),
    #[error("streaming opening endpoint {0} is not an input-interface variant")]
    OpeningEndpointNotInputVariant(Identifier),
    #[error("streaming acknowledgement endpoint {0} is not an output-interface variant")]
    AcknowledgementEndpointNotOutputVariant(Identifier),
    #[error("streaming {part} reference {identifier} does not resolve in this Ethos value")]
    UnresolvedStreamingReference {
        part: StreamingRelationReference,
        identifier: Identifier,
    },
    #[error(
        "streaming {part} reference {identifier} must name a data-type declaration, not {actual:?}"
    )]
    StreamingReferenceNotDataType {
        part: StreamingRelationReference,
        identifier: Identifier,
        actual: crate::declaration::DeclarationRole,
    },
    #[error("streaming {part} reference must name a data-type declaration, not a {form} reference")]
    StreamingReferenceMustNameDataType {
        part: StreamingRelationReference,
        form: StreamingReferenceForm,
    },
}

/// A failure at the validated EncodedEthos archive boundary.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EncodedEthosLoadError {
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    #[error(transparent)]
    Ethos(#[from] EncodedEthosError),
}
