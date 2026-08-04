//! Whole-Ethos document decoding over translator-issued encoded-ID chains.
//!
//! Every source document is a header, imports object, and kind-selected body.
//! The shared structural evaluator first decodes the header, then the header's
//! kind selects one addressed document root for the complete decode. File kinds
//! differ only by that root type and body record.

use content_identity::{ArchiveError, PortableArchive};
use nomos_types::StreamInitiation;
use raw_discovery::{
    BlockTree, BlockTreeDiscoveryConfiguration, BoundaryDiscoveryConfiguration,
    BoundaryDiscoveryContext, BoundaryDiscoveryContextIdentifier, BoundaryDiscoveryTransition,
    DiscoveredBlockTree, SealedTokenProfile, TriggerIdentifier, TriggerSet,
};
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::{
    AcceptedDecodeForm, AddressedStructuralTable, AdjacentApplicationDelimitedBody,
    AdjacentApplicationDelimitedHead, AdjacentApplicationDelimitedItems,
    AdjacentApplicationDelimitedRoot, AdjacentApplicationDelimitedRule, ApplicationDelimitedBody,
    ApplicationDelimitedHead, ApplicationDelimitedItems, ApplicationDelimitedRoot,
    ApplicationDelimitedRule, ApplicationHead, ApplicationPayload, ApplicationRoot,
    ApplicationRule, AtomCase, AtomDescriptor, BorrowedFieldView, ConstructorCodec,
    ContextualTextualPolicy, DeclarationAssignment, DecodeFormId, DecodeNameBindings,
    EncodedConstructorId, EncodedTypeId, FieldRole, FieldValue, FieldVisitor, LeafCodec,
    OrderedProduct, OrderedSequence, Position, ResolvedReference, RuleCoproduct, ScalarValue,
    SharedDescriptor, StableRoleId, StructuralEntry, StructuralEvaluator, StructuralRule,
    StructuralValue, StructuralVocabularyIdentity, StructureRecord, TableIdentityPayload,
    TargetLayoutIdentity, TextualRenderingPolicy, UnaryRoot, UnaryRule,
};

const SQUARE_BOUNDARY: TriggerIdentifier = TriggerIdentifier::new(1);
const BRACE_BOUNDARY: TriggerIdentifier = TriggerIdentifier::new(2);
const APPLICATION_OPERATOR: TriggerIdentifier = TriggerIdentifier::new(3);
const PARENTHESIS_BOUNDARY: TriggerIdentifier = TriggerIdentifier::new(0);
const TEXT_CARRIER: TriggerIdentifier = TriggerIdentifier::new(4);
const WHITESPACE_TRIVIA: TriggerIdentifier = TriggerIdentifier::new(5);
const ANGLE_BOUNDARY: TriggerIdentifier = TriggerIdentifier::new(7);
const ROOT_CONTEXT: BoundaryDiscoveryContextIdentifier = BoundaryDiscoveryContextIdentifier::new(1);
const CHILD_CONTEXT: BoundaryDiscoveryContextIdentifier =
    BoundaryDiscoveryContextIdentifier::new(2);
const SUPPORTED_VERSION: u64 = 1;

/// A supported specialized Ethos file kind.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub enum WholeEthosFileKind {
    /// Public wire vocabulary and its universal role memberships.
    Interface,
    /// Internal operand types and behavior traits.
    Nexus,
    /// Private record types and tables.
    Sema,
}

impl WholeEthosFileKind {
    /// Canonical textual spelling in the header.
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Interface => "Interface",
            Self::Nexus => "Nexus",
            Self::Sema => "Sema",
        }
    }

    fn from_spelling(spelling: &str) -> Option<Self> {
        match spelling {
            "Interface" => Some(Self::Interface),
            "Nexus" => Some(Self::Nexus),
            "Sema" => Some(Self::Sema),
            _ => None,
        }
    }
}

/// The typed two-position document header.
///
/// `[assumption primary-vq6.1-A1 — fixture version projection]`: the reviewed
/// fixtures use the single integer spelling `.1`. It is retained as the first
/// SemVer-style compatibility generation until the later version standard
/// defines a wider encoded version anatomy.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosHeader {
    kind: WholeEthosFileKind,
    version: u64,
}

impl WholeEthosHeader {
    /// Construct a supported header.
    pub const fn new(
        kind: WholeEthosFileKind,
        version: u64,
    ) -> Result<Self, WholeEthosArchiveError> {
        if version == SUPPORTED_VERSION {
            Ok(Self { kind, version })
        } else {
            Err(WholeEthosArchiveError::UnsupportedVersion {
                kind,
                found: version,
                supported: SUPPORTED_VERSION,
            })
        }
    }

    /// Specialized file kind.
    pub const fn kind(&self) -> WholeEthosFileKind {
        self.kind
    }

    /// Compatibility generation accepted by this codec.
    pub const fn version(&self) -> u64 {
        self.version
    }
}

/// Encoded Whole-Ethos document: header plus the selected body.
///
/// Imports do not appear here because they are source-only. Their typed source
/// representation remains on [`DecodedEthos`].
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthos {
    header: WholeEthosHeader,
    body: WholeEthosBody,
}

impl structural_codec::EncodedForm for WholeEthos {
    type VocabularyRoot = VocabularyRoot;
    type Language = protos::Ethos;
}

/// The semantic document capability implemented by [`WholeEthos`].
pub trait EthosDocument {
    /// Typed header.
    fn header(&self) -> &WholeEthosHeader;
    /// Kind-selected body.
    fn body(&self) -> &WholeEthosBody;
}

impl EthosDocument for WholeEthos {
    fn header(&self) -> &WholeEthosHeader {
        &self.header
    }

    fn body(&self) -> &WholeEthosBody {
        &self.body
    }
}

// Trait exception — too trivial: these methods provide constructor and archive
// ergonomics around the EthosDocument contract.
impl WholeEthos {
    /// Construct a typed document after enforcing all carrier invariants.
    pub fn new(
        header: WholeEthosHeader,
        body: WholeEthosBody,
    ) -> Result<Self, WholeEthosArchiveError> {
        let ethos = Self { header, body };
        ethos.validate()?;
        Ok(ethos)
    }

    /// Typed header.
    pub const fn header(&self) -> &WholeEthosHeader {
        &self.header
    }

    /// Kind-selected body.
    pub const fn body(&self) -> &WholeEthosBody {
        &self.body
    }

    /// Serialize the complete encoded carrier. Source-only imports are absent.
    pub fn to_archive_bytes(&self) -> Result<Vec<u8>, WholeEthosArchiveError> {
        self.validate()?;
        Ok(<Self as PortableArchive>::to_archive_bytes(self)?
            .as_ref()
            .to_vec())
    }

    /// Restore a carrier after archive and identity validation.
    pub fn from_archive_bytes(bytes: &[u8]) -> Result<Self, WholeEthosArchiveError> {
        let restored = <Self as PortableArchive>::from_archive_bytes(bytes)?;
        restored.validate()?;
        Ok(restored)
    }

    fn validate(&self) -> Result<(), WholeEthosArchiveError> {
        if self.header.version != SUPPORTED_VERSION {
            return Err(WholeEthosArchiveError::UnsupportedVersion {
                kind: self.header.kind,
                found: self.header.version,
                supported: SUPPORTED_VERSION,
            });
        }
        if self.header.kind != self.body.kind() {
            return Err(WholeEthosArchiveError::HeaderBodyKindMismatch {
                header: self.header.kind,
                body: self.body.kind(),
            });
        }
        self.body.validate()
    }
}

/// One of the three specialized body roots.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum WholeEthosBody {
    /// Interface inputs, outputs, refusals, and shared types.
    Interface(WholeEthosInterfaceBody),
    /// Nexus operand types and traits.
    Nexus(WholeEthosNexusBody),
    /// Sema record types and tables.
    Sema(WholeEthosSemaBody),
}

impl WholeEthosBody {
    /// Kind selected by this body variant.
    pub const fn kind(&self) -> WholeEthosFileKind {
        match self {
            Self::Interface(_) => WholeEthosFileKind::Interface,
            Self::Nexus(_) => WholeEthosFileKind::Nexus,
            Self::Sema(_) => WholeEthosFileKind::Sema,
        }
    }

    fn validate(&self) -> Result<(), WholeEthosArchiveError> {
        match self {
            Self::Interface(body) => body.validate(),
            Self::Nexus(body) => body.validate(),
            Self::Sema(body) => body.validate(),
        }
    }
}

/// Typed source-only imports retained beside an encoded document.
///
/// These entries participate in textual reconstruction but are deliberately
/// absent from [`WholeEthos`] archives.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WholeEthosImports {
    entries: Vec<WholeEthosImport>,
}

impl WholeEthosImports {
    fn new(entries: Vec<WholeEthosImport>) -> Self {
        Self { entries }
    }

    /// Imports in authored order.
    pub fn entries(&self) -> &[WholeEthosImport] {
        &self.entries
    }
}

/// One source module and its imported names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WholeEthosImport {
    source: String,
    names: Vec<String>,
}

impl WholeEthosImport {
    fn new(source: String, names: Vec<String>) -> Self {
        Self { source, names }
    }

    /// Source module spelling retained as source-only text.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Imported name spellings in authored order.
    pub fn names(&self) -> &[String] {
        &self.names
    }
}

/// Interface body positions: inputs, outputs, refusals, shared types.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosInterfaceBody {
    inputs: Vec<WholeEthosNewtype>,
    outputs: Vec<WholeEthosNewtype>,
    refusals: Vec<WholeEthosStruct>,
    types: Vec<WholeEthosItem>,
}

impl WholeEthosInterfaceBody {
    /// Construct the four positional interface roles.
    pub fn new(
        inputs: Vec<WholeEthosNewtype>,
        outputs: Vec<WholeEthosNewtype>,
        refusals: Vec<WholeEthosStruct>,
        types: Vec<WholeEthosItem>,
    ) -> Self {
        Self {
            inputs,
            outputs,
            refusals,
            types,
        }
    }

    /// Types falling under the universal Input trait.
    pub fn inputs(&self) -> &[WholeEthosNewtype] {
        &self.inputs
    }

    /// Types falling under the universal Output trait.
    pub fn outputs(&self) -> &[WholeEthosNewtype] {
        &self.outputs
    }

    /// Public refusal products.
    pub fn refusals(&self) -> &[WholeEthosStruct] {
        &self.refusals
    }

    /// Shared wire vocabulary.
    pub fn types(&self) -> &[WholeEthosItem] {
        &self.types
    }

    fn validate(&self) -> Result<(), WholeEthosArchiveError> {
        validate_newtypes(&self.inputs)?;
        validate_newtypes(&self.outputs)?;
        validate_structs(&self.refusals)?;
        validate_items(&self.types)
    }
}

/// Nexus body positions: operand types and behavior traits.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosNexusBody {
    types: Vec<WholeEthosItem>,
    traits: Vec<WholeEthosTrait>,
}

impl WholeEthosNexusBody {
    /// Construct the two positional nexus sections.
    pub fn new(types: Vec<WholeEthosItem>, traits: Vec<WholeEthosTrait>) -> Self {
        Self { types, traits }
    }

    /// Operand and decision vocabulary.
    pub fn types(&self) -> &[WholeEthosItem] {
        &self.types
    }

    /// Behavior traits.
    pub fn traits(&self) -> &[WholeEthosTrait] {
        &self.traits
    }

    fn validate(&self) -> Result<(), WholeEthosArchiveError> {
        validate_items(&self.types)?;
        for trait_definition in &self.traits {
            trait_definition.validate()?;
        }
        Ok(())
    }
}

/// Sema body positions: record types and tables.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosSemaBody {
    record_types: Vec<WholeEthosItem>,
    tables: Vec<WholeEthosTable>,
}

impl WholeEthosSemaBody {
    /// Construct private record types and tables.
    pub fn new(record_types: Vec<WholeEthosItem>, tables: Vec<WholeEthosTable>) -> Self {
        Self {
            record_types,
            tables,
        }
    }

    /// Private record products.
    pub fn record_types(&self) -> &[WholeEthosItem] {
        &self.record_types
    }

    /// `table.{Record Key}` declarations with the operator supplied by position.
    pub fn tables(&self) -> &[WholeEthosTable] {
        &self.tables
    }

    fn validate(&self) -> Result<(), WholeEthosArchiveError> {
        validate_items(&self.record_types)?;
        for table in &self.tables {
            table.validate()?;
        }
        Ok(())
    }
}

/// Declaration alternatives admitted in an ordinary types position.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum WholeEthosItem {
    /// One newtype declaration.
    Newtype(WholeEthosNewtype),
    /// One enumeration declaration.
    Enumeration(WholeEthosEnumeration),
    /// One positional struct declaration.
    Struct(WholeEthosStruct),
    /// One standalone stream-initiation transformer, such as
    /// `Observer.Stream.(ObserverFilter ObservationEvent)`.
    StreamInitiation(WholeEthosStreamInitiation),
}

/// Stream-initiation source schema over Whole-Ethos identities and references.
///
/// The stream identity is declared by the outer name; query and event retain
/// their distinct Nomos roles without admitting a universal operator-payload
/// carrier. The separate termination input is implied by this declaration and
/// generated by the lifecycle Nomos rather than repeated in source.
pub type WholeEthosStreamInitiation =
    StreamInitiation<VocabularyEncodedId, WholeEthosTypeReference, WholeEthosTypeReference>;

/// A newtype declaration.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosNewtype {
    name: VocabularyEncodedId,
    visibility: WholeEthosVisibility,
    attributes: WholeEthosAttributes,
    type_parameters: Vec<WholeEthosTypeParameter>,
    wrapped_field: WholeEthosWrappedField,
}

impl WholeEthosNewtype {
    /// Construct one newtype.
    pub fn new(
        name: VocabularyEncodedId,
        visibility: WholeEthosVisibility,
        attributes: WholeEthosAttributes,
        wrapped_field: WholeEthosWrappedField,
    ) -> Self {
        Self {
            name,
            visibility,
            attributes,
            type_parameters: type_parameters_for_reference(wrapped_field.reference()),
            wrapped_field,
        }
    }

    /// Translator-issued declaration identity.
    pub const fn name(&self) -> &VocabularyEncodedId {
        &self.name
    }

    /// Item visibility.
    pub const fn visibility(&self) -> &WholeEthosVisibility {
        &self.visibility
    }

    /// Typed attribute sequence, currently empty.
    pub const fn attributes(&self) -> &WholeEthosAttributes {
        &self.attributes
    }

    /// Trait-quality parameters picked up by uses in the wrapped field.
    pub fn type_parameters(&self) -> &[WholeEthosTypeParameter] {
        &self.type_parameters
    }

