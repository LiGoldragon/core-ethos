//! The first complete Ethos slice over translator-issued encoded-ID chains.
//!
//! This module is deliberately separate from the legacy flat-identifier algebra.
//! Its item vocabulary is deliberately small: attribute-free newtypes,
//! enumerations with unit or positional tuple variants, and named or unary
//! application type references. Every name position carries the complete
//! Universal encoded-ID chain supplied by the naming authority.

use content_identity::{ArchiveError, PortableArchive};
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use slice_raw_discovery::{
    BlockTreeDiscoveryConfiguration, BoundaryDiscoveryConfiguration, BoundaryDiscoveryContext,
    BoundaryDiscoveryContextIdentifier, BoundaryDiscoveryTransition, SealedTokenProfile,
    TriggerIdentifier, TriggerSet,
};
use slice_structural_codec::{
    AcceptedDecodeForm, AddressedStructuralTable, ApplicationDelimitedHead,
    ApplicationDelimitedItems, ApplicationDelimitedRule, ApplicationHead, ApplicationPayload,
    ApplicationRule, AtomCase, AtomDescriptor, BorrowedFieldView, ConstructorCodec,
    ContextualTextualPolicy, DecodeFormId, DecodeNameBindings, EncodedConstructorId, EncodedTypeId,
    FieldRole, FieldValue, FieldVisitor, OrderedProduct, Position, RuleCoproduct, SharedDescriptor,
    StableRoleId, StructuralEntry, StructuralEvaluator, StructuralRule,
    StructuralVocabularyIdentity, StructureRecord, TableIdentityPayload, TargetLayoutIdentity,
    TextualRenderingPolicy, UnaryRoot, UnaryRule,
};

pub use slice_raw_discovery::SourceBound;

const SQUARE_BOUNDARY: TriggerIdentifier = TriggerIdentifier::new(1);
const BRACE_BOUNDARY: TriggerIdentifier = TriggerIdentifier::new(2);
const APPLICATION_OPERATOR: TriggerIdentifier = TriggerIdentifier::new(3);
const WHITESPACE_TRIVIA: TriggerIdentifier = TriggerIdentifier::new(5);
const ROOT_CONTEXT: BoundaryDiscoveryContextIdentifier = BoundaryDiscoveryContextIdentifier::new(1);
const CHILD_CONTEXT: BoundaryDiscoveryContextIdentifier =
    BoundaryDiscoveryContextIdentifier::new(2);

/// Ordered Ethos content admitted by the first vertical slice.
///
/// The carrier contains no complete NameTree pin and is not a Capsule.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthos(Vec<WholeEthosItem>);

impl WholeEthos {
    /// Construct the content in authored item order.
    pub fn new(items: Vec<WholeEthosItem>) -> Self {
        Self(items)
    }

    /// Items in authored order.
    pub fn items(&self) -> &[WholeEthosItem] {
        &self.0
    }

    /// Consume the carrier without changing item order.
    pub fn into_items(self) -> Vec<WholeEthosItem> {
        self.0
    }

    /// Serialize the complete positional carrier.
    pub fn to_archive_bytes(&self) -> Result<Vec<u8>, WholeEthosArchiveError> {
        Ok(<Self as PortableArchive>::to_archive_bytes(self)?
            .as_ref()
            .to_vec())
    }

    /// Restore a carrier after archive and Universal-chain validation.
    pub fn from_archive_bytes(bytes: &[u8]) -> Result<Self, WholeEthosArchiveError> {
        let restored = <Self as PortableArchive>::from_archive_bytes(bytes)?;
        restored.validate()?;
        Ok(restored)
    }

