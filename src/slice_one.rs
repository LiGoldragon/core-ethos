//! The first complete Ethos slice over translator-issued encoded-ID chains.
//!
//! This module is deliberately separate from the legacy flat-identifier algebra.
//! Its only item is the attribute-free newtype needed by the first vertical
//! witness. Every name position carries the complete Universal encoded-ID chain
//! supplied by the naming authority.

use content_identity::{ArchiveError, PortableArchive};
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use slice_raw_discovery::{
    BlockTreeDiscoveryConfiguration, BoundaryDiscoveryConfiguration, BoundaryDiscoveryContext,
    BoundaryDiscoveryContextIdentifier, BoundaryDiscoveryTransition, SealedTokenProfile,
    TriggerIdentifier, TriggerSet,
};
use slice_structural_codec::{
    AcceptedDecodeForm, AddressedStructuralTable, ApplicationHead, ApplicationPayload,
    ApplicationRule, AtomCase, AtomDescriptor, BorrowedFieldView, ConstructorCodec,
    ContextualTextualPolicy, DecodeFormId, DecodeNameBindings, EncodedConstructorId, EncodedTypeId,
    FieldRole, FieldValue, FieldVisitor, OrderedProduct, Position, RuleCoproduct, SharedDescriptor,
    StableRoleId, StructuralEntry, StructuralEvaluator, StructuralRule,
    StructuralVocabularyIdentity, StructureRecord, TableIdentityPayload, TargetLayoutIdentity,
    TextualRenderingPolicy,
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
                    validate_universal(item_index, NewtypeEncodedIdPosition::Name, newtype.name())?;
                    validate_universal(
                        item_index,
                        NewtypeEncodedIdPosition::Wrapped,
                        newtype.wrapped_field().reference(),
                    )?;
                }
            }
        }
        Ok(())
    }
}

fn validate_universal(
    item_index: usize,
    position: NewtypeEncodedIdPosition,
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

/// Item kinds supported by the first Ethos slice.
#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub enum WholeEthosItem {
    /// One attribute-free newtype declaration.
    Newtype(WholeEthosNewtype),
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
pub struct WholeEthosWrappedField(WholeEthosVisibility, VocabularyEncodedId);

impl WholeEthosWrappedField {
    /// Construct the wrapped field.
    pub fn new(visibility: WholeEthosVisibility, reference: VocabularyEncodedId) -> Self {
        Self(visibility, reference)
    }

    /// Wrapped-field visibility.
    pub const fn visibility(&self) -> &WholeEthosVisibility {
        &self.0
    }

    /// Complete lookup-resolved type identity.
    pub const fn reference(&self) -> &VocabularyEncodedId {
        &self.1
    }
}

/// Encoded-ID position rejected while restoring a first-slice archive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NewtypeEncodedIdPosition {
    /// Declaration identity.
    Name,
    /// Wrapped type reference.
    Wrapped,
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
        position: NewtypeEncodedIdPosition,
    },

    /// Ethos declarations and references in this slice must use shared vocabulary.
    #[error("whole-Ethos item {item_index} uses non-Universal root {root:?} at {position:?}")]
    NonUniversalEncodedId {
        /// Ordered item index.
        item_index: usize,
        /// Positional encoded-ID role.
        position: NewtypeEncodedIdPosition,
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
    newtype_declaration: EncodedTypeId<VocabularyRoot>,
}