    /// Wrapped field.
    pub const fn wrapped_field(&self) -> &WholeEthosWrappedField {
        &self.wrapped_field
    }
}

/// One positional struct declaration.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosStruct {
    name: VocabularyEncodedId,
    fields: Vec<WholeEthosTypeReference>,
}

impl WholeEthosStruct {
    /// Construct a positional product. Empty products are not admitted by this slice.
    pub fn new(
        name: VocabularyEncodedId,
        fields: Vec<WholeEthosTypeReference>,
    ) -> Result<Self, EmptyStructFields> {
        if fields.is_empty() {
            Err(EmptyStructFields)
        } else {
            Ok(Self { name, fields })
        }
    }

    /// Declaration identity.
    pub const fn name(&self) -> &VocabularyEncodedId {
        &self.name
    }

    /// Positional field types.
    pub fn fields(&self) -> &[WholeEthosTypeReference] {
        &self.fields
    }
}

/// One enumeration declaration.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosEnumeration {
    name: VocabularyEncodedId,
    visibility: WholeEthosVisibility,
    attributes: WholeEthosAttributes,
    variants: Vec<WholeEthosVariant>,
}

impl WholeEthosEnumeration {
    /// Construct an attribute-free enumeration.
    pub fn new(
        name: VocabularyEncodedId,
        visibility: WholeEthosVisibility,
        attributes: WholeEthosAttributes,
        variants: Vec<WholeEthosVariant>,
    ) -> Result<Self, EmptyEnumerationVariants> {
        if variants.is_empty() {
            Err(EmptyEnumerationVariants)
        } else {
            Ok(Self {
                name,
                visibility,
                attributes,
                variants,
            })
        }
    }

    /// Declaration identity.
    pub const fn name(&self) -> &VocabularyEncodedId {
        &self.name
    }

    /// Item visibility.
    pub const fn visibility(&self) -> &WholeEthosVisibility {
        &self.visibility
    }

    /// Typed empty attribute sequence.
    pub const fn attributes(&self) -> &WholeEthosAttributes {
        &self.attributes
    }

    /// Variants in authored order.
    pub fn variants(&self) -> &[WholeEthosVariant] {
        &self.variants
    }
}

/// One enumeration variant.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosVariant {
    name: VocabularyEncodedId,
    attributes: WholeEthosAttributes,
    payload: WholeEthosVariantPayload,
}

impl WholeEthosVariant {
    /// Construct one variant.
    pub fn new(
        name: VocabularyEncodedId,
        attributes: WholeEthosAttributes,
        payload: WholeEthosVariantPayload,
    ) -> Self {
        Self {
            name,
            attributes,
            payload,
        }
    }

    /// Variant declaration identity.
    pub const fn name(&self) -> &VocabularyEncodedId {
        &self.name
    }

    /// Typed empty attribute sequence.
    pub const fn attributes(&self) -> &WholeEthosAttributes {
        &self.attributes
    }

    /// Unit or positional payload.
    pub const fn payload(&self) -> &WholeEthosVariantPayload {
        &self.payload
    }
}

/// Enumeration payload vocabulary.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum WholeEthosVariantPayload {
    /// Unit variant.
    Unit,
    /// One or more positional fields.
    Tuple(WholeEthosTupleFields),
}

/// Positional variant fields.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosTupleFields(Vec<WholeEthosTypeReference>);

impl WholeEthosTupleFields {
    /// Construct a non-empty positional payload.
    pub fn new(fields: Vec<WholeEthosTypeReference>) -> Result<Self, EmptyTupleFields> {
        if fields.is_empty() {
            Err(EmptyTupleFields)
        } else {
            Ok(Self(fields))
        }
    }

    /// Positional payload fields.
    pub fn fields(&self) -> &[WholeEthosTypeReference] {
        &self.0
    }
}

/// A tuple-payload construction attempted to encode no fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("tuple variant payload requires at least one positional field")]
pub struct EmptyTupleFields;

/// A struct construction attempted to encode no positional fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("struct declaration requires at least one positional field")]
pub struct EmptyStructFields;

/// An enumeration construction attempted to encode no variants.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("enumeration declaration requires at least one variant")]
pub struct EmptyEnumerationVariants;

/// One behavior trait declaration.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosTrait {
    name: VocabularyEncodedId,
}

impl WholeEthosTrait {
    /// Construct a behavior-trait identity.
    ///
    /// Trait method signatures deliberately have no sectioned `name.(...)`
    /// source form. Their future strict transformer schemas belong to the
    /// standalone transformer surface instead of a trait-local application.
    pub const fn new(name: VocabularyEncodedId) -> Self {
        Self { name }
    }

    /// Trait declaration identity.
    pub const fn name(&self) -> &VocabularyEncodedId {
        &self.name
    }

    fn validate(&self) -> Result<(), WholeEthosArchiveError> {
        validate_identity(&self.name, WholeEthosEncodedIdPosition::TraitName)
    }
}

/// One sema table declaration. The section supplies the table operator.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosTable {
    name: VocabularyEncodedId,
    record: WholeEthosTypeReference,
    key: WholeEthosTypeReference,
}

impl WholeEthosTable {
    /// Construct `name.{Record Key}`.
    pub fn new(
        name: VocabularyEncodedId,
        record: WholeEthosTypeReference,
        key: WholeEthosTypeReference,
    ) -> Self {
        Self { name, record, key }
    }

    /// Table declaration identity.
    pub const fn name(&self) -> &VocabularyEncodedId {
        &self.name
    }

    /// Stored record type.
    pub const fn record(&self) -> &WholeEthosTypeReference {
        &self.record
    }

    /// Key type.
    pub const fn key(&self) -> &WholeEthosTypeReference {
        &self.key
    }

    fn validate(&self) -> Result<(), WholeEthosArchiveError> {
        validate_identity(&self.name, WholeEthosEncodedIdPosition::TableName)?;
        validate_reference(&self.record)?;
        validate_reference(&self.key)
    }
}

/// A positional type reference.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum WholeEthosTypeReference {
    /// One complete lookup-resolved identity.
    Identity(VocabularyEncodedId),
    /// A use of an item-local type parameter picked up from a trait quality.
    Parameter(WholeEthosTypeParameter),
    /// A non-empty adjacent angle application such as `Vector<Topic>`.
    Application(WholeEthosTypeApplication),
}

/// The semantic role carried by a type identity.
///
/// Shapes head type applications, while traits constrain item-local parameters.
/// Keeping the distinction in the carrier prevents downstream transformations
/// from inferring a role from an untyped identity.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum WholeEthosQuality {
    /// An identity used as an applied type shape.
    Shape(VocabularyEncodedId),
    /// An identity used as a type-parameter trait bound.
    Trait(VocabularyEncodedId),
}

impl WholeEthosQuality {
    /// The complete identity retained with this quality role.
    pub const fn identity(&self) -> &VocabularyEncodedId {
        match self {
            Self::Shape(identity) | Self::Trait(identity) => identity,
        }
    }
}

/// One picked-up parameter and its authored trait-quality bound.
///
/// The parameter name and quality are both complete concept-layer identities.
/// Rust-only spelling translation is deliberately outside this carrier.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosTypeParameter {
    name: VocabularyEncodedId,
    quality: WholeEthosQuality,
}

impl WholeEthosTypeParameter {
    /// Construct one parameter with an explicit trait-quality bound.
    pub fn new(name: VocabularyEncodedId, quality: WholeEthosQuality) -> Self {
        Self { name, quality }
    }

    /// The retained proper parameter name.
    pub const fn name(&self) -> &VocabularyEncodedId {
        &self.name
    }

    /// The authored trait quality constraining this parameter.
    pub const fn quality(&self) -> &WholeEthosQuality {
        &self.quality
    }
}

fn type_parameters_for_reference(
    reference: &WholeEthosTypeReference,
) -> Vec<WholeEthosTypeParameter> {
    let mut parameters = Vec::new();
    collect_type_parameters(reference, &mut parameters);
    parameters
}

fn collect_type_parameters(
    reference: &WholeEthosTypeReference,
    parameters: &mut Vec<WholeEthosTypeParameter>,
) {
    match reference {
        WholeEthosTypeReference::Identity(_) => {}
        WholeEthosTypeReference::Parameter(parameter) => {
            if !parameters.contains(parameter) {
                parameters.push(parameter.clone());
            }
        }
        WholeEthosTypeReference::Application(application) => {
            for argument in application.arguments() {
                collect_type_parameters(argument, parameters);
            }
        }
    }
}

/// One non-empty adjacent angle type application.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[rkyv(
    serialize_bounds(__S: rkyv::ser::Writer + rkyv::ser::Allocator, __S::Error: rkyv::rancor::Source),
    deserialize_bounds(__D::Error: rkyv::rancor::Source),
    bytecheck(bounds(__C: rkyv::validation::ArchiveContext, __C::Error: rkyv::rancor::Source)),
)]
pub struct WholeEthosTypeApplication {
    head: WholeEthosQuality,
    #[rkyv(omit_bounds)]
    arguments: Vec<WholeEthosTypeReference>,
}

impl WholeEthosTypeApplication {
    /// Construct a type application with one or more arguments.
    pub fn new(
        head: WholeEthosQuality,
        arguments: Vec<WholeEthosTypeReference>,
    ) -> Result<Self, EmptyTypeArguments> {
        if arguments.is_empty() {
            Err(EmptyTypeArguments)
        } else {
            Ok(Self { head, arguments })
        }
    }

    /// Lookup-resolved application head.
    pub const fn head(&self) -> &WholeEthosQuality {
        &self.head
    }

    /// Application arguments in authored order.
    pub fn arguments(&self) -> &[WholeEthosTypeReference] {
        &self.arguments
    }
}

/// A type application construction attempted to encode no arguments.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("type application requires at least one argument")]
pub struct EmptyTypeArguments;

/// Visibility vocabulary retained by downstream Logos transformations.
#[derive(Clone, Copy, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum WholeEthosVisibility {
    /// Exported item.
    Public,
    /// Private wrapped field.
    Private,
}

/// Typed empty attribute sequence admitted by this codec.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub struct WholeEthosAttributes;

impl WholeEthosAttributes {
    /// Construct the only admitted attribute sequence.
    pub const fn empty() -> Self {
        Self
    }

    /// This sequence currently contains no attributes.
    pub const fn is_empty(self) -> bool {
        true
    }
}

/// A newtype field with visibility and referenced type identity.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosWrappedField {
    visibility: WholeEthosVisibility,
    reference: WholeEthosTypeReference,
}

impl WholeEthosWrappedField {
    /// Construct the wrapped field.
    pub fn new(visibility: WholeEthosVisibility, reference: WholeEthosTypeReference) -> Self {
        Self {
            visibility,
            reference,
        }
    }

    /// Wrapped-field visibility.
    pub const fn visibility(&self) -> &WholeEthosVisibility {
        &self.visibility
    }

    /// Complete lookup-resolved type identity.
    pub const fn reference(&self) -> &WholeEthosTypeReference {
        &self.reference
    }
}

fn validate_items(items: &[WholeEthosItem]) -> Result<(), WholeEthosArchiveError> {
    for item in items {
        match item {
            WholeEthosItem::Newtype(newtype) => validate_newtype(newtype)?,
            WholeEthosItem::Enumeration(enumeration) => validate_enumeration(enumeration)?,
            WholeEthosItem::Struct(product) => validate_struct(product)?,
            WholeEthosItem::StreamInitiation(initiation) => {
                validate_identity(
                    &initiation.stream,
                    WholeEthosEncodedIdPosition::ApplicationName,
                )?;
                validate_references(&[initiation.query.clone(), initiation.event.clone()])?;
            }
        }
    }
    Ok(())
}

fn validate_newtypes(newtypes: &[WholeEthosNewtype]) -> Result<(), WholeEthosArchiveError> {
    for newtype in newtypes {
        validate_newtype(newtype)?;
    }
    Ok(())
}

fn validate_newtype(newtype: &WholeEthosNewtype) -> Result<(), WholeEthosArchiveError> {
    if newtype.visibility != WholeEthosVisibility::Public {
        return Err(WholeEthosArchiveError::InvalidItemVisibility);
    }
    if newtype.wrapped_field.visibility != WholeEthosVisibility::Private {
        return Err(WholeEthosArchiveError::InvalidWrappedFieldVisibility);
    }
    validate_identity(&newtype.name, WholeEthosEncodedIdPosition::ItemName)?;
    validate_reference(&newtype.wrapped_field.reference)?;
    validate_type_parameters(&newtype.type_parameters)?;
    if newtype.type_parameters != type_parameters_for_reference(&newtype.wrapped_field.reference) {
        return Err(WholeEthosArchiveError::TypeParameterPickupMismatch);
    }
    Ok(())
}

fn validate_type_parameters(
    parameters: &[WholeEthosTypeParameter],
) -> Result<(), WholeEthosArchiveError> {
    for (index, parameter) in parameters.iter().enumerate() {
        validate_identity(
            parameter.name(),
            WholeEthosEncodedIdPosition::TypeParameterName,
        )?;
        let WholeEthosQuality::Trait(quality) = parameter.quality() else {
            return Err(WholeEthosArchiveError::TypeParameterQualityMustBeTrait {
                quality: parameter.quality().identity().clone(),
            });
        };
        validate_identity(quality, WholeEthosEncodedIdPosition::TypeParameterQuality)?;
        if let Some(prior) = parameters[..index]
            .iter()
            .find(|prior| prior.name() == parameter.name())
        {
            if prior.quality() != parameter.quality() {
                return Err(WholeEthosArchiveError::ConflictingTypeParameterQuality {
                    name: parameter.name().clone(),
                });
            }
            return Err(WholeEthosArchiveError::DuplicateTypeParameterName {
                name: parameter.name().clone(),
            });
        }
    }
    Ok(())
}

fn validate_structs(products: &[WholeEthosStruct]) -> Result<(), WholeEthosArchiveError> {
    for product in products {
        validate_struct(product)?;
    }
    Ok(())
}

fn validate_struct(product: &WholeEthosStruct) -> Result<(), WholeEthosArchiveError> {
    if product.fields.is_empty() {
        return Err(WholeEthosArchiveError::EmptyStructFields);
    }
    validate_identity(&product.name, WholeEthosEncodedIdPosition::ItemName)?;
    validate_references(&product.fields)
}

fn validate_enumeration(enumeration: &WholeEthosEnumeration) -> Result<(), WholeEthosArchiveError> {
    if enumeration.variants.is_empty() {
        return Err(WholeEthosArchiveError::EmptyEnumerationVariants);
    }
    if enumeration.visibility != WholeEthosVisibility::Public {
        return Err(WholeEthosArchiveError::InvalidItemVisibility);
    }
    validate_identity(&enumeration.name, WholeEthosEncodedIdPosition::ItemName)?;
    for variant in &enumeration.variants {
        validate_identity(&variant.name, WholeEthosEncodedIdPosition::VariantName)?;
        if let WholeEthosVariantPayload::Tuple(fields) = &variant.payload {
            if fields.fields().is_empty() {
                return Err(WholeEthosArchiveError::EmptyTupleFields);
            }
            validate_references(fields.fields())?;
        }
    }
    Ok(())
}