    fn validate(&self) -> Result<(), WholeEthosArchiveError> {
        for (item_index, item) in self.0.iter().enumerate() {
            match item {
                WholeEthosItem::Newtype(newtype) => {
                    validate_universal(
                        item_index,
                        WholeEthosEncodedIdPosition::ItemName,
                        newtype.name(),
                    )?;
                    validate_type_reference(
                        item_index,
                        WholeEthosEncodedIdPosition::NewtypeField,
                        newtype.wrapped_field().reference(),
                    )?;
                }
                WholeEthosItem::Enumeration(enumeration) => {
                    validate_universal(
                        item_index,
                        WholeEthosEncodedIdPosition::ItemName,
                        enumeration.name(),
                    )?;
                    for variant in enumeration.variants() {
                        validate_universal(
                            item_index,
                            WholeEthosEncodedIdPosition::VariantName,
                            variant.name(),
                        )?;
                        if let WholeEthosVariantPayload::Tuple(fields) = variant.payload() {
                            for field in fields.fields() {
                                validate_type_reference(
                                    item_index,
                                    WholeEthosEncodedIdPosition::VariantField,
                                    field,
                                )?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn validate_universal(
    item_index: usize,
    position: WholeEthosEncodedIdPosition,
    encoded_id: &VocabularyEncodedId,
) -> Result<(), WholeEthosArchiveError> {
    if encoded_id.root_variant() != &VocabularyRoot::Universal {
        return Err(WholeEthosArchiveError::NonUniversalEncodedId {
            item_index,
            position,
            root: *encoded_id.root_variant(),
        });
    }
    if encoded_id.chain().is_empty() {
        return Err(WholeEthosArchiveError::EmptyEncodedId {
            item_index,
            position,
        });
    }
    Ok(())
}

fn validate_type_reference(
    item_index: usize,
    position: WholeEthosEncodedIdPosition,
    reference: &WholeEthosTypeReference,
) -> Result<(), WholeEthosArchiveError> {
    match reference {
        WholeEthosTypeReference::Identity(encoded_id) => {
            validate_universal(item_index, position, encoded_id)
        }
        WholeEthosTypeReference::Application(application) => {
            validate_universal(
                item_index,
                WholeEthosEncodedIdPosition::ApplicationHead,
                application.head(),
            )?;
            validate_type_reference(item_index, position, application.payload())
        }
    }
}

/// Item kinds supported by the first Ethos slice.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum WholeEthosItem {
    /// One attribute-free newtype declaration.
    Newtype(WholeEthosNewtype),
    /// One attribute-free, non-generic enumeration declaration.
    Enumeration(WholeEthosEnumeration),
}

/// One positional newtype payload.
///
/// The positions are the declaration identity, item visibility, the typed empty
/// attribute sequence, and the wrapped field. Field names do not enter the
/// encoded value.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosNewtype(
    VocabularyEncodedId,
    WholeEthosVisibility,
    WholeEthosAttributes,
    WholeEthosWrappedField,
);

impl WholeEthosNewtype {
    /// Construct one positional newtype.
    pub fn new(
        name: VocabularyEncodedId,
        visibility: WholeEthosVisibility,
        attributes: WholeEthosAttributes,
        wrapped_field: WholeEthosWrappedField,
    ) -> Self {
        Self(name, visibility, attributes, wrapped_field)
    }

    /// Complete translator-issued declaration identity.
    pub const fn name(&self) -> &VocabularyEncodedId {
        &self.0
    }

    /// Item visibility.
    pub const fn visibility(&self) -> &WholeEthosVisibility {
        &self.1
    }

    /// Typed attribute sequence, empty in this slice.
    pub const fn attributes(&self) -> &WholeEthosAttributes {
        &self.2
    }

    /// Positional wrapped field.
    pub const fn wrapped_field(&self) -> &WholeEthosWrappedField {
        &self.3
    }
}

/// The closed visibility vocabulary needed by the first slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum WholeEthosVisibility {
    /// Exported item.
    Public,
    /// Private wrapped field.
    Private,
}

/// The typed empty attribute sequence admitted by the first slice.
///
/// This is a position in the newtype payload, not an omitted or inferred field.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub struct WholeEthosAttributes;

impl WholeEthosAttributes {
    /// Construct the only attribute sequence currently admitted.
    pub const fn empty() -> Self {
        Self
    }

    /// This first-slice sequence contains no attributes.
    pub const fn is_empty(self) -> bool {
        true
    }
}

/// A positional newtype field: visibility followed by referenced type identity.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosWrappedField(WholeEthosVisibility, WholeEthosTypeReference);

impl WholeEthosWrappedField {
    /// Construct the wrapped field.
    pub fn new(visibility: WholeEthosVisibility, reference: WholeEthosTypeReference) -> Self {
        Self(visibility, reference)
    }

    /// Wrapped-field visibility.
    pub const fn visibility(&self) -> &WholeEthosVisibility {
        &self.0
    }

    /// Complete lookup-resolved type identity.
    pub const fn reference(&self) -> &WholeEthosTypeReference {
        &self.1
    }
}

/// A positional type reference.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum WholeEthosTypeReference {
    /// One complete lookup-resolved identity.
    Identity(VocabularyEncodedId),
    /// A unary application whose head and payload are both typed references.
    Application(WholeEthosTypeApplication),
}

/// A positional unary type application such as `Vector.Integer`.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[rkyv(
    serialize_bounds(__S: rkyv::ser::Writer + rkyv::ser::Allocator, __S::Error: rkyv::rancor::Source),
    deserialize_bounds(__D::Error: rkyv::rancor::Source),
    bytecheck(bounds(__C: rkyv::validation::ArchiveContext, __C::Error: rkyv::rancor::Source)),
)]
pub struct WholeEthosTypeApplication(
    VocabularyEncodedId,
    #[rkyv(omit_bounds)] Box<WholeEthosTypeReference>,
);

impl WholeEthosTypeApplication {
    /// Construct a unary application.
    pub fn new(head: VocabularyEncodedId, payload: WholeEthosTypeReference) -> Self {
        Self(head, Box::new(payload))
    }

    /// Lookup-resolved application head.
    pub const fn head(&self) -> &VocabularyEncodedId {
        &self.0
    }

    /// Positional application payload.
    pub const fn payload(&self) -> &WholeEthosTypeReference {
        &self.1
    }
}

/// One positional enumeration payload.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosEnumeration(
    VocabularyEncodedId,
    WholeEthosVisibility,
    WholeEthosAttributes,
    Vec<WholeEthosVariant>,
);

impl WholeEthosEnumeration {
    /// Construct an attribute-free enumeration.
    pub fn new(
        name: VocabularyEncodedId,
        visibility: WholeEthosVisibility,
        attributes: WholeEthosAttributes,
        variants: Vec<WholeEthosVariant>,
    ) -> Self {
        Self(name, visibility, attributes, variants)
    }

    /// Complete declaration identity.
    pub const fn name(&self) -> &VocabularyEncodedId {
        &self.0
    }

    /// Item visibility.
    pub const fn visibility(&self) -> &WholeEthosVisibility {
        &self.1
    }

    /// Typed empty attribute sequence.
    pub const fn attributes(&self) -> &WholeEthosAttributes {
        &self.2
    }

