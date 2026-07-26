//! The proof-of-concept schema family and its typed structural vocabulary.

use std::collections::BTreeMap;

use raw_discovery::{
    BlockPrefixAttachment, BlockPrefixRule, BlockTreeDiscoveryConfiguration,
    BoundaryDiscoveryConfiguration, BoundaryDiscoveryContext, BoundaryDiscoveryContextIdentifier,
    BoundaryDiscoveryTransition, CharacterClass, Delimiter, RawProfile, SealedTokenProfile,
    TriggerIdentifier, TriggerSet,
};
use structural_codec::{
    AcceptedDecodeForm, AddressedStructuralTable, AtomCase, AtomDescriptor, ConstructorCodec,
    ContextualTextualPolicy, DecodeFormId, EncodedConstructorId, EncodedLanguage, LeafCodec,
    SharedDescriptor, StructuralEntry, StructuralRule, StructuralVocabularyIdentity,
    TableIdentityPayload, TargetLayoutIdentity, TextualRenderingPolicy, UnaryRule,
};

use crate::declaration::{
    EncodedDeclaration, EncodedField, EncodedNewtype, EncodedSchema, EncodedStruct, EncodedType,
};
use crate::error::UniverseError;
use crate::reference::EncodedReference;
use crate::rules::{SchemaRule, SignatureApplicationDelimitedRule, core_rule, signature_rule};
use crate::universe::{ENCODED_UNIVERSE, EncodedUniverse, EncodedUniverseBuilder, ScalarSlot};

// Schema-owned local identities retain the established values, now in the closed
// Schema namespace rather than the retired fixture universe.
pub const INTEGER: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(10);
pub const FLOAT: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(9);
pub const TEXT: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(33);
pub const SUMMARY: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(32);
pub const DOCUMENTATION: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(31);
pub const COMMIT_SEQUENCE: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(1);
pub const STATE_DIGEST: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(2);
pub const DATABASE_MARKER: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(3);
pub const FIELD: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(23);

pub(crate) const PARENTHESIS_BOUNDARY: TriggerIdentifier = TriggerIdentifier::new(0);
pub(crate) const SQUARE_BOUNDARY: TriggerIdentifier = TriggerIdentifier::new(1);
pub(crate) const BRACE_BOUNDARY: TriggerIdentifier = TriggerIdentifier::new(2);
pub(crate) const APPLICATION_OPERATOR: TriggerIdentifier = TriggerIdentifier::new(3);
pub(crate) const PIPE_CARRIER: TriggerIdentifier = TriggerIdentifier::new(4);
pub(crate) const WHITESPACE_TRIVIA: TriggerIdentifier = TriggerIdentifier::new(5);
pub(crate) const COMMENT_TRIVIA: TriggerIdentifier = TriggerIdentifier::new(6);
pub(crate) const ROOT_CONTEXT: BoundaryDiscoveryContextIdentifier =
    BoundaryDiscoveryContextIdentifier::new(1);

pub(crate) fn standard_token_profile() -> SealedTokenProfile {
    RawProfile::standard()
        .seal()
        .expect("the standard schema token profile seals")
}

pub(crate) fn standard_block_discovery() -> BlockTreeDiscoveryConfiguration {
    BlockTreeDiscoveryConfiguration::new(
        BoundaryDiscoveryConfiguration::new(
            ROOT_CONTEXT,
            vec![BoundaryDiscoveryContext::new(
                ROOT_CONTEXT,
                TriggerSet::new(vec![
                    PARENTHESIS_BOUNDARY,
                    SQUARE_BOUNDARY,
                    BRACE_BOUNDARY,
                    PIPE_CARRIER,
                    WHITESPACE_TRIVIA,
                    COMMENT_TRIVIA,
                ]),
            )],
            vec![
                BoundaryDiscoveryTransition::new(ROOT_CONTEXT, PARENTHESIS_BOUNDARY, ROOT_CONTEXT),
                BoundaryDiscoveryTransition::new(ROOT_CONTEXT, SQUARE_BOUNDARY, ROOT_CONTEXT),
                BoundaryDiscoveryTransition::new(ROOT_CONTEXT, BRACE_BOUNDARY, ROOT_CONTEXT),
            ],
        ),
        vec![
            BlockPrefixAttachment::new(
                PARENTHESIS_BOUNDARY,
                BlockPrefixRule::new(".", CharacterClass::AsciiAlphabetic),
            ),
            BlockPrefixAttachment::new(
                SQUARE_BOUNDARY,
                BlockPrefixRule::new(".", CharacterClass::AsciiAlphabetic),
            ),
            BlockPrefixAttachment::new(
                BRACE_BOUNDARY,
                BlockPrefixRule::new(".", CharacterClass::AsciiAlphabetic),
            ),
        ],
    )
}