fn validate_references(
    references: &[WholeEthosTypeReference],
) -> Result<(), WholeEthosArchiveError> {
    for reference in references {
        validate_reference(reference)?;
    }
    Ok(())
}

fn validate_reference(reference: &WholeEthosTypeReference) -> Result<(), WholeEthosArchiveError> {
    match reference {
        WholeEthosTypeReference::Identity(identity) => {
            validate_identity(identity, WholeEthosEncodedIdPosition::Reference)
        }
        WholeEthosTypeReference::Parameter(parameter) => {
            validate_type_parameters(std::slice::from_ref(parameter))
        }
        WholeEthosTypeReference::Application(application) => {
            let WholeEthosQuality::Shape(head) = &application.head else {
                return Err(WholeEthosArchiveError::TypeApplicationHeadMustBeShape {
                    quality: application.head.identity().clone(),
                });
            };
            validate_identity(head, WholeEthosEncodedIdPosition::ApplicationHead)?;
            if application.arguments.is_empty() {
                return Err(WholeEthosArchiveError::EmptyTypeArguments);
            }
            validate_references(&application.arguments)
        }
    }
}

fn validate_identity(
    encoded_id: &VocabularyEncodedId,
    position: WholeEthosEncodedIdPosition,
) -> Result<(), WholeEthosArchiveError> {
    if encoded_id.root_variant() != &VocabularyRoot::Universal {
        return Err(WholeEthosArchiveError::NonUniversalEncodedId {
            position,
            root: *encoded_id.root_variant(),
        });
    }
    if encoded_id.chain().is_empty() {
        return Err(WholeEthosArchiveError::EmptyEncodedId { position });
    }
    Ok(())
}

/// Encoded-ID position rejected at the archive boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WholeEthosEncodedIdPosition {
    /// Item declaration.
    ItemName,
    /// Enumeration variant.
    VariantName,
    /// Type reference.
    Reference,
    /// Unary or object application head.
    ApplicationHead,
    /// Picked-up parameter name.
    TypeParameterName,
    /// Trait quality attached to a picked-up parameter.
    TypeParameterQuality,
    /// Authored object application name.
    ApplicationName,
    /// Trait declaration.
    TraitName,
    /// Table declaration.
    TableName,
}

/// Failure at an encoded Whole-Ethos construction, serialization, or archive boundary.
#[derive(Clone, Debug, thiserror::Error)]
pub enum WholeEthosArchiveError {
    /// Canonical archive serialization or validated reconstruction failed.
    #[error("whole-Ethos portable archive failed: {0}")]
    Archive(#[from] ArchiveError),

    /// Stored header and body select different kinds.
    #[error("Whole-Ethos header kind {header:?} does not match body kind {body:?}")]
    HeaderBodyKindMismatch {
        /// Header kind.
        header: WholeEthosFileKind,
        /// Body variant kind.
        body: WholeEthosFileKind,
    },

    /// Stored version is not accepted by this codec.
    #[error("Whole-Ethos {kind:?} version {found} is unsupported; expected {supported}")]
    UnsupportedVersion {
        /// File kind.
        kind: WholeEthosFileKind,
        /// Stored generation.
        found: u64,
        /// Accepted generation.
        supported: u64,
    },

    /// A struct cannot be represented by the grammar without fields.
    #[error("Whole-Ethos struct declaration has no positional fields")]
    EmptyStructFields,

    /// An enumeration cannot be represented by the grammar without variants.
    #[error("Whole-Ethos enumeration declaration has no variants")]
    EmptyEnumerationVariants,

    /// A tuple payload cannot be represented by the grammar without fields.
    #[error("Whole-Ethos tuple variant payload has no positional fields")]
    EmptyTupleFields,

    /// A type application cannot be represented without arguments.
    #[error("Whole-Ethos type application has no arguments")]
    EmptyTypeArguments,

    /// Two picked-up parameters used the same retained name.
    #[error("Whole-Ethos type parameter {name:?} appears more than once")]
    DuplicateTypeParameterName {
        /// Repeated parameter identity.
        name: VocabularyEncodedId,
    },

    /// One retained parameter name was assigned incompatible trait bounds.
    #[error("Whole-Ethos type parameter {name:?} has conflicting trait qualities")]
    ConflictingTypeParameterQuality {
        /// Retained parameter identity.
        name: VocabularyEncodedId,
    },

    /// Type parameters are constrained only by trait qualities.
    #[error("Whole-Ethos type parameter quality {quality:?} must be a trait")]
    TypeParameterQualityMustBeTrait {
        /// Quality supplied in the wrong role.
        quality: VocabularyEncodedId,
    },

    /// Type applications are headed only by shape qualities.
    #[error("Whole-Ethos type application head {quality:?} must be a shape")]
    TypeApplicationHeadMustBeShape {
        /// Quality supplied in the wrong role.
        quality: VocabularyEncodedId,
    },

    /// Archived parameter metadata did not equal the parameters in the type use.
    #[error("Whole-Ethos type parameter metadata does not match picked-up references")]
    TypeParameterPickupMismatch,

    /// Authored declarations in this grammar are always public.
    #[error("Whole-Ethos declaration has visibility not admitted by the grammar")]
    InvalidItemVisibility,

    /// The wrapped newtype field in this grammar is always private.
    #[error("Whole-Ethos wrapped newtype field has visibility not admitted by the grammar")]
    InvalidWrappedFieldVisibility,

    /// A stored name contains the empty chain reserved for table addresses.
    #[error("Whole-Ethos contains an empty encoded-ID chain at {position:?}")]
    EmptyEncodedId {
        /// Positional role.
        position: WholeEthosEncodedIdPosition,
    },

    /// Whole-Ethos vocabulary positions must carry Universal identities.
    #[error("Whole-Ethos uses non-Universal root {root:?} at {position:?}")]
    NonUniversalEncodedId {
        /// Positional role.
        position: WholeEthosEncodedIdPosition,
        /// Unexpected root.
        root: VocabularyRoot,
    },
}

/// Named translator-issued identities required to assemble the grammar table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EthosGrammarIdentities {
    pub interface_document: VocabularyEncodedId,
    pub nexus_document: VocabularyEncodedId,
    pub sema_document: VocabularyEncodedId,
    pub header: VocabularyEncodedId,
    pub imports: VocabularyEncodedId,
    pub import_entry: VocabularyEncodedId,
    pub interface_body: VocabularyEncodedId,
    pub nexus_body: VocabularyEncodedId,
    pub sema_body: VocabularyEncodedId,
    pub newtype_list: VocabularyEncodedId,
    pub struct_list: VocabularyEncodedId,
    pub item_list: VocabularyEncodedId,
    pub trait_list: VocabularyEncodedId,
    pub table_list: VocabularyEncodedId,
    pub newtype_declaration: VocabularyEncodedId,
    pub struct_declaration: VocabularyEncodedId,
    pub item: VocabularyEncodedId,
    pub variant: VocabularyEncodedId,
    pub type_reference: VocabularyEncodedId,
    pub trait_declaration: VocabularyEncodedId,
    pub table: VocabularyEncodedId,
}

/// Validated structural type identities used by [`EthosCodec`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EthosGrammarIds {
    interface_document: EncodedTypeId<VocabularyRoot>,
    nexus_document: EncodedTypeId<VocabularyRoot>,
    sema_document: EncodedTypeId<VocabularyRoot>,
    header: EncodedTypeId<VocabularyRoot>,
    imports: EncodedTypeId<VocabularyRoot>,
    import_entry: EncodedTypeId<VocabularyRoot>,
    interface_body: EncodedTypeId<VocabularyRoot>,
    nexus_body: EncodedTypeId<VocabularyRoot>,
    sema_body: EncodedTypeId<VocabularyRoot>,
    newtype_list: EncodedTypeId<VocabularyRoot>,
    struct_list: EncodedTypeId<VocabularyRoot>,
    item_list: EncodedTypeId<VocabularyRoot>,
    trait_list: EncodedTypeId<VocabularyRoot>,
    table_list: EncodedTypeId<VocabularyRoot>,
    newtype_declaration: EncodedTypeId<VocabularyRoot>,
    struct_declaration: EncodedTypeId<VocabularyRoot>,
    item: EncodedTypeId<VocabularyRoot>,
    variant: EncodedTypeId<VocabularyRoot>,
    type_reference: EncodedTypeId<VocabularyRoot>,
    trait_declaration: EncodedTypeId<VocabularyRoot>,
    table: EncodedTypeId<VocabularyRoot>,
}

// Trait exception — too trivial: validation and field conversion only.
impl EthosGrammarIds {
    /// Validate every grammar identity as Universal and retain its full chain.
    pub fn new(input: EthosGrammarIdentities) -> Result<Self, EthosGrammarError> {
        macro_rules! grammar_id {
            ($field:ident, $position:ident) => {{
                validate_grammar_id(EthosGrammarIdPosition::$position, &input.$field)?;
                EncodedTypeId::new(input.$field)
            }};
        }
        Ok(Self {
            interface_document: grammar_id!(interface_document, InterfaceDocument),
            nexus_document: grammar_id!(nexus_document, NexusDocument),
            sema_document: grammar_id!(sema_document, SemaDocument),
            header: grammar_id!(header, Header),
            imports: grammar_id!(imports, Imports),
            import_entry: grammar_id!(import_entry, ImportEntry),
            interface_body: grammar_id!(interface_body, InterfaceBody),
            nexus_body: grammar_id!(nexus_body, NexusBody),
            sema_body: grammar_id!(sema_body, SemaBody),
            newtype_list: grammar_id!(newtype_list, NewtypeList),
            struct_list: grammar_id!(struct_list, StructList),
            item_list: grammar_id!(item_list, ItemList),
            trait_list: grammar_id!(trait_list, TraitList),
            table_list: grammar_id!(table_list, TableList),
            newtype_declaration: grammar_id!(newtype_declaration, NewtypeDeclaration),
            struct_declaration: grammar_id!(struct_declaration, StructDeclaration),
            item: grammar_id!(item, Item),
            variant: grammar_id!(variant, Variant),
            type_reference: grammar_id!(type_reference, TypeReference),
            trait_declaration: grammar_id!(trait_declaration, TraitDeclaration),
            table: grammar_id!(table, Table),
        })
    }
}

fn validate_grammar_id(
    position: EthosGrammarIdPosition,
    encoded_id: &VocabularyEncodedId,
) -> Result<(), EthosGrammarError> {
    if encoded_id.root_variant() == &VocabularyRoot::Universal {
        Ok(())
    } else {
        Err(EthosGrammarError::NonUniversal {
            position,
            root: *encoded_id.root_variant(),
        })
    }
}

/// Structural grammar identity position rejected during construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EthosGrammarIdPosition {
    InterfaceDocument,
    NexusDocument,
    SemaDocument,
    Header,
    Imports,
    ImportEntry,
    InterfaceBody,
    NexusBody,
    SemaBody,
    NewtypeList,
    StructList,
    ItemList,
    TraitList,
    TableList,
    NewtypeDeclaration,
    StructDeclaration,
    Item,
    Variant,
    TypeReference,
    TraitDeclaration,
    Table,
}

/// Failure before the structuretree can be sealed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum EthosGrammarError {
    /// Grammar identities belong to the Universal vocabulary.
    #[error("Ethos grammar identity {position:?} uses non-Universal root {root:?}")]
    NonUniversal {
        /// Grammar identity role.
        position: EthosGrammarIdPosition,
        /// Unexpected root.
        root: VocabularyRoot,
    },
}

/// Lookup-only identities accepted at reference and transformer positions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WholeEthosBuiltinPriors {
    identities: Vec<VocabularyEncodedId>,
    application_heads: Vec<VocabularyEncodedId>,
    trait_qualities: Vec<VocabularyEncodedId>,
    stream_transformer: Option<VocabularyEncodedId>,
}

// Trait exception — the proper trait cannot be determined: this public lookup
// configuration preserves the naming-authority boundary.
impl WholeEthosBuiltinPriors {
    /// Register exact Universal identities already assigned to Integer and Vector.
    pub fn new(
        integer: VocabularyEncodedId,
        vector: VocabularyEncodedId,
    ) -> Result<Self, WholeEthosBuiltinPriorError> {
        validate_builtin_prior(WholeEthosBuiltinPriorPosition::Integer, &integer)?;
        validate_builtin_prior(WholeEthosBuiltinPriorPosition::Vector, &vector)?;
        Ok(Self {
            identities: vec![integer],
            application_heads: vec![vector],
            trait_qualities: Vec::new(),
            stream_transformer: None,
        })
    }

    /// Builtin Integer identity.
    pub fn integer(&self) -> &VocabularyEncodedId {
        &self.identities[0]
    }

    /// Builtin Vector identity.
    pub fn vector(&self) -> &VocabularyEncodedId {
        &self.application_heads[0]
    }

    /// Admit another lookup-only type identity.
    pub fn with_identity(
        mut self,
        identity: VocabularyEncodedId,
    ) -> Result<Self, WholeEthosBuiltinPriorError> {
        validate_builtin_prior(WholeEthosBuiltinPriorPosition::Identity, &identity)?;
        if !self.identities.contains(&identity) {
            self.identities.push(identity);
        }
        Ok(self)
    }

    /// Admit another adjacent angle application head.
    pub fn with_application_head(
        mut self,
        head: VocabularyEncodedId,
    ) -> Result<Self, WholeEthosBuiltinPriorError> {
        validate_builtin_prior(WholeEthosBuiltinPriorPosition::ApplicationHead, &head)?;
        if !self.application_heads.contains(&head) {
            self.application_heads.push(head);
        }
        Ok(self)
    }

    /// Admit a trait quality whose bare use picks up an item-local parameter.
    pub fn with_trait_quality(
        mut self,
        quality: VocabularyEncodedId,
    ) -> Result<Self, WholeEthosBuiltinPriorError> {
        validate_builtin_prior(WholeEthosBuiltinPriorPosition::TraitQuality, &quality)?;
        if !self.trait_qualities.contains(&quality) {
            self.trait_qualities.push(quality);
        }
        Ok(self)
    }

    /// Admit the strict standalone stream transformer head, such as `Stream`.
    pub fn with_stream_transformer(
        mut self,
        head: VocabularyEncodedId,
    ) -> Result<Self, WholeEthosBuiltinPriorError> {
        validate_builtin_prior(WholeEthosBuiltinPriorPosition::StreamTransformer, &head)?;
        self.stream_transformer = Some(head);
        Ok(self)
    }

