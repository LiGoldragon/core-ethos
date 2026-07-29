//! # core-ethos
//!
//! The first REAL stringless Encoded Ethos layer of the next-generation NOTA family,
//! and the first REAL Textual form ([`TextualEthos`]).
//!
//! Slice one delivered four foundation crates — `content-identity`, `name-table`,
//! `raw-discovery`, `structural-codec` — with a synthetic fixture universe whose
//! ids keyed no real Encoded layout. This crate makes that layer real:
//!
//! - **Stringless `EncodedEthos` value types** ([`declaration`], [`mod@reference`])
//!   modelled on `schema-language`'s `EncodedType { Struct | Enum | Newtype }`: every
//!   name is an [`Identifier`](name_table::Identifier) into the `NameTable`, and
//!   type references dispatch by kind and projection, never a head string. Content
//!   identity is blake3 over the stringless rkyv bytes with the NameTable excluded,
//!   so a rename is hash-stable by construction.
//! - **The universe bridge** ([`universe`]): a set of `EncodedEthos` declarations
//!   forms a `structural-codec` Encoded universe — one [`ScopedEncodedTypeId`] per type,
//!   one constructor id per constructor, and typed record metadata derived from
//!   the Encoded layout. This closes `structural-codec`'s deferred
//!   signature-vs-Encoded deviation: [`EncodedUniverse::validate_table`] proves
//!   every authored codec signature equals the Encoded field signature, and a
//!   mismatch fails loudly.
//! - **`TextualEthos`** ([`textual`]): real Ethos text decodes — through
//!   raw-discovery and the trusted evaluator — into real `EncodedEthos` values with a
//!   real `NameTable`, and encodes back canonically. The derived-name rule (a field
//!   name elided when it equals the `snake_case` of its type) works against the real
//!   Encoded layout.
//!
//! This crate is greenfield by design. It models the proven Encoded shapes of the
//! existing `schema-language`/`schema`/`schema-rust` repositories in the new
//! stringless discipline; convergence with those repositories happens later on the
//! release train and readapts to it. See `ARCHITECTURE.md`.
//!
//! [`ScopedEncodedTypeId`]: structural_codec::ids::ScopedEncodedTypeId

pub mod capsule;
pub mod declaration;
pub mod document;
pub mod error;
pub mod fixture;
pub mod reference;
pub mod rules;
pub mod slice_one;
pub mod textual;
pub mod universe;

pub use capsule::capsule_from_issued_hash;
pub use declaration::{
    DeclarationRole, EncodedDeclaration, EncodedEnum, EncodedEthos, EncodedEthosDomain,
    EncodedField, EncodedNewtype, EncodedStruct, EncodedType, EncodedVariant, StreamingRelation,
    Visibility,
};
pub use document::{
    DOCUMENT_SLOTS, DeclarationConstructor, EthosDocumentGrammar, ReferenceConstructor,
};
pub use error::{
    EncodedEthosError, EncodedEthosLoadError, EncodedIdentityError, StreamingReferenceForm,
    StreamingRelationReference, StructuralRedefinition, TextualError, UniverseError,
};
pub use fixture::FixtureFamily;
pub use reference::{
    BuiltinReference, EncodedReference, MultiTypeReferenceProjection,
    SingleTypeReferenceProjection, ValueReferenceProjection,
};
pub use slice_one::{
    DecodedEncodedIdPosition, DecodedSixSlotEthos, EmptyTupleFields, SixSlotCodecBuildError,
    SixSlotDecodeError, SixSlotEthosCodec, SixSlotGrammarError, SixSlotGrammarIdPosition,
    SixSlotGrammarIds, SixSlotSourceBounds, SliceOneBuiltinPriorError,
    SliceOneBuiltinPriorPosition, SliceOneBuiltinPriors, SliceOneReferencePriorPosition,
    WholeEthos, WholeEthosArchiveError, WholeEthosAttributes, WholeEthosEncodedIdPosition,
    WholeEthosEnumeration, WholeEthosItem, WholeEthosNewtype, WholeEthosTupleFields,
    WholeEthosTypeApplication, WholeEthosTypeReference, WholeEthosVariant,
    WholeEthosVariantPayload, WholeEthosVisibility, WholeEthosWrappedField,
};
pub use textual::{EthosLanguage, TextualEthos};
pub use universe::{
    AssignedKind, AssignedMember, ENCODED_UNIVERSE, EncodedUniverse, EncodedUniverseBuilder,
    MemberKind, ScalarSlot, UniverseType,
};
