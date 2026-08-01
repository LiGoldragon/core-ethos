//! # core-ethos
//!
//! The stringless encoded Ethos carriers for the protos language family.
//!
//! [`whole`] is the sole textual and structural surface. It round-trips types-only
//! Ethos files through the canonical full-chain `structural-codec` contract,
//! receiving declaration assignments and lookup-only references from the naming
//! authority. The older flat declaration algebra remains available only as sealed
//! execution data while its consumers migrate; it is not a textual authoring path.

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
    DecodedEncodedIdPosition, DecodedEthos, EmptyTupleFields, EthosCodec, EthosCodecBuildError,
    EthosDecodeError, EthosEncodeError, EthosGrammarError, EthosGrammarIdPosition, EthosGrammarIds,
    WholeEthos, WholeEthosArchiveError, WholeEthosAttributes, WholeEthosBuiltinPriorError,
    WholeEthosBuiltinPriorPosition, WholeEthosBuiltinPriors, WholeEthosEncodedIdPosition,
    WholeEthosEnumeration, WholeEthosItem, WholeEthosNewtype, WholeEthosReferencePriorPosition,
    WholeEthosTupleFields, WholeEthosTypeApplication, WholeEthosTypeReference, WholeEthosVariant,
    WholeEthosVariantPayload, WholeEthosVisibility, WholeEthosWrappedField,
};