    fn accepts_identity(&self, identity: &VocabularyEncodedId) -> bool {
        self.identities.contains(identity)
    }

    fn accepts_application_head(&self, head: &VocabularyEncodedId) -> bool {
        self.application_heads.contains(head)
    }

    fn accepts_trait_quality(&self, quality: &VocabularyEncodedId) -> bool {
        self.trait_qualities.contains(quality)
    }
}

fn validate_builtin_prior(
    position: WholeEthosBuiltinPriorPosition,
    encoded_id: &VocabularyEncodedId,
) -> Result<(), WholeEthosBuiltinPriorError> {
    if encoded_id.root_variant() == &VocabularyRoot::Universal {
        Ok(())
    } else {
        Err(WholeEthosBuiltinPriorError::NonUniversal {
            position,
            root: *encoded_id.root_variant(),
        })
    }
}

/// Builtin-prior role rejected during codec construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WholeEthosBuiltinPriorPosition {
    Integer,
    Vector,
    Identity,
    ApplicationHead,
    TraitQuality,
    StreamTransformer,
}

/// Invalid lookup-only prior configuration.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WholeEthosBuiltinPriorError {
    /// Priors belong to the Universal vocabulary.
    #[error("builtin {position:?} prior uses non-Universal root {root:?}")]
    NonUniversal {
        /// Prior role.
        position: WholeEthosBuiltinPriorPosition,
        /// Unexpected root.
        root: VocabularyRoot,
    },
}

macro_rules! whole_role {
    ($name:ident, $id:expr) => {
        #[derive(
            rkyv::Archive,
            rkyv::Serialize,
            rkyv::Deserialize,
            Clone,
            Copy,
            Debug,
            Eq,
            Hash,
            Ord,
            PartialEq,
            PartialOrd,
        )]
        struct $name;

        impl FieldRole for $name {
            const STABLE_ID: u16 = $id;
        }
    };
}

whole_role!(DocumentRootRole, 2001);
whole_role!(HeaderRole, 2002);
whole_role!(ImportsRole, 2003);
whole_role!(BodyRole, 2004);
whole_role!(DelimitedRootRole, 2010);
whole_role!(DelimitedItemsRole, 2011);
whole_role!(BodyProductRole, 2020);
whole_role!(InputsRole, 2021);
whole_role!(OutputsRole, 2022);
whole_role!(RefusalsRole, 2023);
whole_role!(TypesRole, 2024);
whole_role!(TraitsRole, 2025);
whole_role!(RecordTypesRole, 2026);
whole_role!(TablesRole, 2027);
whole_role!(ImportApplicationRole, 2030);
whole_role!(ImportSourceRole, 2031);
whole_role!(ImportPayloadRole, 2032);
whole_role!(ImportNamesRole, 2033);

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
struct DocumentRootRecord {
    root: Position<DocumentRootRole, VocabularyRoot>,
    header: Position<HeaderRole, VocabularyRoot>,
    imports: Position<ImportsRole, VocabularyRoot>,
    body: Position<BodyRole, VocabularyRoot>,
}

impl DocumentRootRecord {
    fn new(
        ids: &EthosGrammarIds,
        body: &EncodedTypeId<VocabularyRoot>,
    ) -> Result<Self, structural_codec::AuthoringError> {
        let sequence = OrderedSequence::try_new::<HeaderRole>()?
            .then::<ImportsRole>()?
            .then::<BodyRole>()?;
        Ok(Self {
            root: Position::try_new(SharedDescriptor::OrderedSequence(sequence))?,
            header: Position::try_new(delegate(&ids.header))?,
            imports: Position::try_new(delegate(&ids.imports))?,
            body: Position::try_new(delegate(body))?,
        })
    }
}

struct DocumentRootView<'record> {
    record: &'record DocumentRootRecord,
}

impl BorrowedFieldView<VocabularyRoot> for DocumentRootView<'_> {
    fn expose<Visitor: FieldVisitor<VocabularyRoot>>(&self, visitor: &mut Visitor) {
        visitor.field(&self.record.root);
        visitor.field(&self.record.header);
        visitor.field(&self.record.imports);
        visitor.field(&self.record.body);
    }
}

impl StructureRecord<VocabularyRoot> for DocumentRootRecord {
    type View<'record> = DocumentRootView<'record>;

    fn root_role(&self) -> StableRoleId {
        self.root.role()
    }

    fn fields(&self) -> Self::View<'_> {
        DocumentRootView { record: self }
    }
}

macro_rules! body_record {
    (
        $name:ident,
        $view:ident,
        [$($role:ident => $field:ident),+ $(,)?]
    ) => {
        #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
        struct $name {
            root: Position<DelimitedRootRole, VocabularyRoot>,
            product: Position<BodyProductRole, VocabularyRoot>,
            $($field: Position<$role, VocabularyRoot>,)+
        }

        impl $name {
            fn new(
                $($field: &EncodedTypeId<VocabularyRoot>,)+
            ) -> Result<Self, structural_codec::AuthoringError> {
                let product = body_record!(@product $($role),+);
                let product = Position::try_new(SharedDescriptor::OrderedProduct(product))?;
                Ok(Self {
                    root: Position::try_new(SharedDescriptor::Delimited {
                        boundary: BRACE_BOUNDARY,
                        content: product.role(),
                    })?,
                    product,
                    $($field: Position::try_new(delegate($field))?,)+
                })
            }
        }

        struct $view<'record> {
            record: &'record $name,
        }

        impl BorrowedFieldView<VocabularyRoot> for $view<'_> {
            fn expose<Visitor: FieldVisitor<VocabularyRoot>>(&self, visitor: &mut Visitor) {
                visitor.field(&self.record.root);
                visitor.field(&self.record.product);
                $(visitor.field(&self.record.$field);)+
            }
        }

        impl StructureRecord<VocabularyRoot> for $name {
            type View<'record> = $view<'record>;

            fn root_role(&self) -> StableRoleId {
                self.root.role()
            }

            fn fields(&self) -> Self::View<'_> {
                $view { record: self }
            }
        }
    };
    (@product $first:ident $(, $rest:ident)*) => {
        OrderedProduct::try_new::<$first>()?$(.then::<$rest>()?)*
    };
}

body_record!(
    InterfaceBodyRecord,
    InterfaceBodyView,
    [
        InputsRole => inputs,
        OutputsRole => outputs,
        RefusalsRole => refusals,
        TypesRole => types,
    ]
);
body_record!(
    NexusBodyRecord,
    NexusBodyView,
    [TypesRole => types, TraitsRole => traits]
);
body_record!(
    SemaBodyRecord,
    SemaBodyView,
    [RecordTypesRole => record_types, TablesRole => tables]
);

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
struct DelimitedItemsRecord {
    root: Position<DelimitedRootRole, VocabularyRoot>,
    items: Position<DelimitedItemsRole, VocabularyRoot>,
}

impl DelimitedItemsRecord {
    fn new(
        boundary: TriggerIdentifier,
        item: &EncodedTypeId<VocabularyRoot>,
        minimum: u64,
        maximum: Option<u64>,
    ) -> Result<Self, structural_codec::AuthoringError> {
        let items = Position::try_new(SharedDescriptor::Repeated {
            minimum,
            maximum,
            element: Box::new(delegate(item)),
        })?;
        Ok(Self {
            root: Position::try_new(SharedDescriptor::Delimited {
                boundary,
                content: items.role(),
            })?,
            items,
        })
    }
}

struct DelimitedItemsView<'record> {
    record: &'record DelimitedItemsRecord,
}

impl BorrowedFieldView<VocabularyRoot> for DelimitedItemsView<'_> {
    fn expose<Visitor: FieldVisitor<VocabularyRoot>>(&self, visitor: &mut Visitor) {
        visitor.field(&self.record.root);
        visitor.field(&self.record.items);
    }
}

impl StructureRecord<VocabularyRoot> for DelimitedItemsRecord {
    type View<'record> = DelimitedItemsView<'record>;

    fn root_role(&self) -> StableRoleId {
        self.root.role()
    }

    fn fields(&self) -> Self::View<'_> {
        DelimitedItemsView { record: self }
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
struct ImportEntryRecord {
    application: Position<ImportApplicationRole, VocabularyRoot>,
    source: Position<ImportSourceRole, VocabularyRoot>,
    payload: Position<ImportPayloadRole, VocabularyRoot>,
    names: Position<ImportNamesRole, VocabularyRoot>,
}

impl ImportEntryRecord {
    fn new() -> Result<Self, structural_codec::AuthoringError> {
        let source = Position::try_new(SharedDescriptor::Leaf(LeafCodec::Text))?;
        let names = Position::try_new(SharedDescriptor::Repeated {
            minimum: 1,
            maximum: None,
            element: Box::new(SharedDescriptor::Leaf(LeafCodec::Text)),
        })?;
        let payload = Position::try_new(SharedDescriptor::Alternation(vec![
            SharedDescriptor::Leaf(LeafCodec::Text),
            SharedDescriptor::Delimited {
                boundary: BRACE_BOUNDARY,
                content: names.role(),
            },
        ]))?;
        Ok(Self {
            application: Position::try_new(SharedDescriptor::Application {
                operator: APPLICATION_OPERATOR,
                head: source.role(),
                payload: payload.role(),
            })?,
            source,
            payload,
            names,
        })
    }
}

struct ImportEntryView<'record> {
    record: &'record ImportEntryRecord,
}

impl BorrowedFieldView<VocabularyRoot> for ImportEntryView<'_> {
    fn expose<Visitor: FieldVisitor<VocabularyRoot>>(&self, visitor: &mut Visitor) {
        visitor.field(&self.record.application);
        visitor.field(&self.record.source);
        visitor.field(&self.record.payload);
        visitor.field(&self.record.names);
    }
}

impl StructureRecord<VocabularyRoot> for ImportEntryRecord {
    type View<'record> = ImportEntryView<'record>;

    fn root_role(&self) -> StableRoleId {
        self.application.role()
    }

    fn fields(&self) -> Self::View<'_> {
        ImportEntryView { record: self }
    }
}

type WholeEthosRule = RuleCoproduct<
    DocumentRootRecord,
    RuleCoproduct<
        InterfaceBodyRecord,
        RuleCoproduct<
            NexusBodyRecord,
            RuleCoproduct<
                SemaBodyRecord,
                RuleCoproduct<
                    ImportEntryRecord,
                    RuleCoproduct<DelimitedItemsRecord, StructuralRule<VocabularyRoot>>,
                >,
            >,
        >,
    >,
>;

fn document_rule(record: DocumentRootRecord) -> WholeEthosRule {
    RuleCoproduct::Left(record)
}

fn interface_body_rule(record: InterfaceBodyRecord) -> WholeEthosRule {
    RuleCoproduct::Right(RuleCoproduct::Left(record))
}

fn nexus_body_rule(record: NexusBodyRecord) -> WholeEthosRule {
    RuleCoproduct::Right(RuleCoproduct::Right(RuleCoproduct::Left(record)))
}

fn sema_body_rule(record: SemaBodyRecord) -> WholeEthosRule {
    RuleCoproduct::Right(RuleCoproduct::Right(RuleCoproduct::Right(
        RuleCoproduct::Left(record),
    )))
}

fn import_rule(record: ImportEntryRecord) -> WholeEthosRule {
    RuleCoproduct::Right(RuleCoproduct::Right(RuleCoproduct::Right(
        RuleCoproduct::Right(RuleCoproduct::Left(record)),
    )))
}

fn list_rule(record: DelimitedItemsRecord) -> WholeEthosRule {
    RuleCoproduct::Right(RuleCoproduct::Right(RuleCoproduct::Right(
        RuleCoproduct::Right(RuleCoproduct::Right(RuleCoproduct::Left(record))),
    )))
}

fn structural_rule(record: StructuralRule<VocabularyRoot>) -> WholeEthosRule {
    RuleCoproduct::Right(RuleCoproduct::Right(RuleCoproduct::Right(
        RuleCoproduct::Right(RuleCoproduct::Right(RuleCoproduct::Right(record))),
    )))
}

fn delegate(target: &EncodedTypeId<VocabularyRoot>) -> SharedDescriptor<VocabularyRoot> {
    SharedDescriptor::Delegate {
        target: target.clone(),
        payload: None,
    }
}

struct ConstructorRule {
    local: u16,
    rule: WholeEthosRule,
}

impl ConstructorRule {
    fn new(local: u16, rule: WholeEthosRule) -> Self {
        Self { local, rule }
    }
}

/// One file kind's root seating in the shared addressed table.
trait EthosFileRoot {
    const KIND: WholeEthosFileKind;

    fn document_id(ids: &EthosGrammarIds) -> &EncodedTypeId<VocabularyRoot>;
    fn body_id(ids: &EthosGrammarIds) -> &EncodedTypeId<VocabularyRoot>;
    fn body_rule(ids: &EthosGrammarIds)
    -> Result<WholeEthosRule, structural_codec::AuthoringError>;
    fn reify_body(
        codec: &EthosCodec,
        body: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosBody, EthosDecodeError>;
}

struct InterfaceFileRoot;
struct NexusFileRoot;
struct SemaFileRoot;

impl EthosFileRoot for InterfaceFileRoot {
    const KIND: WholeEthosFileKind = WholeEthosFileKind::Interface;

    fn document_id(ids: &EthosGrammarIds) -> &EncodedTypeId<VocabularyRoot> {
        &ids.interface_document
    }

    fn body_id(ids: &EthosGrammarIds) -> &EncodedTypeId<VocabularyRoot> {
        &ids.interface_body
    }

    fn body_rule(
        ids: &EthosGrammarIds,
    ) -> Result<WholeEthosRule, structural_codec::AuthoringError> {
        Ok(interface_body_rule(InterfaceBodyRecord::new(
            &ids.newtype_list,
            &ids.newtype_list,
            &ids.struct_list,
            &ids.item_list,
        )?))
    }

    fn reify_body(
        codec: &EthosCodec,
        body: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosBody, EthosDecodeError> {
        codec
            .reify_interface_body(body)
            .map(WholeEthosBody::Interface)
    }
}

impl EthosFileRoot for NexusFileRoot {
    const KIND: WholeEthosFileKind = WholeEthosFileKind::Nexus;

    fn document_id(ids: &EthosGrammarIds) -> &EncodedTypeId<VocabularyRoot> {
        &ids.nexus_document
    }

    fn body_id(ids: &EthosGrammarIds) -> &EncodedTypeId<VocabularyRoot> {
        &ids.nexus_body
    }

    fn body_rule(
        ids: &EthosGrammarIds,
    ) -> Result<WholeEthosRule, structural_codec::AuthoringError> {
        Ok(nexus_body_rule(NexusBodyRecord::new(
            &ids.item_list,
            &ids.trait_list,
        )?))
    }

