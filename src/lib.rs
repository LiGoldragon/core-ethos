//! # core-ethos
//!
//! Strict bootstrap Ethos reading and encoded carriers for the protos language family.
//!
//! [`bootstrap`] is the canonical two-phase textual boundary. [`whole`] and the
//! older flat declaration algebra remain temporarily for consumer migration and
//! are not extended by the bootstrap schema.

pub mod bootstrap;
pub mod capsule;
pub mod declaration;
pub mod error;
pub mod reference;
pub mod whole;

pub use capsule::capsule_from_issued_hash;
pub use declaration::{
    DeclarationRole, EncodedDeclaration, EncodedEnum, EncodedEthos, EncodedEthosDomain,
    EncodedField, EncodedNewtype, EncodedStruct, EncodedType, EncodedVariant, StreamingRelation,
    Visibility,
};
pub use error::{
    EncodedEthosError, EncodedEthosLoadError, EncodedIdentityError, StreamingReferenceForm,
    StreamingRelationReference,
};
pub use reference::{
    BuiltinReference, EncodedReference, MultiTypeReferenceProjection,
    SingleTypeReferenceProjection, ValueReferenceProjection,
};
pub use whole::{
    DecodedEncodedIdPosition, DecodedEthos, EmptyEnumerationVariants, EmptyStructFields,
    EmptyTupleFields, EmptyTypeArguments, EthosCodec, EthosCodecBuildError, EthosDecodeError,
    EthosDocument, EthosDocumentCodec, EthosEncodeError, EthosGrammarError, EthosGrammarIdPosition,
    EthosGrammarIdentities, EthosGrammarIds, WholeEthos, WholeEthosArchiveError,
    WholeEthosAttributes, WholeEthosBody, WholeEthosBuiltinPriorError,
    WholeEthosBuiltinPriorPosition, WholeEthosBuiltinPriors, WholeEthosEncodedIdPosition,
    WholeEthosEnumeration, WholeEthosFileKind, WholeEthosHeader, WholeEthosImport,
    WholeEthosImports, WholeEthosInterfaceBody, WholeEthosItem, WholeEthosNewtype,
    WholeEthosNexusBody, WholeEthosQuality, WholeEthosReferencePriorPosition, WholeEthosSemaBody,
    WholeEthosSemaTableKey, WholeEthosStreamInitiation, WholeEthosStruct, WholeEthosTable,
    WholeEthosTrait, WholeEthosTupleFields, WholeEthosTypeApplication, WholeEthosTypeParameter,
    WholeEthosTypeReference, WholeEthosVariant, WholeEthosVariantPayload, WholeEthosVisibility,
    WholeEthosWrappedField,
};