impl SixSlotGrammarIds {
    /// Bind the grammar to five complete Universal encoded-ID chains.
    pub fn new(
        document: VocabularyEncodedId,
        empty_braces: VocabularyEncodedId,
        empty_square: VocabularyEncodedId,
        types_block: VocabularyEncodedId,
        newtype_declaration: VocabularyEncodedId,
    ) -> Result<Self, SixSlotGrammarError> {
        validate_grammar_id(SixSlotGrammarIdPosition::Document, &document)?;
        validate_grammar_id(SixSlotGrammarIdPosition::EmptyBraces, &empty_braces)?;
        validate_grammar_id(SixSlotGrammarIdPosition::EmptySquare, &empty_square)?;
        validate_grammar_id(SixSlotGrammarIdPosition::TypesBlock, &types_block)?;
        validate_grammar_id(
            SixSlotGrammarIdPosition::NewtypeDeclaration,
            &newtype_declaration,
        )?;
        Ok(Self {
            document: EncodedTypeId::new(document),
            empty_braces: EncodedTypeId::new(empty_braces),
            empty_square: EncodedTypeId::new(empty_square),
            types_block: EncodedTypeId::new(types_block),
            newtype_declaration: EncodedTypeId::new(newtype_declaration),
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
    /// Newtype declaration.
    NewtypeDeclaration,
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

/// The one table-driven decoder for the first six-slot Ethos document.
#[derive(Clone, Debug)]
pub struct SixSlotNewtypeCodec {
    ids: SixSlotGrammarIds,
    priors: SliceOneBuiltinPriors,
    table: AddressedStructuralTable<VocabularyRoot, SliceOneRule>,
}

impl SixSlotNewtypeCodec {
    /// Seal the typed structuretree against caller-supplied grammar identities.
    pub fn build(
        ids: SixSlotGrammarIds,
        priors: SliceOneBuiltinPriors,
    ) -> Result<Self, SixSlotCodecBuildError> {
        let document_rule = SliceOneRule::Left(SixSlotDocumentRecord::new(&ids)?);
        let empty_braces_rule = SliceOneRule::Right(RuleCoproduct::Left(
            DelimitedItemsRecord::new(BRACE_BOUNDARY, &ids.newtype_declaration, 0, Some(0))?,
        ));
        let empty_square_rule = SliceOneRule::Right(RuleCoproduct::Left(
            DelimitedItemsRecord::new(SQUARE_BOUNDARY, &ids.newtype_declaration, 0, Some(0))?,
        ));
        let types_rule = SliceOneRule::Right(RuleCoproduct::Left(DelimitedItemsRecord::new(
            BRACE_BOUNDARY,
            &ids.newtype_declaration,
            1,
            Some(1),
        )?));
        let newtype_rule = SliceOneRule::Right(RuleCoproduct::Right(StructuralRule::Application(
            ApplicationRule::new(
                APPLICATION_OPERATOR,
                SharedDescriptor::Declaration(AtomDescriptor::with_case(AtomCase::PascalCase)),
                SharedDescriptor::Reference(AtomDescriptor::with_case(AtomCase::PascalCase)),
            )?,
        )));
        let profile = SealedTokenProfile::standard();
        let table = AddressedStructuralTable::seal(
            TableIdentityPayload::new(
                TargetLayoutIdentity::derive(b"core-ethos first six-slot positional layout"),
                profile.identity(),
                StructuralVocabularyIdentity::language(
                    b"core-ethos first six-slot typed vocabulary",
                ),
                six_slot_discovery(),
                six_slot_rendering(),
                vec![
                    typed_entry(ids.document.clone(), document_rule),
                    typed_entry(ids.empty_braces.clone(), empty_braces_rule),
                    typed_entry(ids.empty_square.clone(), empty_square_rule),
                    typed_entry(ids.types_block.clone(), types_rule),
                    typed_entry(ids.newtype_declaration.clone(), newtype_rule),
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
        let [FieldValue::Delegated(declaration)] = declarations else {
            return Err(SixSlotDecodeError::Shape("one newtype declaration"));
        };
        let name = match declaration.field::<ApplicationHead>() {
            Some(FieldValue::Declaration(assignment)) => assignment.encoded_id().clone(),
            _ => return Err(SixSlotDecodeError::Shape("newtype declaration identity")),
        };
        let wrapped = match declaration.field::<ApplicationPayload>() {
            Some(FieldValue::Reference(reference)) => reference.encoded_id().clone(),
            _ => return Err(SixSlotDecodeError::Shape("newtype reference")),
        };
        validate_decoded_id(DecodedEncodedIdPosition::Declaration, &name)?;
        validate_decoded_id(DecodedEncodedIdPosition::Reference, &wrapped)?;
        if &wrapped != self.priors.integer() {
            return Err(SixSlotDecodeError::BuiltinPriorMismatch {
                expected: self.priors.integer().clone(),
                found: wrapped,
            });
        }

        let bounds = SixSlotSourceBounds(
            field_bound::<ImportsRole>(&decoded, "imports")?,
            field_bound::<InputRole>(&decoded, "input")?,
            field_bound::<OutputRole>(&decoded, "output")?,
            field_bound::<TypesRole>(&decoded, "types")?,
            field_bound::<GenericsRole>(&decoded, "generics")?,
            field_bound::<ImplsRole>(&decoded, "impls")?,
        );
        let ethos = WholeEthos::new(vec![WholeEthosItem::Newtype(WholeEthosNewtype::new(
            name,
            WholeEthosVisibility::Public,
            WholeEthosAttributes::empty(),
            WholeEthosWrappedField::new(
                WholeEthosVisibility::Private,
                self.priors.integer().clone(),
            ),
        ))]);
        Ok(DecodedSixSlotEthos(ethos, bounds))
    }
}

/// Lookup-only builtin identities required by the first Ethos slice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SliceOneBuiltinPriors(VocabularyEncodedId);

impl SliceOneBuiltinPriors {
    /// Register the exact Universal identity already assigned to builtin Integer.
    pub fn new(integer: VocabularyEncodedId) -> Result<Self, SliceOneBuiltinPriorError> {
        if integer.root_variant() == &VocabularyRoot::Universal {
            Ok(Self(integer))
        } else {
            Err(SliceOneBuiltinPriorError::NonUniversal {
                root: *integer.root_variant(),
            })
        }
    }

    /// Builtin Integer's complete translator-issued identity.
    pub const fn integer(&self) -> &VocabularyEncodedId {
        &self.0
    }
}

/// Invalid builtin-prior configuration.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SliceOneBuiltinPriorError {
    /// Builtin Integer is part of the shared Universal vocabulary.
    #[error("builtin Integer prior uses non-Universal root {root:?}")]
    NonUniversal {
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
    #[error("newtype reference resolved to {found:?}, expected builtin Integer {expected:?}")]
    BuiltinPriorMismatch {
        /// Registered lookup-only Integer prior.
        expected: VocabularyEncodedId,
        /// Identity resolved at the reference occurrence.
        found: VocabularyEncodedId,
    },
}