    /// Variants in authored order.
    pub fn variants(&self) -> &[WholeEthosVariant] {
        &self.3
    }
}

/// One enumeration variant with positional payload data.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosVariant(
    VocabularyEncodedId,
    WholeEthosAttributes,
    WholeEthosVariantPayload,
);

impl WholeEthosVariant {
    /// Construct one variant.
    pub fn new(
        name: VocabularyEncodedId,
        attributes: WholeEthosAttributes,
        payload: WholeEthosVariantPayload,
    ) -> Self {
        Self(name, attributes, payload)
    }

    /// Complete declaration identity.
    pub const fn name(&self) -> &VocabularyEncodedId {
        &self.0
    }

    /// Typed empty attribute sequence.
    pub const fn attributes(&self) -> &WholeEthosAttributes {
        &self.1
    }

    /// Unit or positional tuple payload.
    pub const fn payload(&self) -> &WholeEthosVariantPayload {
        &self.2
    }
}

/// Closed variant-payload vocabulary for this fixture breadth.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum WholeEthosVariantPayload {
    /// A unit variant.
    Unit,
    /// One or more positional fields.
    Tuple(WholeEthosTupleFields),
}

/// Positional tuple fields. No stored field-name position exists.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct WholeEthosTupleFields(Vec<WholeEthosTypeReference>);

impl WholeEthosTupleFields {
    /// Construct a non-empty positional tuple payload.
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

/// Encoded-ID position rejected while restoring a first-slice archive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WholeEthosEncodedIdPosition {
    /// Item declaration identity.
    ItemName,
    /// Newtype wrapped-field reference.
    NewtypeField,
    /// Enumeration variant declaration identity.
    VariantName,
    /// Enumeration tuple-field reference.
    VariantField,
    /// Unary application head.
    ApplicationHead,
}

/// Failure at the first-slice Whole-Ethos archive boundary.
#[derive(Clone, Debug, thiserror::Error)]
pub enum WholeEthosArchiveError {
    /// Canonical archive serialization or validated reconstruction failed.
    #[error("whole-Ethos portable archive failed: {0}")]
    Archive(#[from] ArchiveError),

    /// A stored name position contains the empty chain reserved for table addresses.
    #[error("whole-Ethos item {item_index} has an empty encoded-ID chain at {position:?}")]
    EmptyEncodedId {
        /// Ordered item index.
        item_index: usize,
        /// Positional encoded-ID role.
        position: WholeEthosEncodedIdPosition,
    },

    /// Ethos declarations and references in this slice must use shared vocabulary.
    #[error("whole-Ethos item {item_index} uses non-Universal root {root:?} at {position:?}")]
    NonUniversalEncodedId {
        /// Ordered item index.
        item_index: usize,
        /// Positional encoded-ID role.
        position: WholeEthosEncodedIdPosition,
        /// Unexpected vocabulary root.
        root: VocabularyRoot,
    },
}

/// Translator-issued structural type identities needed by the six-slot codec.
///
/// These identify the document record and its expected positional shapes. The
/// codec never manufactures them and never turns their spellings into IDs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SixSlotGrammarIds {
    document: EncodedTypeId<VocabularyRoot>,
    empty_braces: EncodedTypeId<VocabularyRoot>,
    empty_square: EncodedTypeId<VocabularyRoot>,
    types_block: EncodedTypeId<VocabularyRoot>,
    item: EncodedTypeId<VocabularyRoot>,
    variant: EncodedTypeId<VocabularyRoot>,
    type_reference: EncodedTypeId<VocabularyRoot>,
}

impl SixSlotGrammarIds {
    /// Bind the grammar to seven complete Universal encoded-ID chains.
    pub fn new(
        document: VocabularyEncodedId,
        empty_braces: VocabularyEncodedId,
        empty_square: VocabularyEncodedId,
        types_block: VocabularyEncodedId,
        item: VocabularyEncodedId,
        variant: VocabularyEncodedId,
        type_reference: VocabularyEncodedId,
    ) -> Result<Self, SixSlotGrammarError> {
        validate_grammar_id(SixSlotGrammarIdPosition::Document, &document)?;
        validate_grammar_id(SixSlotGrammarIdPosition::EmptyBraces, &empty_braces)?;
        validate_grammar_id(SixSlotGrammarIdPosition::EmptySquare, &empty_square)?;
        validate_grammar_id(SixSlotGrammarIdPosition::TypesBlock, &types_block)?;
        validate_grammar_id(SixSlotGrammarIdPosition::Item, &item)?;
        validate_grammar_id(SixSlotGrammarIdPosition::Variant, &variant)?;
        validate_grammar_id(SixSlotGrammarIdPosition::TypeReference, &type_reference)?;
        Ok(Self {
            document: EncodedTypeId::new(document),
            empty_braces: EncodedTypeId::new(empty_braces),
            empty_square: EncodedTypeId::new(empty_square),
            types_block: EncodedTypeId::new(types_block),
            item: EncodedTypeId::new(item),
            variant: EncodedTypeId::new(variant),
            type_reference: EncodedTypeId::new(type_reference),
        })
    }
}

fn validate_grammar_id(
    position: SixSlotGrammarIdPosition,
    encoded_id: &VocabularyEncodedId,
) -> Result<(), SixSlotGrammarError> {
    if encoded_id.root_variant() == &VocabularyRoot::Universal {
        Ok(())
    } else {
        Err(SixSlotGrammarError::NonUniversal {
            position,
            root: *encoded_id.root_variant(),
        })
    }
}

