//! # core-ethos
//!
//! The stringless encoded Ethos carriers for the protos language family.
//!
//! [`whole`] is the sole textual and structural surface. It decodes composite
//! Interface, Nexus, and Sema documents and emits their encoded meaning through
//! the canonical full-chain `structural-codec` contract, receiving declaration
//! assignments and lookup-only references from the naming authority. The older
//! flat declaration algebra remains available only as sealed execution data while
//! its consumers migrate; it is not a textual authoring path.

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
    WholeEthosStreamInitiation, WholeEthosStruct, WholeEthosTable, WholeEthosTrait,
    WholeEthosTupleFields, WholeEthosTypeApplication, WholeEthosTypeParameter,
    WholeEthosTypeReference, WholeEthosVariant, WholeEthosVariantPayload, WholeEthosVisibility,
    WholeEthosWrappedField,
};