pub(crate) fn standard_textual_rendering() -> TextualRenderingPolicy {
    TextualRenderingPolicy::new(vec![ContextualTextualPolicy::new(
        ROOT_CONTEXT,
        Some(WHITESPACE_TRIVIA),
        Some(PIPE_CARRIER),
    )])
}

#[derive(Clone, Debug)]
pub struct FixtureFamily {
    universe: EncodedUniverse,
    schema: EncodedSchema,
}

impl FixtureFamily {
    pub fn build() -> Self {
        let mut builder = EncodedUniverseBuilder::new();
        builder
            .primitive(INTEGER, "Integer", ScalarSlot::Integer)
            .expect("fixture namespace capacity");
        builder
            .primitive(TEXT, "Text", ScalarSlot::Text)
            .expect("fixture namespace capacity");
        builder
            .primitive_leaf(FLOAT, "Float")
            .expect("fixture namespace capacity");
        builder
            .field_meta(FIELD, "Field")
            .expect("fixture namespace capacity");

        let commit_sequence = builder.intern("CommitSequence").expect("fixture name");
        let state_digest = builder.intern("StateDigest").expect("fixture name");
        let text_name = builder.intern("Text").expect("fixture name");
        let summary_name = builder.intern("Summary").expect("fixture name");
        let documentation_name = builder.intern("Documentation").expect("fixture name");
        let database_marker = builder.intern("DatabaseMarker").expect("fixture name");
        let commit_field = builder.intern("commit_sequence").expect("fixture name");
        let state_field = builder.intern("state_digest").expect("fixture name");

        let commit_declaration = EncodedDeclaration::public(EncodedType::Newtype(
            EncodedNewtype::new(commit_sequence, EncodedReference::Integer),
        ));
        let state_declaration = EncodedDeclaration::public(EncodedType::Newtype(
            EncodedNewtype::new(state_digest, EncodedReference::Integer),
        ));
        let summary_declaration = EncodedDeclaration::public(EncodedType::Newtype(
            EncodedNewtype::new(summary_name, EncodedReference::Plain(text_name)),
        ));
        let documentation_declaration = EncodedDeclaration::public(EncodedType::Newtype(
            EncodedNewtype::new(documentation_name, EncodedReference::Plain(summary_name)),
        ));
        let database_declaration =
            EncodedDeclaration::public(EncodedType::Struct(EncodedStruct::new(
                database_marker,
                vec![
                    EncodedField::new(commit_field, EncodedReference::Plain(commit_sequence)),
                    EncodedField::new(state_field, EncodedReference::Plain(state_digest)),
                    EncodedField::new(state_field, EncodedReference::Plain(state_digest)),
                ],
            )));

        builder.declaration(COMMIT_SEQUENCE, commit_declaration.clone());
        builder.declaration(STATE_DIGEST, state_declaration.clone());
        builder.declaration(SUMMARY, summary_declaration.clone());
        builder.declaration(DOCUMENTATION, documentation_declaration.clone());
        builder.declaration(DATABASE_MARKER, database_declaration.clone());

        let universe = builder
            .build(ENCODED_UNIVERSE)
            .expect("fixture universe satisfies the universal builder seal");
        let schema = EncodedSchema::new(vec![
            commit_declaration,
            state_declaration,
            summary_declaration,
            documentation_declaration,
            database_declaration,
        ]);
        Self { universe, schema }
    }

    pub fn universe(&self) -> &EncodedUniverse {
        &self.universe
    }

    pub fn schema(&self) -> &EncodedSchema {
        &self.schema
    }

    pub fn standard_table(&self) -> Result<AddressedStructuralTable<SchemaRule>, UniverseError> {
        self.table(Delimiter::Brace)
    }

    pub fn table(
        &self,
        delimiter: Delimiter,
    ) -> Result<AddressedStructuralTable<SchemaRule>, UniverseError> {
        self.seal_entries(
            self.entries(
                delimiter,
                [
                    Some(COMMIT_SEQUENCE),
                    Some(STATE_DIGEST),
                    Some(STATE_DIGEST),
                ],
            )
            .into_iter()
            .map(|entry| (entry.encoded_type(), entry))
            .collect(),
        )
    }

    /// Negative control: this record keeps the executable form but replaces one
    /// archived layout witness.  Validation must reject the mismatch.
    pub fn corrupted_table(&self) -> Result<AddressedStructuralTable<SchemaRule>, UniverseError> {
        self.seal_entries(
            self.entries(
                Delimiter::Brace,
                [Some(STATE_DIGEST), Some(STATE_DIGEST), Some(STATE_DIGEST)],
            )
            .into_iter()
            .map(|entry| (entry.encoded_type(), entry))
            .collect(),
        )
    }