/// Structural identity position rejected while assembling the grammar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SixSlotGrammarIdPosition {
    /// Whole document record.
    Document,
    /// Empty brace slot shape.
    EmptyBraces,
    /// Empty square slot shape.
    EmptySquare,
    /// Types block.
    TypesBlock,
    /// Type item.
    Item,
    /// Enumeration variant.
    Variant,
    /// Recursive type reference.
    TypeReference,
}

/// Failure before a six-slot structuretree can be sealed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SixSlotGrammarError {
    /// Ethos grammar identities are shared Universal vocabulary.
    #[error("six-slot grammar identity {position:?} uses non-Universal root {root:?}")]
    NonUniversal {
        /// Grammar identity position.
        position: SixSlotGrammarIdPosition,
        /// Unexpected root.
        root: VocabularyRoot,
    },
}

macro_rules! slice_role {
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

slice_role!(DocumentRootRole, 2001);
slice_role!(ImportsRole, 2002);
slice_role!(InputRole, 2003);
slice_role!(OutputRole, 2004);
slice_role!(TypesRole, 2005);
slice_role!(GenericsRole, 2006);
slice_role!(ImplsRole, 2007);
slice_role!(DelimitedRootRole, 2010);
slice_role!(DelimitedItemsRole, 2011);

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
struct SixSlotDocumentRecord {
    document: Position<DocumentRootRole, VocabularyRoot>,
    imports: Position<ImportsRole, VocabularyRoot>,
    input: Position<InputRole, VocabularyRoot>,
    output: Position<OutputRole, VocabularyRoot>,
    types: Position<TypesRole, VocabularyRoot>,
    generics: Position<GenericsRole, VocabularyRoot>,
    impls: Position<ImplsRole, VocabularyRoot>,
}

impl SixSlotDocumentRecord {
    fn new(ids: &SixSlotGrammarIds) -> Result<Self, slice_structural_codec::AuthoringError> {
        let product = OrderedProduct::try_new::<ImportsRole>()?
            .then::<InputRole>()?
            .then::<OutputRole>()?
            .then::<TypesRole>()?
            .then::<GenericsRole>()?
            .then::<ImplsRole>()?;
        let delegate = |target: &EncodedTypeId<VocabularyRoot>| SharedDescriptor::Delegate {
            target: target.clone(),
            payload: None,
        };
        Ok(Self {
            document: Position::try_new(SharedDescriptor::OrderedProduct(product))?,
            imports: Position::try_new(delegate(&ids.empty_braces))?,
            input: Position::try_new(delegate(&ids.empty_square))?,
            output: Position::try_new(delegate(&ids.empty_square))?,
            types: Position::try_new(delegate(&ids.types_block))?,
            generics: Position::try_new(delegate(&ids.empty_braces))?,
            impls: Position::try_new(delegate(&ids.empty_braces))?,
        })
    }
}

struct SixSlotDocumentView<'record> {
    record: &'record SixSlotDocumentRecord,
}

impl BorrowedFieldView<VocabularyRoot> for SixSlotDocumentView<'_> {
    fn expose<Visitor: FieldVisitor<VocabularyRoot>>(&self, visitor: &mut Visitor) {
        visitor.field(&self.record.document);
        visitor.field(&self.record.imports);
        visitor.field(&self.record.input);
        visitor.field(&self.record.output);
        visitor.field(&self.record.types);
        visitor.field(&self.record.generics);
        visitor.field(&self.record.impls);
    }
}

impl StructureRecord<VocabularyRoot> for SixSlotDocumentRecord {
    type View<'record> = SixSlotDocumentView<'record>;

    fn root_role(&self) -> StableRoleId {
        self.document.role()
    }