    fn reify_body(
        codec: &EthosCodec,
        body: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosBody, EthosDecodeError> {
        codec.reify_nexus_body(body).map(WholeEthosBody::Nexus)
    }
}

impl EthosFileRoot for SemaFileRoot {
    const KIND: WholeEthosFileKind = WholeEthosFileKind::Sema;

    fn document_id(ids: &EthosGrammarIds) -> &EncodedTypeId<VocabularyRoot> {
        &ids.sema_document
    }

    fn body_id(ids: &EthosGrammarIds) -> &EncodedTypeId<VocabularyRoot> {
        &ids.sema_body
    }

    fn body_rule(
        ids: &EthosGrammarIds,
    ) -> Result<WholeEthosRule, structural_codec::AuthoringError> {
        Ok(sema_body_rule(SemaBodyRecord::new(
            &ids.item_list,
            &ids.table_list,
        )?))
    }

    fn reify_body(
        codec: &EthosCodec,
        body: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosBody, EthosDecodeError> {
        codec.reify_sema_body(body).map(WholeEthosBody::Sema)
    }
}

#[derive(Clone, Debug)]
struct FileKindRegistration {
    kind: WholeEthosFileKind,
    document: EncodedTypeId<VocabularyRoot>,
    reify_body: fn(
        &EthosCodec,
        &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosBody, EthosDecodeError>,
}

struct FileKindSeating {
    registration: FileKindRegistration,
    document_entry: StructuralEntry<VocabularyRoot, WholeEthosRule>,
    body_entry: StructuralEntry<VocabularyRoot, WholeEthosRule>,
}

impl FileKindSeating {
    fn new<Root: EthosFileRoot>(
        ids: &EthosGrammarIds,
    ) -> Result<Self, structural_codec::AuthoringError> {
        let document = Root::document_id(ids).clone();
        let body = Root::body_id(ids).clone();
        Ok(Self {
            registration: FileKindRegistration {
                kind: Root::KIND,
                document: document.clone(),
                reify_body: Root::reify_body,
            },
            document_entry: typed_entry(
                document,
                document_rule(DocumentRootRecord::new(ids, &body)?),
            ),
            body_entry: typed_entry(body, Root::body_rule(ids)?),
        })
    }
}

/// One shared table-driven codec for every supported file kind.
#[derive(Clone, Debug)]
pub struct EthosCodec {
    ids: EthosGrammarIds,
    priors: WholeEthosBuiltinPriors,
    table: AddressedStructuralTable<VocabularyRoot, WholeEthosRule>,
    file_kinds: Vec<FileKindRegistration>,
}

/// Shared document codec behavior.
pub trait EthosDocumentCodec {
    /// Decode header first, select the addressed document root, then reify.
    fn decode<Bindings: DecodeNameBindings<VocabularyRoot> + ?Sized>(
        &self,
        source: &str,
        bindings: &Bindings,
    ) -> Result<DecodedEthos, EthosDecodeError>;

    /// Reflect encoded meaning plus source-only imports and emit canonical text
    /// through the shared structural writer.
    fn encode<Resolver: structural_codec::EncodedNameResolver<VocabularyRoot> + ?Sized>(
        &self,
        decoded: &DecodedEthos,
        resolver: &Resolver,
    ) -> Result<String, EthosEncodeError>;
}

// Trait exception — constructor ergonomics: behavior lives under
// EthosDocumentCodec; this inherent surface only seals the table and delegates.
impl EthosCodec {
    /// Seal the complete grammar table.
    pub fn build(
        ids: EthosGrammarIds,
        priors: WholeEthosBuiltinPriors,
    ) -> Result<Self, EthosCodecBuildError> {
        let profile = SealedTokenProfile::standard();
        let mut entries = Vec::new();
        let file_kind_seatings = vec![
            FileKindSeating::new::<InterfaceFileRoot>(&ids)?,
            FileKindSeating::new::<NexusFileRoot>(&ids)?,
            FileKindSeating::new::<SemaFileRoot>(&ids)?,
        ];
        let file_kinds = file_kind_seatings
            .iter()
            .map(|seating| seating.registration.clone())
            .collect();
        for seating in file_kind_seatings {
            entries.push(seating.document_entry);
            entries.push(seating.body_entry);
        }

        let header_rule = structural_rule(StructuralRule::Application(ApplicationRule::new(
            APPLICATION_OPERATOR,
            SharedDescriptor::Leaf(LeafCodec::Text),
            SharedDescriptor::Leaf(LeafCodec::Integer),
        )?));
        entries.push(typed_entry(ids.header.clone(), header_rule));
        entries.push(typed_entry(
            ids.imports.clone(),
            list_rule(DelimitedItemsRecord::new(
                SQUARE_BOUNDARY,
                &ids.import_entry,
                0,
                None,
            )?),
        ));
        entries.push(typed_entry(
            ids.import_entry.clone(),
            import_rule(ImportEntryRecord::new()?),
        ));

        entries.push(delimited_list_entry(
            &ids.newtype_list,
            &ids.newtype_declaration,
        )?);
        entries.push(delimited_list_entry(
            &ids.struct_list,
            &ids.struct_declaration,
        )?);
        entries.push(delimited_list_entry(&ids.item_list, &ids.item)?);
        entries.push(delimited_list_entry(
            &ids.trait_list,
            &ids.trait_declaration,
        )?);
        entries.push(delimited_list_entry(&ids.table_list, &ids.table)?);

        let newtype = || -> Result<WholeEthosRule, structural_codec::AuthoringError> {
            Ok(structural_rule(StructuralRule::Application(
                ApplicationRule::new(
                    APPLICATION_OPERATOR,
                    SharedDescriptor::Declaration(AtomDescriptor::with_case(AtomCase::PascalCase)),
                    delegate(&ids.type_reference),
                )?,
            )))
        };
        let product = || -> Result<WholeEthosRule, structural_codec::AuthoringError> {
            Ok(structural_rule(StructuralRule::ApplicationDelimited(
                ApplicationDelimitedRule::new(
                    APPLICATION_OPERATOR,
                    BRACE_BOUNDARY,
                    SharedDescriptor::Declaration(AtomDescriptor::with_case(AtomCase::PascalCase)),
                    delegate(&ids.type_reference),
                    1,
                    None,
                )?,
            )))
        };
        entries.push(typed_entry(ids.newtype_declaration.clone(), newtype()?));
        entries.push(typed_entry(ids.struct_declaration.clone(), product()?));

        let enumeration = structural_rule(StructuralRule::ApplicationDelimited(
            ApplicationDelimitedRule::new(
                APPLICATION_OPERATOR,
                SQUARE_BOUNDARY,
                SharedDescriptor::Declaration(AtomDescriptor::with_case(AtomCase::PascalCase)),
                delegate(&ids.variant),
                1,
                None,
            )?,
        ));
        let mut item_rules = vec![
            ConstructorRule::new(0, newtype()?),
            ConstructorRule::new(1, enumeration),
            ConstructorRule::new(2, product()?),
        ];
        if priors.stream_transformer.is_some() {
            item_rules.push(ConstructorRule::new(
                3,
                structural_rule(StructuralRule::ApplicationDelimited(
                    ApplicationDelimitedRule::new(
                        APPLICATION_OPERATOR,
                        PARENTHESIS_BOUNDARY,
                        SharedDescriptor::InlineApplication {
                            operator: APPLICATION_OPERATOR,
                            head: Box::new(SharedDescriptor::Declaration(
                                AtomDescriptor::with_case(AtomCase::PascalCase),
                            )),
                            payload: Box::new(SharedDescriptor::Literal(
                                priors
                                    .stream_transformer
                                    .clone()
                                    .expect("stream transformer is required by its grammar rule"),
                            )),
                        },
                        delegate(&ids.type_reference),
                        2,
                        Some(2),
                    )?,
                )),
            ));
        }
        entries.push(typed_entry_with_rules(ids.item.clone(), item_rules));

        entries.push(typed_entry_with_rules(
            ids.variant.clone(),
            vec![
                ConstructorRule::new(
                    0,
                    structural_rule(StructuralRule::Unary(UnaryRule::new(
                        SharedDescriptor::Declaration(AtomDescriptor::with_case(
                            AtomCase::PascalCase,
                        )),
                    )?)),
                ),
                ConstructorRule::new(
                    1,
                    structural_rule(StructuralRule::ApplicationDelimited(
                        ApplicationDelimitedRule::new(
                            APPLICATION_OPERATOR,
                            BRACE_BOUNDARY,
                            SharedDescriptor::Declaration(AtomDescriptor::with_case(
                                AtomCase::PascalCase,
                            )),
                            delegate(&ids.type_reference),
                            1,
                            None,
                        )?,
                    )),
                ),
                ConstructorRule::new(
                    2,
                    structural_rule(StructuralRule::Application(ApplicationRule::new(
                        APPLICATION_OPERATOR,
                        SharedDescriptor::Declaration(AtomDescriptor::with_case(
                            AtomCase::PascalCase,
                        )),
                        delegate(&ids.type_reference),
                    )?)),
                ),
            ],
        ));

        entries.push(typed_entry_with_rules(
            ids.type_reference.clone(),
            vec![
                ConstructorRule::new(
                    0,
                    structural_rule(StructuralRule::Unary(UnaryRule::new(
                        SharedDescriptor::Reference(AtomDescriptor::with_case(
                            AtomCase::PascalCase,
                        )),
                    )?)),
                ),
                ConstructorRule::new(
                    1,
                    structural_rule(StructuralRule::AdjacentApplicationDelimited(
                        AdjacentApplicationDelimitedRule::new(
                            ANGLE_BOUNDARY,
                            SharedDescriptor::Reference(AtomDescriptor::with_case(
                                AtomCase::PascalCase,
                            )),
                            delegate(&ids.type_reference),
                            1,
                            None,
                        )?,
                    )),
                ),
                ConstructorRule::new(
                    2,
                    structural_rule(StructuralRule::Application(ApplicationRule::new(
                        APPLICATION_OPERATOR,
                        SharedDescriptor::Declaration(AtomDescriptor::with_case(
                            AtomCase::PascalCase,
                        )),
                        SharedDescriptor::Reference(AtomDescriptor::with_case(
                            AtomCase::PascalCase,
                        )),
                    )?)),
                ),
            ],
        ));

        entries.push(typed_entry(
            ids.trait_declaration.clone(),
            structural_rule(StructuralRule::Unary(UnaryRule::new(
                SharedDescriptor::Declaration(AtomDescriptor::with_case(AtomCase::PascalCase)),
            )?)),
        ));
        entries.push(typed_entry(
            ids.table.clone(),
            structural_rule(StructuralRule::ApplicationDelimited(
                ApplicationDelimitedRule::new(
                    APPLICATION_OPERATOR,
                    BRACE_BOUNDARY,
                    SharedDescriptor::Declaration(AtomDescriptor::with_case(AtomCase::CamelCase)),
                    delegate(&ids.type_reference),
                    2,
                    Some(2),
                )?,
            )),
        ));

        let table = AddressedStructuralTable::seal(
            TableIdentityPayload::new(
                TargetLayoutIdentity::derive(b"core-ethos composite document layout v1"),
                profile.identity(),
                StructuralVocabularyIdentity::language(
                    b"core-ethos interface nexus sema typed vocabulary v1",
                ),
                ethos_discovery(),
                ethos_rendering(),
                entries,
            ),
            &profile,
        )
        .map_err(|error| EthosCodecBuildError::Table(Box::new(error)))?;
        Ok(Self {
            ids,
            priors,
            table,
            file_kinds,
        })
    }

    /// Inherent compatibility wrapper for [`EthosDocumentCodec::decode`].
    pub fn decode<Bindings: DecodeNameBindings<VocabularyRoot> + ?Sized>(
        &self,
        source: &str,
        bindings: &Bindings,
    ) -> Result<DecodedEthos, EthosDecodeError> {
        EthosDocumentCodec::decode(self, source, bindings)
    }

    /// Inherent compatibility wrapper for [`EthosDocumentCodec::encode`].
    pub fn encode<Resolver: structural_codec::EncodedNameResolver<VocabularyRoot> + ?Sized>(
        &self,
        decoded: &DecodedEthos,
        resolver: &Resolver,
    ) -> Result<String, EthosEncodeError> {
        EthosDocumentCodec::encode(self, decoded, resolver)
    }