    fn encoded_layout(&self) -> Result<TargetLayoutIdentity, UniverseError> {
        let bytes = self
            .schema
            .to_archive_bytes()
            .map_err(|error| match error {
                crate::error::EncodedSchemaLoadError::Archive(archive) => {
                    UniverseError::Table(structural_codec::TableError::Archive(archive))
                }
                crate::error::EncodedSchemaLoadError::Schema(_) => unreachable!("fresh schema"),
            })?;
        Ok(TargetLayoutIdentity::derive(bytes.as_ref()))
    }

    fn seal_entries(
        &self,
        entries: BTreeMap<structural_codec::ScopedEncodedTypeId, StructuralEntry<SchemaRule>>,
    ) -> Result<AddressedStructuralTable<SchemaRule>, UniverseError> {
        let profile = standard_token_profile();
        Ok(AddressedStructuralTable::seal(
            TableIdentityPayload::new(
                EncodedLanguage::Schema,
                self.encoded_layout()?,
                profile.identity(),
                StructuralVocabularyIdentity::language(
                    b"core-schema fixture typed structural vocabulary",
                ),
                standard_block_discovery(),
                standard_textual_rendering(),
                entries,
            ),
            &profile,
        )?)
    }

    fn entries(
        &self,
        delimiter: Delimiter,
        database_signature: [Option<structural_codec::ScopedEncodedTypeId>; 3],
    ) -> Vec<StructuralEntry<SchemaRule>> {
        vec![
            Self::unary(INTEGER, SharedDescriptor::Leaf(LeafCodec::Integer)),
            Self::unary(FLOAT, SharedDescriptor::Leaf(LeafCodec::Float)),
            Self::unary(TEXT, SharedDescriptor::Leaf(LeafCodec::Text)),
            Self::unary(
                SUMMARY,
                SharedDescriptor::Delegate {
                    target: TEXT,
                    payload: None,
                },
            ),
            Self::unary(
                DOCUMENTATION,
                SharedDescriptor::Delegate {
                    target: SUMMARY,
                    payload: None,
                },
            ),
            Self::newtype(COMMIT_SEQUENCE, delimiter, INTEGER),
            Self::newtype(STATE_DIGEST, delimiter, INTEGER),
            Self::database_marker(database_signature),
            Self::unary(
                FIELD,
                SharedDescriptor::Atom(AtomDescriptor::with_case(AtomCase::PascalCase)),
            ),
        ]
    }

    fn unary(
        type_id: structural_codec::ScopedEncodedTypeId,
        descriptor: SharedDescriptor,
    ) -> StructuralEntry<SchemaRule> {
        Self::entry(
            type_id,
            core_rule(StructuralRule::Unary(
                UnaryRule::new(descriptor).expect("kernel role"),
            )),
        )
    }

    fn newtype(
        type_id: structural_codec::ScopedEncodedTypeId,
        delimiter: Delimiter,
        reference: structural_codec::ScopedEncodedTypeId,
    ) -> StructuralEntry<SchemaRule> {
        let rule = SignatureApplicationDelimitedRule::new(
            APPLICATION_OPERATOR,
            Self::boundary_trigger(delimiter),
            SharedDescriptor::Atom(AtomDescriptor::with_case(AtomCase::PascalCase)),
            SharedDescriptor::Atom(AtomDescriptor::with_case(AtomCase::PascalCase)),
            1,
            Some(1),
            [Some(reference), None, None],
        )
        .expect("schema roles");
        Self::entry(type_id, signature_rule(rule))
    }

    fn database_marker(
        signature: [Option<structural_codec::ScopedEncodedTypeId>; 3],
    ) -> StructuralEntry<SchemaRule> {
        let rule = SignatureApplicationDelimitedRule::new(
            APPLICATION_OPERATOR,
            BRACE_BOUNDARY,
            SharedDescriptor::Atom(AtomDescriptor::with_case(AtomCase::PascalCase)),
            SharedDescriptor::Delegate {
                target: FIELD,
                payload: None,
            },
            3,
            Some(3),
            signature,
        )
        .expect("schema roles");
        Self::entry(DATABASE_MARKER, signature_rule(rule))
    }

    fn entry(
        type_id: structural_codec::ScopedEncodedTypeId,
        rule: SchemaRule,
    ) -> StructuralEntry<SchemaRule> {
        StructuralEntry::new(
            type_id,
            vec![ConstructorCodec::new(
                EncodedConstructorId::under(type_id, 0),
                vec![AcceptedDecodeForm::new(DecodeFormId::new(0), rule.clone())],
                rule,
            )],
        )
    }

    fn boundary_trigger(delimiter: Delimiter) -> TriggerIdentifier {
        match delimiter {
            Delimiter::Parenthesis => PARENTHESIS_BOUNDARY,
            Delimiter::SquareBracket => SQUARE_BOUNDARY,
            Delimiter::Brace => BRACE_BOUNDARY,
        }
    }
}