    fn fields(&self) -> Self::View<'_> {
        SixSlotDocumentView { record: self }
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
struct DelimitedItemsRecord {
    delimited: Position<DelimitedRootRole, VocabularyRoot>,
    items: Position<DelimitedItemsRole, VocabularyRoot>,
}

impl DelimitedItemsRecord {
    fn new(
        boundary: TriggerIdentifier,
        item: &EncodedTypeId<VocabularyRoot>,
        minimum: u64,
        maximum: Option<u64>,
    ) -> Result<Self, slice_structural_codec::AuthoringError> {
        let items = Position::try_new(SharedDescriptor::Repeated {
            minimum,
            maximum,
            element: Box::new(SharedDescriptor::Delegate {
                target: item.clone(),
                payload: None,
            }),
        })?;
        Ok(Self {
            delimited: Position::try_new(SharedDescriptor::Delimited {
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
        visitor.field(&self.record.delimited);
        visitor.field(&self.record.items);
    }
}

impl StructureRecord<VocabularyRoot> for DelimitedItemsRecord {
    type View<'record> = DelimitedItemsView<'record>;

    fn root_role(&self) -> StableRoleId {
        self.delimited.role()
    }

    fn fields(&self) -> Self::View<'_> {
        DelimitedItemsView { record: self }
    }
}

type SliceOneRule = RuleCoproduct<
    SixSlotDocumentRecord,
    RuleCoproduct<DelimitedItemsRecord, StructuralRule<VocabularyRoot>>,
>;

/// The one table-driven decoder for the bounded six-slot Ethos fixture breadth.
#[derive(Clone, Debug)]
pub struct SixSlotEthosCodec {
    ids: SixSlotGrammarIds,
    priors: SliceOneBuiltinPriors,
    table: AddressedStructuralTable<VocabularyRoot, SliceOneRule>,
}

impl SixSlotEthosCodec {
    /// Seal the typed structuretree against caller-supplied grammar identities.
    pub fn build(
        ids: SixSlotGrammarIds,
        priors: SliceOneBuiltinPriors,
    ) -> Result<Self, SixSlotCodecBuildError> {
        let document_rule = SliceOneRule::Left(SixSlotDocumentRecord::new(&ids)?);
        let empty_braces_rule = SliceOneRule::Right(RuleCoproduct::Left(
            DelimitedItemsRecord::new(BRACE_BOUNDARY, &ids.item, 0, Some(0))?,
        ));
        let empty_square_rule = SliceOneRule::Right(RuleCoproduct::Left(
            DelimitedItemsRecord::new(SQUARE_BOUNDARY, &ids.item, 0, Some(0))?,
        ));
        let types_rule = SliceOneRule::Right(RuleCoproduct::Left(DelimitedItemsRecord::new(
            BRACE_BOUNDARY,
            &ids.item,
            1,
            None,
        )?));
        let newtype_rule = SliceOneRule::Right(RuleCoproduct::Right(StructuralRule::Application(
            ApplicationRule::new(
                APPLICATION_OPERATOR,
                SharedDescriptor::Declaration(AtomDescriptor::with_case(AtomCase::PascalCase)),
                SharedDescriptor::Delegate {
                    target: ids.type_reference.clone(),
                    payload: None,
                },
            )?,
        )));
        let enumeration_rule = SliceOneRule::Right(RuleCoproduct::Right(
            StructuralRule::ApplicationDelimited(ApplicationDelimitedRule::new(
                APPLICATION_OPERATOR,
                BRACE_BOUNDARY,
                SharedDescriptor::Declaration(AtomDescriptor::with_case(AtomCase::PascalCase)),
                SharedDescriptor::Delegate {
                    target: ids.variant.clone(),
                    payload: None,
                },
                1,
                None,
            )?),
        ));
        let unit_variant_rule =
            SliceOneRule::Right(RuleCoproduct::Right(StructuralRule::Unary(UnaryRule::new(
                SharedDescriptor::Declaration(AtomDescriptor::with_case(AtomCase::PascalCase)),
            )?)));
        let tuple_variant_rule = SliceOneRule::Right(RuleCoproduct::Right(
            StructuralRule::ApplicationDelimited(ApplicationDelimitedRule::new(
                APPLICATION_OPERATOR,
                BRACE_BOUNDARY,
                SharedDescriptor::Declaration(AtomDescriptor::with_case(AtomCase::PascalCase)),
                SharedDescriptor::Delegate {
                    target: ids.type_reference.clone(),
                    payload: None,
                },
                1,
                None,
            )?),
        ));
        let identity_reference_rule =
            SliceOneRule::Right(RuleCoproduct::Right(StructuralRule::Unary(UnaryRule::new(
                SharedDescriptor::Reference(AtomDescriptor::with_case(AtomCase::PascalCase)),
            )?)));
        let application_reference_rule = SliceOneRule::Right(RuleCoproduct::Right(
            StructuralRule::Application(ApplicationRule::new(
                APPLICATION_OPERATOR,
                SharedDescriptor::Reference(AtomDescriptor::with_case(AtomCase::PascalCase)),
                SharedDescriptor::Delegate {
                    target: ids.type_reference.clone(),
                    payload: None,
                },
            )?),
        ));
        let profile = SealedTokenProfile::standard();
        let table = AddressedStructuralTable::seal(
            TableIdentityPayload::new(
                TargetLayoutIdentity::derive(
                    b"core-ethos six-slot enum and unary-application fixture layout v2",
                ),
                profile.identity(),
                StructuralVocabularyIdentity::language(
                    b"core-ethos six-slot enum and unary-application typed vocabulary v2",
                ),
                six_slot_discovery(),
                six_slot_rendering(),
                vec![
                    typed_entry(ids.document.clone(), document_rule),
                    typed_entry(ids.empty_braces.clone(), empty_braces_rule),
                    typed_entry(ids.empty_square.clone(), empty_square_rule),
                    typed_entry(ids.types_block.clone(), types_rule),
                    typed_entry_with_rules(
                        ids.item.clone(),
                        vec![(0, newtype_rule), (1, enumeration_rule)],
                    ),
                    typed_entry_with_rules(
                        ids.variant.clone(),
                        vec![(0, unit_variant_rule), (1, tuple_variant_rule)],
                    ),
                    typed_entry_with_rules(
                        ids.type_reference.clone(),
                        vec![
                            (0, identity_reference_rule),
                            (1, application_reference_rule),
                        ],
                    ),
                ],
            ),
            &profile,
        )
        .map_err(|error| SixSlotCodecBuildError::Table(Box::new(error)))?;
        Ok(Self { ids, priors, table })
    }

    /// Decode all six heterogeneous roots through one full-source evaluator.
    ///
    /// Name bindings are read-only: declaration positions require an assignment
    /// already issued by the translator, and reference positions require an
    /// already-resolved prior. No spelling-to-ID operation exists here.
    pub fn decode<Bindings: DecodeNameBindings<VocabularyRoot> + ?Sized>(
        &self,
        source: &str,
        bindings: &Bindings,
    ) -> Result<DecodedSixSlotEthos, SixSlotDecodeError> {
        let decoded = StructuralEvaluator::new(&self.table)?.decode_text_bounded(
            &self.ids.document,
            source,
            bindings,
        )?;
        let value = decoded.value();
        ensure_delegated::<ImportsRole>(value, "imports")?;
        ensure_delegated::<InputRole>(value, "input")?;
        ensure_delegated::<OutputRole>(value, "output")?;
        ensure_delegated::<GenericsRole>(value, "generics")?;
        ensure_delegated::<ImplsRole>(value, "impls")?;
        let types = delegated::<TypesRole>(value, "types")?;
        let declarations = repeated::<DelimitedItemsRole>(types, "types declarations")?;
        let mut items = Vec::with_capacity(declarations.len());
        for declaration in declarations {
            let FieldValue::Delegated(declaration) = declaration else {
                return Err(SixSlotDecodeError::Shape("type declaration"));
            };
            items.push(self.reify_item(declaration)?);
        }

        let bounds = SixSlotSourceBounds(
            field_bound::<ImportsRole>(&decoded, "imports")?,
            field_bound::<InputRole>(&decoded, "input")?,
            field_bound::<OutputRole>(&decoded, "output")?,
            field_bound::<TypesRole>(&decoded, "types")?,
            field_bound::<GenericsRole>(&decoded, "generics")?,
            field_bound::<ImplsRole>(&decoded, "impls")?,
        );
        let ethos = WholeEthos::new(items);
        Ok(DecodedSixSlotEthos(ethos, bounds))
    }

    fn reify_item(
        &self,
        declaration: &slice_structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosItem, SixSlotDecodeError> {
        if declaration.constructor() == &EncodedConstructorId::under(&self.ids.item, 0) {
            let name = declaration_id::<ApplicationHead>(declaration, "newtype identity")?;
            let wrapped = delegated::<ApplicationPayload>(declaration, "newtype reference")?;
            return Ok(WholeEthosItem::Newtype(WholeEthosNewtype::new(
                name,
                WholeEthosVisibility::Public,
                WholeEthosAttributes::empty(),
                WholeEthosWrappedField::new(
                    WholeEthosVisibility::Private,
                    self.reify_reference(wrapped)?,
                ),
            )));
        }
        if declaration.constructor() == &EncodedConstructorId::under(&self.ids.item, 1) {
            let name =
                declaration_id::<ApplicationDelimitedHead>(declaration, "enumeration identity")?;
            let encoded_variants =
                repeated::<ApplicationDelimitedItems>(declaration, "enumeration variants")?;
            let mut variants = Vec::with_capacity(encoded_variants.len());
            for variant in encoded_variants {
                let FieldValue::Delegated(variant) = variant else {
                    return Err(SixSlotDecodeError::Shape("enumeration variant"));
                };
                variants.push(self.reify_variant(variant)?);
            }
            return Ok(WholeEthosItem::Enumeration(WholeEthosEnumeration::new(
                name,
                WholeEthosVisibility::Public,
                WholeEthosAttributes::empty(),
                variants,
            )));
        }
        Err(SixSlotDecodeError::Shape("type-item constructor"))
    }

    fn reify_variant(
        &self,
        variant: &slice_structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosVariant, SixSlotDecodeError> {
        if variant.constructor() == &EncodedConstructorId::under(&self.ids.variant, 0) {
            return Ok(WholeEthosVariant::new(
                declaration_id::<UnaryRoot>(variant, "unit variant identity")?,
                WholeEthosAttributes::empty(),
                WholeEthosVariantPayload::Unit,
            ));
        }
        if variant.constructor() == &EncodedConstructorId::under(&self.ids.variant, 1) {
            let name =
                declaration_id::<ApplicationDelimitedHead>(variant, "tuple variant identity")?;
            let encoded_fields =
                repeated::<ApplicationDelimitedItems>(variant, "tuple variant fields")?;
            let mut fields = Vec::with_capacity(encoded_fields.len());
            for field in encoded_fields {
                let FieldValue::Delegated(field) = field else {
                    return Err(SixSlotDecodeError::Shape("tuple variant field"));
                };
                fields.push(self.reify_reference(field)?);
            }
            return Ok(WholeEthosVariant::new(
                name,
                WholeEthosAttributes::empty(),
                WholeEthosVariantPayload::Tuple(
                    WholeEthosTupleFields::new(fields)
                        .map_err(|_| SixSlotDecodeError::Shape("non-empty tuple variant"))?,
                ),
            ));
        }
        Err(SixSlotDecodeError::Shape("variant constructor"))
    }

    fn reify_reference(
        &self,
        reference: &slice_structural_codec::StructuralValue<VocabularyRoot>,
    ) -> Result<WholeEthosTypeReference, SixSlotDecodeError> {
        if reference.constructor() == &EncodedConstructorId::under(&self.ids.type_reference, 0) {
            let identity = reference_id::<UnaryRoot>(reference, "identity reference")?;
            if &identity != self.priors.integer() {
                return Err(SixSlotDecodeError::BuiltinPriorMismatch {
                    expected: self.priors.integer().clone(),
                    found: identity,
                });
            }
            return Ok(WholeEthosTypeReference::Identity(identity));
        }
        if reference.constructor() == &EncodedConstructorId::under(&self.ids.type_reference, 1) {
            let head = reference_id::<ApplicationHead>(reference, "application reference head")?;
            if &head != self.priors.vector() {
                return Err(SixSlotDecodeError::BuiltinPriorMismatch {
                    expected: self.priors.vector().clone(),
                    found: head,
                });
            }
            let payload =
                delegated::<ApplicationPayload>(reference, "application reference payload")?;
            return Ok(WholeEthosTypeReference::Application(
                WholeEthosTypeApplication::new(head, self.reify_reference(payload)?),
            ));
        }
        Err(SixSlotDecodeError::Shape("type-reference constructor"))
    }
}

/// Lookup-only builtin identities required by the first Ethos slice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SliceOneBuiltinPriors(VocabularyEncodedId, VocabularyEncodedId);

impl SliceOneBuiltinPriors {
    /// Register exact Universal identities already assigned to Integer and Vector.
    pub fn new(
        integer: VocabularyEncodedId,
        vector: VocabularyEncodedId,
    ) -> Result<Self, SliceOneBuiltinPriorError> {
        validate_builtin_prior(SliceOneBuiltinPriorPosition::Integer, &integer)?;
        validate_builtin_prior(SliceOneBuiltinPriorPosition::Vector, &vector)?;
        Ok(Self(integer, vector))
    }

    /// Builtin Integer's complete translator-issued identity.
    pub const fn integer(&self) -> &VocabularyEncodedId {
        &self.0
    }

    /// Builtin Vector's complete translator-issued identity.
    pub const fn vector(&self) -> &VocabularyEncodedId {
        &self.1
    }
}

fn validate_builtin_prior(
    position: SliceOneBuiltinPriorPosition,
    encoded_id: &VocabularyEncodedId,
) -> Result<(), SliceOneBuiltinPriorError> {
    if encoded_id.root_variant() == &VocabularyRoot::Universal {
        Ok(())
    } else {
        Err(SliceOneBuiltinPriorError::NonUniversal {
            position,
            root: *encoded_id.root_variant(),
        })
    }
}

/// Builtin-prior position rejected during codec construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SliceOneBuiltinPriorPosition {
    /// Scalar Integer.
    Integer,
    /// Unary Vector application.
    Vector,
}

/// Invalid builtin-prior configuration.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SliceOneBuiltinPriorError {
    /// Builtin Integer is part of the shared Universal vocabulary.
    #[error("builtin {position:?} prior uses non-Universal root {root:?}")]
    NonUniversal {
        /// Builtin position.
        position: SliceOneBuiltinPriorPosition,
        /// Unexpected root.
        root: VocabularyRoot,
    },
}

fn typed_entry(
    type_id: EncodedTypeId<VocabularyRoot>,
    rule: SliceOneRule,
) -> StructuralEntry<VocabularyRoot, SliceOneRule> {
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

fn typed_entry_with_rules(
    type_id: EncodedTypeId<VocabularyRoot>,
    rules: Vec<(u16, SliceOneRule)>,
) -> StructuralEntry<VocabularyRoot, SliceOneRule> {
    let constructors = rules
        .into_iter()
        .map(|(local, rule)| {
            ConstructorCodec::new(
                EncodedConstructorId::under(&type_id, local),
                vec![AcceptedDecodeForm::new(DecodeFormId::new(0), rule.clone())],
                rule,
            )
        })
        .collect();
    StructuralEntry::new(type_id, constructors)
}

fn six_slot_discovery() -> BlockTreeDiscoveryConfiguration {
    let active = TriggerSet::new(vec![SQUARE_BOUNDARY, BRACE_BOUNDARY, WHITESPACE_TRIVIA]);
    BlockTreeDiscoveryConfiguration::new(
        BoundaryDiscoveryConfiguration::new(
            ROOT_CONTEXT,
            vec![
                BoundaryDiscoveryContext::new(ROOT_CONTEXT, active.clone()),
                BoundaryDiscoveryContext::new(CHILD_CONTEXT, active),
            ],
            vec![
                BoundaryDiscoveryTransition::new(ROOT_CONTEXT, SQUARE_BOUNDARY, CHILD_CONTEXT),
                BoundaryDiscoveryTransition::new(ROOT_CONTEXT, BRACE_BOUNDARY, CHILD_CONTEXT),
                BoundaryDiscoveryTransition::new(CHILD_CONTEXT, SQUARE_BOUNDARY, CHILD_CONTEXT),
                BoundaryDiscoveryTransition::new(CHILD_CONTEXT, BRACE_BOUNDARY, CHILD_CONTEXT),
            ],
        ),
        vec![],
    )
}

fn six_slot_rendering() -> TextualRenderingPolicy {
    TextualRenderingPolicy::new(vec![
        ContextualTextualPolicy::new(ROOT_CONTEXT, Some(WHITESPACE_TRIVIA), None),
        ContextualTextualPolicy::new(CHILD_CONTEXT, Some(WHITESPACE_TRIVIA), None),
    ])
}

fn ensure_delegated<Role: FieldRole>(
    value: &slice_structural_codec::StructuralValue<VocabularyRoot>,
    what: &'static str,
) -> Result<(), SixSlotDecodeError> {
    delegated::<Role>(value, what).map(|_| ())
}

fn delegated<'value, Role: FieldRole>(
    value: &'value slice_structural_codec::StructuralValue<VocabularyRoot>,
    what: &'static str,
) -> Result<&'value slice_structural_codec::StructuralValue<VocabularyRoot>, SixSlotDecodeError> {
    match value.field::<Role>() {
        Some(FieldValue::Delegated(inner)) => Ok(inner),
        _ => Err(SixSlotDecodeError::Shape(what)),
    }
}

fn repeated<'value, Role: FieldRole>(
    value: &'value slice_structural_codec::StructuralValue<VocabularyRoot>,
    what: &'static str,
) -> Result<&'value [FieldValue<VocabularyRoot>], SixSlotDecodeError> {
    match value.field::<Role>() {
        Some(FieldValue::Repeated(items)) => Ok(items),
        _ => Err(SixSlotDecodeError::Shape(what)),
    }
}

fn declaration_id<Role: FieldRole>(
    value: &slice_structural_codec::StructuralValue<VocabularyRoot>,
    what: &'static str,
) -> Result<VocabularyEncodedId, SixSlotDecodeError> {
    match value.field::<Role>() {
        Some(FieldValue::Declaration(assignment)) => {
            let encoded_id = assignment.encoded_id().clone();
            validate_decoded_id(DecodedEncodedIdPosition::Declaration, &encoded_id)?;
            Ok(encoded_id)
        }
        _ => Err(SixSlotDecodeError::Shape(what)),
    }
}

fn reference_id<Role: FieldRole>(
    value: &slice_structural_codec::StructuralValue<VocabularyRoot>,
    what: &'static str,
) -> Result<VocabularyEncodedId, SixSlotDecodeError> {
    match value.field::<Role>() {
        Some(FieldValue::Reference(reference)) => {
            let encoded_id = reference.encoded_id().clone();
            validate_decoded_id(DecodedEncodedIdPosition::Reference, &encoded_id)?;
            Ok(encoded_id)
        }
        _ => Err(SixSlotDecodeError::Shape(what)),
    }
}

fn field_bound<Role: FieldRole>(
    value: &slice_structural_codec::SourceBoundedStructuralValue<VocabularyRoot>,
    what: &'static str,
) -> Result<SourceBound, SixSlotDecodeError> {
    value
        .field_bound::<Role>()
        .ok_or(SixSlotDecodeError::MissingSourceBound(what))
}

fn validate_decoded_id(
    position: DecodedEncodedIdPosition,
    encoded_id: &VocabularyEncodedId,
) -> Result<(), SixSlotDecodeError> {
    if encoded_id.root_variant() == &VocabularyRoot::Universal {
        Ok(())
    } else {
        Err(SixSlotDecodeError::NonUniversalEncodedId {
            position,
            root: *encoded_id.root_variant(),
        })
    }
}

/// Result of decoding the first complete six-slot document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedSixSlotEthos(WholeEthos, SixSlotSourceBounds);

impl DecodedSixSlotEthos {
    /// Stringless positional Ethos content.
    pub const fn ethos(&self) -> &WholeEthos {
        &self.0
    }