    fn reify_document(
        &self,
        registration: &FileKindRegistration,
        root: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthos, EthosDecodeError> {
        let header_value = delegated::<HeaderRole>(root, "document header")?;
        let header = self.reify_header(header_value)?;
        if header.kind != registration.kind {
            return Err(EthosDecodeError::HeaderSelectionMismatch {
                selected: registration.kind,
                decoded: header.kind,
            });
        }
        let body_value = delegated::<BodyRole>(root, "document body")?;
        let body = (registration.reify_body)(self, body_value)?;
        WholeEthos::new(header, body).map_err(EthosDecodeError::InvalidDocument)
    }

    fn reify_header(
        &self,
        header: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosHeader, EthosDecodeError> {
        let spelling = scalar_text::<ApplicationHead>(header, "header kind")?;
        let kind = WholeEthosFileKind::from_spelling(spelling).ok_or_else(|| {
            EthosDecodeError::UnknownFileKind {
                found: spelling.to_owned(),
            }
        })?;
        let found = scalar_integer::<ApplicationPayload>(header, "header version")?;
        let version = u64::try_from(found).map_err(|_| EthosDecodeError::UnsupportedVersion {
            kind,
            found,
            supported: SUPPORTED_VERSION,
        })?;
        if version != SUPPORTED_VERSION {
            return Err(EthosDecodeError::UnsupportedVersion {
                kind,
                found,
                supported: SUPPORTED_VERSION,
            });
        }
        WholeEthosHeader::new(kind, version).map_err(EthosDecodeError::InvalidDocument)
    }

    fn reify_imports(
        &self,
        imports: &StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosImports, EthosDecodeError> {
        let entries = repeated_delegates(imports, "imports")?
            .into_iter()
            .map(|entry| self.reify_import(entry))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(WholeEthosImports::new(entries))
    }

    fn reify_import(
        &self,
        import: &StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosImport, EthosDecodeError> {
        let source = scalar_text::<ImportSourceRole>(import, "import source")?.to_owned();
        let names = match import.field::<ImportPayloadRole>() {
            Some(FieldValue::Scalar(ScalarValue::Text(name))) => vec![name.clone()],
            Some(FieldValue::Delimited(_)) => {
                let Some(FieldValue::Repeated(names)) = import.field::<ImportNamesRole>() else {
                    return Err(EthosDecodeError::Shape("grouped import names"));
                };
                names
                    .iter()
                    .map(|name| match name {
                        FieldValue::Scalar(ScalarValue::Text(name)) => Ok(name.clone()),
                        _ => Err(EthosDecodeError::Shape("import name")),
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            _ => return Err(EthosDecodeError::Shape("import payload")),
        };
        if names.is_empty() {
            return Err(EthosDecodeError::Shape("non-empty import names"));
        }
        Ok(WholeEthosImport::new(source, names))
    }

    fn reify_interface_body(
        &self,
        body: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosInterfaceBody, EthosDecodeError> {
        Ok(WholeEthosInterfaceBody::new(
            self.reify_newtype_list(delegated::<InputsRole>(body, "interface inputs")?)?,
            self.reify_newtype_list(delegated::<OutputsRole>(body, "interface outputs")?)?,
            self.reify_struct_list(delegated::<RefusalsRole>(body, "interface refusals")?)?,
            self.reify_item_list(delegated::<TypesRole>(body, "interface types")?)?,
        ))
    }

    fn reify_nexus_body(
        &self,
        body: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosNexusBody, EthosDecodeError> {
        Ok(WholeEthosNexusBody::new(
            self.reify_item_list(delegated::<TypesRole>(body, "nexus types")?)?,
            self.reify_trait_list(delegated::<TraitsRole>(body, "nexus traits")?)?,
        ))
    }

    fn reify_sema_body(
        &self,
        body: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosSemaBody, EthosDecodeError> {
        Ok(WholeEthosSemaBody::new(
            self.reify_item_list(delegated::<RecordTypesRole>(body, "sema record types")?)?,
            self.reify_table_list(delegated::<TablesRole>(body, "sema tables")?)?,
        ))
    }

    fn reify_newtype_list(
        &self,
        list: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<Vec<WholeEthosNewtype>, EthosDecodeError> {
        repeated_delegates(list, "newtype declarations")?
            .iter()
            .map(|value| self.reify_newtype(value))
            .collect()
    }

    fn reify_struct_list(
        &self,
        list: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<Vec<WholeEthosStruct>, EthosDecodeError> {
        repeated_delegates(list, "struct declarations")?
            .iter()
            .map(|value| self.reify_struct(value))
            .collect()
    }

    fn reify_item_list(
        &self,
        list: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<Vec<WholeEthosItem>, EthosDecodeError> {
        repeated_delegates(list, "type declarations")?
            .iter()
            .map(|value| self.reify_item(value))
            .collect()
    }

    fn reify_trait_list(
        &self,
        list: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<Vec<WholeEthosTrait>, EthosDecodeError> {
        repeated_delegates(list, "trait declarations")?
            .iter()
            .map(|value| self.reify_trait(value))
            .collect()
    }

    fn reify_table_list(
        &self,
        list: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<Vec<WholeEthosTable>, EthosDecodeError> {
        repeated_delegates(list, "table declarations")?
            .iter()
            .map(|value| self.reify_table(value))
            .collect()
    }

    fn reify_item(
        &self,
        item: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosItem, EthosDecodeError> {
        let constructor = item.constructor();
        if constructor == &EncodedConstructorId::under(&self.ids.item, 0) {
            return self.reify_newtype(item).map(WholeEthosItem::Newtype);
        }
        if constructor == &EncodedConstructorId::under(&self.ids.item, 1) {
            return self
                .reify_enumeration(item)
                .map(WholeEthosItem::Enumeration);
        }
        if constructor == &EncodedConstructorId::under(&self.ids.item, 2) {
            return self.reify_struct(item).map(WholeEthosItem::Struct);
        }
        if self.priors.stream_transformer.is_some()
            && constructor == &EncodedConstructorId::under(&self.ids.item, 3)
        {
            let stream = inline_application_declaration_head::<ApplicationDelimitedHead>(
                item,
                "stream initiation name",
            )?;
            let fields =
                self.reify_references(repeated_delegates(item, "stream initiation fields")?)?;
            let [query, event] = fields.as_slice() else {
                return Err(EthosDecodeError::Shape("two stream initiation fields"));
            };
            return Ok(WholeEthosItem::StreamInitiation(StreamInitiation {
                stream,
                query: query.clone(),
                event: event.clone(),
            }));
        }
        Err(EthosDecodeError::Shape("type item constructor"))
    }

    fn reify_newtype(
        &self,
        value: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosNewtype, EthosDecodeError> {
        let owner_matches = value.constructor()
            == &EncodedConstructorId::under(&self.ids.newtype_declaration, 0)
            || value.constructor() == &EncodedConstructorId::under(&self.ids.item, 0);
        if !owner_matches {
            return Err(EthosDecodeError::Shape("newtype constructor"));
        }
        Ok(WholeEthosNewtype::new(
            declaration_id::<ApplicationHead>(value, "newtype name")?,
            WholeEthosVisibility::Public,
            WholeEthosAttributes::empty(),
            WholeEthosWrappedField::new(
                WholeEthosVisibility::Private,
                self.reify_reference(delegated::<ApplicationPayload>(value, "newtype reference")?)?,
            ),
        ))
    }

    fn reify_struct(
        &self,
        value: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosStruct, EthosDecodeError> {
        let owner_matches = value.constructor()
            == &EncodedConstructorId::under(&self.ids.struct_declaration, 0)
            || value.constructor() == &EncodedConstructorId::under(&self.ids.item, 2);
        if !owner_matches {
            return Err(EthosDecodeError::Shape("struct constructor"));
        }
        WholeEthosStruct::new(
            declaration_id::<ApplicationDelimitedHead>(value, "struct name")?,
            self.reify_references(repeated_delegates(value, "struct fields")?)?,
        )
        .map_err(|_| EthosDecodeError::Shape("non-empty struct fields"))
    }

    fn reify_enumeration(
        &self,
        value: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosEnumeration, EthosDecodeError> {
        let mut variants = Vec::new();
        for variant in repeated_delegates(value, "enumeration variants")? {
            variants.push(self.reify_variant(variant)?);
        }
        WholeEthosEnumeration::new(
            declaration_id::<ApplicationDelimitedHead>(value, "enumeration name")?,
            WholeEthosVisibility::Public,
            WholeEthosAttributes::empty(),
            variants,
        )
        .map_err(|_| EthosDecodeError::Shape("non-empty enumeration variants"))
    }

    fn reify_variant(
        &self,
        variant: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosVariant, EthosDecodeError> {
        if variant.constructor() == &EncodedConstructorId::under(&self.ids.variant, 0) {
            return Ok(WholeEthosVariant::new(
                declaration_id::<UnaryRoot>(variant, "unit variant name")?,
                WholeEthosAttributes::empty(),
                WholeEthosVariantPayload::Unit,
            ));
        }
        if variant.constructor() == &EncodedConstructorId::under(&self.ids.variant, 1) {
            let fields =
                self.reify_references(repeated_delegates(variant, "tuple variant fields")?)?;
            return Ok(WholeEthosVariant::new(
                declaration_id::<ApplicationDelimitedHead>(variant, "tuple variant name")?,
                WholeEthosAttributes::empty(),
                WholeEthosVariantPayload::Tuple(
                    WholeEthosTupleFields::new(fields)
                        .map_err(|_| EthosDecodeError::Shape("non-empty tuple variant"))?,
                ),
            ));
        }
        if variant.constructor() == &EncodedConstructorId::under(&self.ids.variant, 2) {
            return Ok(WholeEthosVariant::new(
                declaration_id::<ApplicationHead>(variant, "payload variant name")?,
                WholeEthosAttributes::empty(),
                WholeEthosVariantPayload::Tuple(
                    WholeEthosTupleFields::new(vec![self.reify_reference(delegated::<
                        ApplicationPayload,
                    >(
                        variant,
                        "payload variant field",
                    )?)?])
                    .map_err(|_| EthosDecodeError::Shape("payload variant field"))?,
                ),
            ));
        }
        Err(EthosDecodeError::Shape("variant constructor"))
    }

    fn reify_trait(
        &self,
        value: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosTrait, EthosDecodeError> {
        Ok(WholeEthosTrait::new(declaration_id::<UnaryRoot>(
            value,
            "trait name",
        )?))
    }

    fn reify_table(
        &self,
        value: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosTable, EthosDecodeError> {
        let mut references = self.reify_references(repeated_delegates(value, "table types")?)?;
        if references.len() != 2 {
            return Err(EthosDecodeError::Shape("table record and key"));
        }
        let key = references
            .pop()
            .ok_or(EthosDecodeError::Shape("table key"))?;
        let record = references
            .pop()
            .ok_or(EthosDecodeError::Shape("table record"))?;
        Ok(WholeEthosTable::new(
            declaration_id::<ApplicationDelimitedHead>(value, "table name")?,
            record,
            key,
        ))
    }

    fn reify_references(
        &self,
        values: Vec<&structural_codec::StructuralValue<VocabularyRoot>>,
    ) -> Result<Vec<WholeEthosTypeReference>, EthosDecodeError> {
        values
            .iter()
            .map(|value| self.reify_reference(value))
            .collect()
    }

    fn reify_reference(
        &self,
        reference: &structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosTypeReference, EthosDecodeError> {
        if reference.constructor() == &EncodedConstructorId::under(&self.ids.type_reference, 0) {
            let identity = reference_id::<UnaryRoot>(reference, "identity reference")?;
            if self.priors.accepts_trait_quality(&identity) {
                return Ok(WholeEthosTypeReference::Parameter(
                    WholeEthosTypeParameter::new(
                        identity.clone(),
                        WholeEthosQuality::Trait(identity),
                    ),
                ));
            }
            if !self.priors.accepts_identity(&identity) {
                return Err(EthosDecodeError::UnregisteredReferencePrior {
                    position: WholeEthosReferencePriorPosition::Identity,
                    found: identity,
                });
            }
            return Ok(WholeEthosTypeReference::Identity(identity));
        }
        if reference.constructor() == &EncodedConstructorId::under(&self.ids.type_reference, 1) {
            let head =
                reference_id::<AdjacentApplicationDelimitedHead>(reference, "application head")?;
            if !self.priors.accepts_application_head(&head) {
                return Err(EthosDecodeError::UnregisteredReferencePrior {
                    position: WholeEthosReferencePriorPosition::ApplicationHead,
                    found: head,
                });
            }
            return Ok(WholeEthosTypeReference::Application(
                WholeEthosTypeApplication::new(
                    WholeEthosQuality::Shape(head),
                    self.reify_references(repeated_delegates(reference, "application arguments")?)?,
                )
                .map_err(|_| EthosDecodeError::Shape("non-empty type application arguments"))?,
            ));
        }
        if reference.constructor() == &EncodedConstructorId::under(&self.ids.type_reference, 2) {
            let name = declaration_id::<ApplicationHead>(reference, "parameter name")?;
            let quality = reference_id::<ApplicationPayload>(reference, "parameter quality")?;
            if !self.priors.accepts_trait_quality(&quality) {
                return Err(EthosDecodeError::UnregisteredReferencePrior {
                    position: WholeEthosReferencePriorPosition::TraitQuality,
                    found: quality,
                });
            }
            return Ok(WholeEthosTypeReference::Parameter(
                WholeEthosTypeParameter::new(name, WholeEthosQuality::Trait(quality)),
            ));
        }
        Err(EthosDecodeError::Shape("type reference constructor"))
    }

    fn reflect_document(
        &self,
        ethos: &WholeEthos,
        imports: &WholeEthosImports,
    ) -> Result<
        (
            EncodedTypeId<VocabularyRoot>,
            StructuralValue<VocabularyRoot>,
        ),
        EthosEncodeError,
    > {
        ethos.validate()?;
        let registration = self
            .file_kinds
            .iter()
            .find(|registration| registration.kind == ethos.header.kind)
            .ok_or(EthosEncodeError::UnregisteredFileKind {
                kind: ethos.header.kind,
            })?;
        let header = self.reflect_header(&ethos.header)?;
        let imports = self.reflect_imports(imports)?;
        let body = self.reflect_body(&ethos.body)?;

        let mut record =
            StructuralValue::record(EncodedConstructorId::under(&registration.document, 0));
        record.insert::<DocumentRootRole>(FieldValue::OrderedProduct)?;
        record.insert::<HeaderRole>(FieldValue::Delegated(Box::new(header)))?;
        record.insert::<ImportsRole>(FieldValue::Delegated(Box::new(imports)))?;
        record.insert::<BodyRole>(FieldValue::Delegated(Box::new(body)))?;
        Ok((registration.document.clone(), record.finish()))
    }

    fn reflect_header(
        &self,
        header: &WholeEthosHeader,
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        Self::reflect_application(
            &self.ids.header,
            0,
            FieldValue::Scalar(ScalarValue::Text(header.kind.spelling().to_owned())),
            FieldValue::Scalar(ScalarValue::Integer(
                i64::try_from(header.version).expect("the supported Ethos version fits i64"),
            )),
        )
        .map_err(Into::into)
    }

    fn reflect_imports(
        &self,
        imports: &WholeEthosImports,
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        let entries = imports
            .entries
            .iter()
            .map(|entry| self.reflect_import(entry))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|entry| FieldValue::Delegated(Box::new(entry)))
            .collect();
        Self::reflect_delimited_list(&self.ids.imports, entries).map_err(Into::into)
    }

    fn reflect_import(
        &self,
        import: &WholeEthosImport,
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        if import.names.is_empty() {
            return Err(EthosEncodeError::EmptyImportNames);
        }
        let source = FieldValue::Scalar(ScalarValue::Text(import.source.clone()));
        let (payload, names) = if let [name] = import.names.as_slice() {
            (FieldValue::Scalar(ScalarValue::Text(name.clone())), None)
        } else {
            let names = FieldValue::Repeated(
                import
                    .names
                    .iter()
                    .cloned()
                    .map(ScalarValue::Text)
                    .map(FieldValue::Scalar)
                    .collect(),
            );
            (FieldValue::Delimited(Box::new(names.clone())), Some(names))
        };
        let mut record =
            StructuralValue::record(EncodedConstructorId::under(&self.ids.import_entry, 0));
        record.insert::<ImportApplicationRole>(FieldValue::Application {
            head: Box::new(source.clone()),
            payload: Box::new(payload.clone()),
        })?;
        record.insert::<ImportSourceRole>(source)?;
        record.insert::<ImportPayloadRole>(payload)?;
        if let Some(names) = names {
            record.insert::<ImportNamesRole>(names)?;
        }
        Ok(record.finish())
    }

    fn reflect_body(
        &self,
        body: &WholeEthosBody,
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        match body {
            WholeEthosBody::Interface(body) => {
                let inputs = self.reflect_newtype_list(&body.inputs)?;
                let outputs = self.reflect_newtype_list(&body.outputs)?;
                let refusals = self.reflect_struct_list(&body.refusals)?;
                let types = self.reflect_item_list(&body.types)?;
                let mut record = StructuralValue::record(EncodedConstructorId::under(
                    &self.ids.interface_body,
                    0,
                ));
                record.insert::<DelimitedRootRole>(FieldValue::Delimited(Box::new(
                    FieldValue::OrderedProduct,
                )))?;
                record.insert::<BodyProductRole>(FieldValue::OrderedProduct)?;
                record.insert::<InputsRole>(FieldValue::Delegated(Box::new(inputs)))?;
                record.insert::<OutputsRole>(FieldValue::Delegated(Box::new(outputs)))?;
                record.insert::<RefusalsRole>(FieldValue::Delegated(Box::new(refusals)))?;
                record.insert::<TypesRole>(FieldValue::Delegated(Box::new(types)))?;
                Ok(record.finish())
            }
            WholeEthosBody::Nexus(body) => {
                let types = self.reflect_item_list(&body.types)?;
                let traits = self.reflect_trait_list(&body.traits)?;
                let mut record =
                    StructuralValue::record(EncodedConstructorId::under(&self.ids.nexus_body, 0));
                record.insert::<DelimitedRootRole>(FieldValue::Delimited(Box::new(
                    FieldValue::OrderedProduct,
                )))?;
                record.insert::<BodyProductRole>(FieldValue::OrderedProduct)?;
                record.insert::<TypesRole>(FieldValue::Delegated(Box::new(types)))?;
                record.insert::<TraitsRole>(FieldValue::Delegated(Box::new(traits)))?;
                Ok(record.finish())
            }
            WholeEthosBody::Sema(body) => {
                let record_types = self.reflect_item_list(&body.record_types)?;
                let tables = self.reflect_table_list(&body.tables)?;
                let mut record =
                    StructuralValue::record(EncodedConstructorId::under(&self.ids.sema_body, 0));
                record.insert::<DelimitedRootRole>(FieldValue::Delimited(Box::new(
                    FieldValue::OrderedProduct,
                )))?;
                record.insert::<BodyProductRole>(FieldValue::OrderedProduct)?;
                record.insert::<RecordTypesRole>(FieldValue::Delegated(Box::new(record_types)))?;
                record.insert::<TablesRole>(FieldValue::Delegated(Box::new(tables)))?;
                Ok(record.finish())
            }
        }
    }

    fn reflect_newtype_list(
        &self,
        values: &[WholeEthosNewtype],
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        let values = values
            .iter()
            .map(|value| self.reflect_newtype(value, &self.ids.newtype_declaration, 0))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|value| FieldValue::Delegated(Box::new(value)))
            .collect();
        Self::reflect_delimited_list(&self.ids.newtype_list, values).map_err(Into::into)
    }

    fn reflect_struct_list(
        &self,
        values: &[WholeEthosStruct],
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        let values = values
            .iter()
            .map(|value| self.reflect_struct(value, &self.ids.struct_declaration, 0))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|value| FieldValue::Delegated(Box::new(value)))
            .collect();
        Self::reflect_delimited_list(&self.ids.struct_list, values).map_err(Into::into)
    }

    fn reflect_item_list(
        &self,
        values: &[WholeEthosItem],
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        let values = values
            .iter()
            .map(|value| self.reflect_item(value))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|value| FieldValue::Delegated(Box::new(value)))
            .collect();
        Self::reflect_delimited_list(&self.ids.item_list, values).map_err(Into::into)
    }

    fn reflect_trait_list(
        &self,
        values: &[WholeEthosTrait],
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        let values = values
            .iter()
            .map(|value| self.reflect_trait(value))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|value| FieldValue::Delegated(Box::new(value)))
            .collect();
        Self::reflect_delimited_list(&self.ids.trait_list, values).map_err(Into::into)
    }

    fn reflect_table_list(
        &self,
        values: &[WholeEthosTable],
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        let values = values
            .iter()
            .map(|value| self.reflect_table(value))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|value| FieldValue::Delegated(Box::new(value)))
            .collect();
        Self::reflect_delimited_list(&self.ids.table_list, values).map_err(Into::into)
    }

    fn reflect_item(
        &self,
        item: &WholeEthosItem,
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        match item {
            WholeEthosItem::Newtype(value) => self.reflect_newtype(value, &self.ids.item, 0),
            WholeEthosItem::Enumeration(value) => self.reflect_enumeration(value),
            WholeEthosItem::Struct(value) => self.reflect_struct(value, &self.ids.item, 2),
            WholeEthosItem::StreamInitiation(value) => self.reflect_stream_initiation(value),
        }
    }

    fn reflect_newtype(
        &self,
        value: &WholeEthosNewtype,
        type_id: &EncodedTypeId<VocabularyRoot>,
        local: u16,
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        Self::reflect_application(
            type_id,
            local,
            FieldValue::Declaration(DeclarationAssignment::new(value.name.clone())),
            FieldValue::Delegated(Box::new(
                self.reflect_reference(&value.wrapped_field.reference)?,
            )),
        )
        .map_err(Into::into)
    }

    fn reflect_struct(
        &self,
        value: &WholeEthosStruct,
        type_id: &EncodedTypeId<VocabularyRoot>,
        local: u16,
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        let fields = value
            .fields
            .iter()
            .map(|field| self.reflect_reference(field))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|field| FieldValue::Delegated(Box::new(field)))
            .collect();
        Self::reflect_application_delimited(
            type_id,
            local,
            FieldValue::Declaration(DeclarationAssignment::new(value.name.clone())),
            fields,
        )
        .map_err(Into::into)
    }

    fn reflect_enumeration(
        &self,
        value: &WholeEthosEnumeration,
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        let variants = value
            .variants
            .iter()
            .map(|variant| self.reflect_variant(variant))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|variant| FieldValue::Delegated(Box::new(variant)))
            .collect();
        Self::reflect_application_delimited(
            &self.ids.item,
            1,
            FieldValue::Declaration(DeclarationAssignment::new(value.name.clone())),
            variants,
        )
        .map_err(Into::into)
    }

    fn reflect_variant(
        &self,
        value: &WholeEthosVariant,
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        let name = FieldValue::Declaration(DeclarationAssignment::new(value.name.clone()));
        match &value.payload {
            WholeEthosVariantPayload::Unit => {
                Self::reflect_unary(&self.ids.variant, 0, name).map_err(Into::into)
            }
            WholeEthosVariantPayload::Tuple(fields) if fields.0.len() == 1 => {
                Self::reflect_application(
                    &self.ids.variant,
                    2,
                    name,
                    FieldValue::Delegated(Box::new(self.reflect_reference(&fields.0[0])?)),
                )
                .map_err(Into::into)
            }
            WholeEthosVariantPayload::Tuple(fields) => {
                let fields = fields
                    .0
                    .iter()
                    .map(|field| self.reflect_reference(field))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .map(|field| FieldValue::Delegated(Box::new(field)))
                    .collect();
                Self::reflect_application_delimited(&self.ids.variant, 1, name, fields)
                    .map_err(Into::into)
            }
        }
    }

    fn reflect_stream_initiation(
        &self,
        value: &WholeEthosStreamInitiation,
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        let transformer = self
            .priors
            .stream_transformer
            .as_ref()
            .ok_or(EthosEncodeError::UnregisteredStreamTransformer)?;
        let fields = [&value.query, &value.event]
            .into_iter()
            .map(|field| self.reflect_reference(field))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|field| FieldValue::Delegated(Box::new(field)))
            .collect();
        Self::reflect_application_delimited(
            &self.ids.item,
            3,
            FieldValue::Application {
                head: Box::new(FieldValue::Declaration(DeclarationAssignment::new(
                    value.stream.clone(),
                ))),
                payload: Box::new(FieldValue::Literal(transformer.clone())),
            },
            fields,
        )
        .map_err(Into::into)
    }

    fn reflect_trait(
        &self,
        value: &WholeEthosTrait,
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        Self::reflect_unary(
            &self.ids.trait_declaration,
            0,
            FieldValue::Declaration(DeclarationAssignment::new(value.name.clone())),
        )
        .map_err(Into::into)
    }

    fn reflect_table(
        &self,
        value: &WholeEthosTable,
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        let fields = vec![
            FieldValue::Delegated(Box::new(self.reflect_reference(&value.record)?)),
            FieldValue::Delegated(Box::new(self.reflect_reference(&value.key)?)),
        ];
        Self::reflect_application_delimited(
            &self.ids.table,
            0,
            FieldValue::Declaration(DeclarationAssignment::new(value.name.clone())),
            fields,
        )
        .map_err(Into::into)
    }

    fn reflect_reference(
        &self,
        value: &WholeEthosTypeReference,
    ) -> Result<StructuralValue<VocabularyRoot>, EthosEncodeError> {
        match value {
            WholeEthosTypeReference::Identity(identity) => Self::reflect_unary(
                &self.ids.type_reference,
                0,
                FieldValue::Reference(ResolvedReference::new(identity.clone())),
            )
            .map_err(Into::into),
            WholeEthosTypeReference::Parameter(parameter) => {
                let WholeEthosQuality::Trait(quality) = parameter.quality() else {
                    return Err(EthosEncodeError::InvalidDocument(
                        WholeEthosArchiveError::TypeParameterQualityMustBeTrait {
                            quality: parameter.quality().identity().clone(),
                        },
                    ));
                };
                if parameter.name() == quality {
                    Self::reflect_unary(
                        &self.ids.type_reference,
                        0,
                        FieldValue::Reference(ResolvedReference::new(parameter.name.clone())),
                    )
                    .map_err(Into::into)
                } else {
                    Self::reflect_application(
                        &self.ids.type_reference,
                        2,
                        FieldValue::Declaration(DeclarationAssignment::new(parameter.name.clone())),
                        FieldValue::Reference(ResolvedReference::new(quality.clone())),
                    )
                    .map_err(Into::into)
                }
            }
            WholeEthosTypeReference::Application(application) => {
                Self::reflect_adjacent_application_delimited(
                    &self.ids.type_reference,
                    1,
                    FieldValue::Reference(ResolvedReference::new(
                        application.head.identity().clone(),
                    )),
                    application
                        .arguments
                        .iter()
                        .map(|argument| self.reflect_reference(argument))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .map(|argument| FieldValue::Delegated(Box::new(argument)))
                        .collect(),
                )
                .map_err(Into::into)
            }
        }
    }

    fn reflect_unary(
        type_id: &EncodedTypeId<VocabularyRoot>,
        local: u16,
        root: FieldValue<VocabularyRoot>,
    ) -> Result<StructuralValue<VocabularyRoot>, structural_codec::AuthoringError> {
        let mut record = StructuralValue::record(EncodedConstructorId::under(type_id, local));
        record.insert::<UnaryRoot>(root)?;
        Ok(record.finish())
    }

    fn reflect_adjacent_application_delimited(
        type_id: &EncodedTypeId<VocabularyRoot>,
        local: u16,
        head: FieldValue<VocabularyRoot>,
        items: Vec<FieldValue<VocabularyRoot>>,
    ) -> Result<StructuralValue<VocabularyRoot>, structural_codec::AuthoringError> {
        let items = FieldValue::Repeated(items);
        let body = FieldValue::Delimited(Box::new(items.clone()));
        let mut record = StructuralValue::record(EncodedConstructorId::under(type_id, local));
        record.insert::<AdjacentApplicationDelimitedRoot>(FieldValue::OrderedProduct)?;
        record.insert::<AdjacentApplicationDelimitedHead>(head)?;
        record.insert::<AdjacentApplicationDelimitedBody>(body)?;
        record.insert::<AdjacentApplicationDelimitedItems>(items)?;
        Ok(record.finish())
    }

    fn reflect_application(
        type_id: &EncodedTypeId<VocabularyRoot>,
        local: u16,
        head: FieldValue<VocabularyRoot>,
        payload: FieldValue<VocabularyRoot>,
    ) -> Result<StructuralValue<VocabularyRoot>, structural_codec::AuthoringError> {
        let mut record = StructuralValue::record(EncodedConstructorId::under(type_id, local));
        record.insert::<ApplicationRoot>(FieldValue::Application {
            head: Box::new(head.clone()),
            payload: Box::new(payload.clone()),
        })?;
        record.insert::<ApplicationHead>(head)?;
        record.insert::<ApplicationPayload>(payload)?;
        Ok(record.finish())
    }

    fn reflect_application_delimited(
        type_id: &EncodedTypeId<VocabularyRoot>,
        local: u16,
        head: FieldValue<VocabularyRoot>,
        items: Vec<FieldValue<VocabularyRoot>>,
    ) -> Result<StructuralValue<VocabularyRoot>, structural_codec::AuthoringError> {
        let items = FieldValue::Repeated(items);
        let body = FieldValue::Delimited(Box::new(items.clone()));
        let mut record = StructuralValue::record(EncodedConstructorId::under(type_id, local));
        record.insert::<ApplicationDelimitedRoot>(FieldValue::Application {
            head: Box::new(head.clone()),
            payload: Box::new(body.clone()),
        })?;
        record.insert::<ApplicationDelimitedHead>(head)?;
        record.insert::<ApplicationDelimitedBody>(body)?;
        record.insert::<ApplicationDelimitedItems>(items)?;
        Ok(record.finish())
    }

    fn reflect_delimited_list(
        type_id: &EncodedTypeId<VocabularyRoot>,
        items: Vec<FieldValue<VocabularyRoot>>,
    ) -> Result<StructuralValue<VocabularyRoot>, structural_codec::AuthoringError> {
        let items = FieldValue::Repeated(items);
        let mut record = StructuralValue::record(EncodedConstructorId::under(type_id, 0));
        record.insert::<DelimitedRootRole>(FieldValue::Delimited(Box::new(items.clone())))?;
        record.insert::<DelimitedItemsRole>(items)?;
        Ok(record.finish())
    }
}

impl EthosDocumentCodec for EthosCodec {
    fn decode<Bindings: DecodeNameBindings<VocabularyRoot> + ?Sized>(
        &self,
        source: &str,
        bindings: &Bindings,
    ) -> Result<DecodedEthos, EthosDecodeError> {
        let tree = DiscoveredBlockTree::discover(
            source,
            self.table.token_profile(),
            self.table.block_discovery(),
        )?;
        let root_blocks = tree.root_blocks();
        if root_blocks.len() != 2 {
            return Err(EthosDecodeError::EnvelopeBoundaryCount {
                found: root_blocks.len(),
            });
        }
        let first_delimited = root_blocks[0].source_bound();
        let header_region = &source[..first_delimited.start()];
        let header_source = header_region.trim_matches(char::is_whitespace);
        if header_source.is_empty() {
            return Err(EthosDecodeError::MissingHeader);
        }

        let evaluator = StructuralEvaluator::new(&self.table)?;
        let header_value = evaluator.decode_text(&self.ids.header, header_source, bindings)?;
        let header = self.reify_header(&header_value)?;
        let registration = self
            .file_kinds
            .iter()
            .find(|registration| registration.kind == header.kind)
            .ok_or(EthosDecodeError::UnregisteredFileKind { kind: header.kind })?;
        let mirror = evaluator.decode_text(&registration.document, source, bindings)?;
        let imports = self.reify_imports(delegated::<ImportsRole>(&mirror, "imports")?)?;
        let ethos = self.reify_document(registration, &mirror)?;

        Ok(DecodedEthos { ethos, imports })
    }

    fn encode<Resolver: structural_codec::EncodedNameResolver<VocabularyRoot> + ?Sized>(
        &self,
        decoded: &DecodedEthos,
        resolver: &Resolver,
    ) -> Result<String, EthosEncodeError> {
        let (selected_root, reflected) = self.reflect_document(&decoded.ethos, &decoded.imports)?;
        Ok(StructuralEvaluator::new(&self.table)?.encode_text(
            &selected_root,
            &reflected,
            resolver,
        )?)
    }
}

/// A decoded encoded document plus its typed, source-only imports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedEthos {
    ethos: WholeEthos,
    imports: WholeEthosImports,
}

// Trait exception — too trivial: read-only projections of validated state.
impl DecodedEthos {
    /// Encoded header and body.
    pub const fn ethos(&self) -> &WholeEthos {
        &self.ethos
    }

    /// Typed source-only imports used when reflecting the document to text.
    pub const fn imports(&self) -> &WholeEthosImports {
        &self.imports
    }

    /// Consume the runtime wrapper and retain encoded data only.
    pub fn into_ethos(self) -> WholeEthos {
        self.ethos
    }
}

fn typed_entry(
    type_id: EncodedTypeId<VocabularyRoot>,
    rule: WholeEthosRule,
) -> StructuralEntry<VocabularyRoot, WholeEthosRule> {
    let constructor = EncodedConstructorId::under(&type_id, 0);
    StructuralEntry::new(
        type_id,
        vec![ConstructorCodec::new(
            constructor,
            vec![AcceptedDecodeForm::new(DecodeFormId::new(0), rule.clone())],
            rule,
        )],
    )
}

fn delimited_list_entry(
    list: &EncodedTypeId<VocabularyRoot>,
    element: &EncodedTypeId<VocabularyRoot>,
) -> Result<StructuralEntry<VocabularyRoot, WholeEthosRule>, structural_codec::AuthoringError> {
    Ok(typed_entry(
        list.clone(),
        list_rule(DelimitedItemsRecord::new(
            SQUARE_BOUNDARY,
            element,
            0,
            None,
        )?),
    ))
}

fn typed_entry_with_rules(
    type_id: EncodedTypeId<VocabularyRoot>,
    rules: Vec<ConstructorRule>,
) -> StructuralEntry<VocabularyRoot, WholeEthosRule> {
    let constructors = rules
        .into_iter()
        .map(|entry| {
            ConstructorCodec::new(
                EncodedConstructorId::under(&type_id, entry.local),
                vec![AcceptedDecodeForm::new(
                    DecodeFormId::new(0),
                    entry.rule.clone(),
                )],
                entry.rule,
            )
        })
        .collect();
    StructuralEntry::new(type_id, constructors)
}

fn ethos_discovery() -> BlockTreeDiscoveryConfiguration {
    let active = TriggerSet::new(vec![
        PARENTHESIS_BOUNDARY,
        SQUARE_BOUNDARY,
        BRACE_BOUNDARY,
        TEXT_CARRIER,
        WHITESPACE_TRIVIA,
        ANGLE_BOUNDARY,
    ]);
    BlockTreeDiscoveryConfiguration::new(
        BoundaryDiscoveryConfiguration::new(
            ROOT_CONTEXT,
            vec![
                BoundaryDiscoveryContext::new(ROOT_CONTEXT, active.clone()),
                BoundaryDiscoveryContext::new(CHILD_CONTEXT, active),
            ],
            vec![
                BoundaryDiscoveryTransition::new(ROOT_CONTEXT, PARENTHESIS_BOUNDARY, CHILD_CONTEXT),
                BoundaryDiscoveryTransition::new(ROOT_CONTEXT, SQUARE_BOUNDARY, CHILD_CONTEXT),
                BoundaryDiscoveryTransition::new(ROOT_CONTEXT, BRACE_BOUNDARY, CHILD_CONTEXT),
                BoundaryDiscoveryTransition::new(ROOT_CONTEXT, ANGLE_BOUNDARY, CHILD_CONTEXT),
                BoundaryDiscoveryTransition::new(
                    CHILD_CONTEXT,
                    PARENTHESIS_BOUNDARY,
                    CHILD_CONTEXT,
                ),
                BoundaryDiscoveryTransition::new(CHILD_CONTEXT, SQUARE_BOUNDARY, CHILD_CONTEXT),
                BoundaryDiscoveryTransition::new(CHILD_CONTEXT, BRACE_BOUNDARY, CHILD_CONTEXT),
                BoundaryDiscoveryTransition::new(CHILD_CONTEXT, ANGLE_BOUNDARY, CHILD_CONTEXT),
            ],
        ),
        vec![],
    )
}

fn ethos_rendering() -> TextualRenderingPolicy {
    TextualRenderingPolicy::new(vec![
        ContextualTextualPolicy::new(ROOT_CONTEXT, Some(WHITESPACE_TRIVIA), Some(TEXT_CARRIER)),
        ContextualTextualPolicy::new(CHILD_CONTEXT, Some(WHITESPACE_TRIVIA), Some(TEXT_CARRIER)),
    ])
}

fn delegated<'value, Role: FieldRole>(
    value: &'value structural_codec::StructuralValue<VocabularyRoot>,
    what: &'static str,
) -> Result<&'value structural_codec::StructuralValue<VocabularyRoot>, EthosDecodeError> {
    match value.field::<Role>() {
        Some(FieldValue::Delegated(inner)) => Ok(inner),
        _ => Err(EthosDecodeError::Shape(what)),
    }
}

fn repeated_delegates<'value>(
    value: &'value structural_codec::StructuralValue<VocabularyRoot>,
    what: &'static str,
) -> Result<Vec<&'value structural_codec::StructuralValue<VocabularyRoot>>, EthosDecodeError> {
    let Some(FieldValue::Repeated(items)) = value
        .field::<DelimitedItemsRole>()
        .or_else(|| value.field::<ApplicationDelimitedItems>())
        .or_else(|| value.field::<AdjacentApplicationDelimitedItems>())
    else {
        return Err(EthosDecodeError::Shape(what));
    };
    items
        .iter()
        .map(|item| match item {
            FieldValue::Delegated(inner) => Ok(inner.as_ref()),
            _ => Err(EthosDecodeError::Shape(what)),
        })
        .collect()
}

fn declaration_id<Role: FieldRole>(
    value: &structural_codec::StructuralValue<VocabularyRoot>,
    what: &'static str,
) -> Result<VocabularyEncodedId, EthosDecodeError> {
    match value.field::<Role>() {
        Some(FieldValue::Declaration(assignment)) => {
            let encoded_id = assignment.encoded_id().clone();
            validate_decoded_id(DecodedEncodedIdPosition::Declaration, &encoded_id)?;
            Ok(encoded_id)
        }
        _ => Err(EthosDecodeError::Shape(what)),
    }
}

fn inline_application_declaration_head<Role: FieldRole>(
    value: &structural_codec::StructuralValue<VocabularyRoot>,
    what: &'static str,
) -> Result<VocabularyEncodedId, EthosDecodeError> {
    match value.field::<Role>() {
        Some(FieldValue::Application { head, .. }) => match head.as_ref() {
            FieldValue::Declaration(assignment) => {
                let encoded_id = assignment.encoded_id().clone();
                validate_decoded_id(DecodedEncodedIdPosition::Declaration, &encoded_id)?;
                Ok(encoded_id)
            }
            _ => Err(EthosDecodeError::Shape(what)),
        },
        _ => Err(EthosDecodeError::Shape(what)),
    }
}

fn reference_id<Role: FieldRole>(
    value: &structural_codec::StructuralValue<VocabularyRoot>,
    what: &'static str,
) -> Result<VocabularyEncodedId, EthosDecodeError> {
    match value.field::<Role>() {
        Some(FieldValue::Reference(reference)) => {
            let encoded_id = reference.encoded_id().clone();
            validate_decoded_id(DecodedEncodedIdPosition::Reference, &encoded_id)?;
            Ok(encoded_id)
        }
        _ => Err(EthosDecodeError::Shape(what)),
    }
}

fn scalar_text<'value, Role: FieldRole>(
    value: &'value structural_codec::StructuralValue<VocabularyRoot>,
    what: &'static str,
) -> Result<&'value str, EthosDecodeError> {
    match value.field::<Role>() {
        Some(FieldValue::Scalar(ScalarValue::Text(text))) => Ok(text),
        _ => Err(EthosDecodeError::Shape(what)),
    }
}

fn scalar_integer<Role: FieldRole>(
    value: &structural_codec::StructuralValue<VocabularyRoot>,
    what: &'static str,
) -> Result<i64, EthosDecodeError> {
    match value.field::<Role>() {
        Some(FieldValue::Scalar(ScalarValue::Integer(integer))) => Ok(*integer),
        _ => Err(EthosDecodeError::Shape(what)),
    }
}

fn validate_decoded_id(
    position: DecodedEncodedIdPosition,
    encoded_id: &VocabularyEncodedId,
) -> Result<(), EthosDecodeError> {
    if encoded_id.root_variant() == &VocabularyRoot::Universal {
        Ok(())
    } else {
        Err(EthosDecodeError::NonUniversalEncodedId {
            position,
            root: *encoded_id.root_variant(),
        })
    }
}

/// Encoded-ID role rejected after structural evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodedEncodedIdPosition {
    /// Authored declaration assignment.
    Declaration,
    /// Lookup-only reference resolution.
    Reference,
}

/// Failure while sealing the Whole-Ethos structuretree.
#[derive(Clone, Debug, thiserror::Error)]
pub enum EthosCodecBuildError {
    /// A typed record could not be authored.
    #[error(transparent)]
    Authoring(#[from] structural_codec::AuthoringError),

    /// The complete structural table refused to seal.
    #[error(transparent)]
    Table(Box<structural_codec::TableError<VocabularyRoot>>),
}

/// Failure while rendering a decoded Ethos document.
#[derive(Clone, Debug, thiserror::Error)]
pub enum EthosEncodeError {
    /// The typed encoded carrier violates a public invariant.
    #[error(transparent)]
    InvalidDocument(#[from] WholeEthosArchiveError),

    /// Reflection could not construct a checked typed-role mirror.
    #[error(transparent)]
    Authoring(#[from] structural_codec::AuthoringError),

    /// The sealed table could not construct its evaluator.
    #[error(transparent)]
    Evaluator(#[from] structural_codec::DecodeError<VocabularyRoot>),

    /// The reflected encoded value could not be rendered under its selected root.
    #[error(transparent)]
    Structural(#[from] structural_codec::EncodeError<VocabularyRoot>),

    /// A supported kind was not seated in the codec registry.
    #[error("Ethos file kind {kind:?} has no registered document root")]
    UnregisteredFileKind {
        /// Missing runtime registration.
        kind: WholeEthosFileKind,
    },

    /// A source-only import must name at least one imported item.
    #[error("Ethos import has no imported names")]
    EmptyImportNames,

    /// The strict stream transformer is absent from this codec's priors.
    #[error("Ethos stream initiation requires a registered Stream transformer")]
    UnregisteredStreamTransformer,
}

/// Failure while decoding or reifying a Whole-Ethos document.
#[derive(Clone, Debug, thiserror::Error)]
pub enum EthosDecodeError {
    /// Reified data violated an encoded carrier invariant.
    #[error(transparent)]
    InvalidDocument(#[from] WholeEthosArchiveError),

    /// Shared structural evaluation refused the source.
    #[error(transparent)]
    Structural(#[from] structural_codec::DecodeError<VocabularyRoot>),

    /// Shared boundary discovery refused the source.
    #[error(transparent)]
    Discovery(#[from] raw_discovery::BlockDiscoveryError),

    /// A document must expose imports and body as its two top-level delimiters.
    #[error("Ethos envelope expected two top-level delimited objects, found {found}")]
    EnvelopeBoundaryCount {
        /// Discovered delimiter count.
        found: usize,
    },

    /// No non-trivia header preceded imports.
    #[error("Ethos document omitted its header")]
    MissingHeader,

    /// Header kind has no registered body root.
    #[error("unknown Ethos file kind `{found}`")]
    UnknownFileKind {
        /// Source spelling.
        found: String,
    },

    /// A supported spelling was not seated in the runtime root registry.
    #[error("Ethos file kind {kind:?} has no registered document root")]
    UnregisteredFileKind {
        /// Missing runtime registration.
        kind: WholeEthosFileKind,
    },

    /// Header version is not accepted for the selected kind.
    #[error("unsupported {kind:?} version {found}; expected {supported}")]
    UnsupportedVersion {
        /// Selected kind.
        kind: WholeEthosFileKind,
        /// Source integer.
        found: i64,
        /// Accepted generation.
        supported: u64,
    },

    /// The selected root decoded a different header, indicating table divergence.
    #[error("selected {selected:?} root decoded {decoded:?} header")]
    HeaderSelectionMismatch {
        /// First-phase selection.
        selected: WholeEthosFileKind,
        /// Complete-root decode.
        decoded: WholeEthosFileKind,
    },

    /// The selected structural mirror did not contain its typed role value.
    #[error("the Ethos structural value did not fit {0}")]
    Shape(&'static str),

    /// Whole-Ethos vocabulary uses Universal identities only.
    #[error("decoded {position:?} uses non-Universal root {root:?}")]
    NonUniversalEncodedId {
        /// Name role.
        position: DecodedEncodedIdPosition,
        /// Unexpected root.
        root: VocabularyRoot,
    },

    /// Reference resolution returned an identity absent from the supplied priors.
    #[error("type reference at {position:?} resolved to unregistered prior {found:?}")]
    UnregisteredReferencePrior {
        /// Reference role.
        position: WholeEthosReferencePriorPosition,
        /// Rejected identity.
        found: VocabularyEncodedId,
    },
}

/// Lookup-only prior role rejected while reifying a reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WholeEthosReferencePriorPosition {
    /// Direct type identity.
    Identity,
    /// Unary application head.
    ApplicationHead,
    /// Named type-parameter trait quality.
    TraitQuality,
}

#[cfg(test)]
#[path = "whole_invariant_tests.rs"]
mod invariant_tests;