    /// Exact full-source bounds of the six document slots.
    pub const fn source_bounds(&self) -> &SixSlotSourceBounds {
        &self.1
    }

    /// Consume the decode result.
    pub fn into_parts(self) -> (WholeEthos, SixSlotSourceBounds) {
        (self.0, self.1)
    }
}

/// Exact full-source bounds in document order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SixSlotSourceBounds(
    SourceBound,
    SourceBound,
    SourceBound,
    SourceBound,
    SourceBound,
    SourceBound,
);

impl SixSlotSourceBounds {
    /// Imports slot.
    pub const fn imports(&self) -> SourceBound {
        self.0
    }

    /// Input-interface slot.
    pub const fn input(&self) -> SourceBound {
        self.1
    }

    /// Output-interface slot.
    pub const fn output(&self) -> SourceBound {
        self.2
    }

    /// Type-declaration slot.
    pub const fn types(&self) -> SourceBound {
        self.3
    }

    /// Generics slot.
    pub const fn generics(&self) -> SourceBound {
        self.4
    }

    /// Implementation slot.
    pub const fn impls(&self) -> SourceBound {
        self.5
    }
}

/// Encoded-ID role rejected after typed structural evaluation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodedEncodedIdPosition {
    /// Declared item.
    Declaration,
    /// Wrapped type reference.
    Reference,
}

/// Failure while sealing the first-slice structuretree.
#[derive(Clone, Debug, thiserror::Error)]
pub enum SixSlotCodecBuildError {
    /// A typed record could not be authored.
    #[error(transparent)]
    Authoring(#[from] slice_structural_codec::AuthoringError),

    /// The complete structural table refused to seal.
    #[error(transparent)]
    Table(Box<slice_structural_codec::TableError<VocabularyRoot>>),
}

/// Failure while decoding or reifying the first six-slot document.
#[derive(Clone, Debug, thiserror::Error)]
pub enum SixSlotDecodeError {
    /// Shared structural evaluation refused the source.
    #[error(transparent)]
    Structural(#[from] slice_structural_codec::DecodeError<VocabularyRoot>),

    /// The selected structural mirror did not contain its typed role value.
    #[error("the six-slot structural value did not fit {0}")]
    Shape(&'static str),

    /// A source-bound role was unexpectedly absent.
    #[error("the six-slot structural value omitted the source bound for {0}")]
    MissingSourceBound(&'static str),

    /// The first Ethos slice only carries universally shared vocabulary.
    #[error("decoded {position:?} uses non-Universal root {root:?}")]
    NonUniversalEncodedId {
        /// Name role.
        position: DecodedEncodedIdPosition,
        /// Unexpected root.
        root: VocabularyRoot,
    },

    /// Reference resolution returned a valid name that is not builtin Integer.
    #[error("type reference resolved to {found:?}, expected fixture builtin {expected:?}")]
    BuiltinPriorMismatch {
        /// Registered lookup-only Integer prior.
        expected: VocabularyEncodedId,
        /// Identity resolved at the reference occurrence.
        found: VocabularyEncodedId,
    },
}
