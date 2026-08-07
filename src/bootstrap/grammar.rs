//! One permissive structural tree followed by strict expected-position reification.
//!
//! This layer owns delimiters and application shape only. Semantic alternatives
//! are selected later from the expected bootstrap schema, never from spelling.

use super::model::BootstrapLanguage;
use name_table::EncodedName;
use name_table::TextualName;
use raw_discovery::{
    BlockTreeDiscoveryConfiguration, BoundaryDiscoveryConfiguration, BoundaryDiscoveryContext,
    BoundaryDiscoveryContextIdentifier, BoundaryDiscoveryTransition, CharacterSet, ProfileRevision,
    SealedTokenProfile, SourceBound, TokenProfileData, Trigger, TriggerDefinition,
    TriggerIdentifier, TriggerSet,
};
use structural_codec::{
    AcceptedDecodeForm, AddressedStructuralTable, AdjacentApplicationDelimitedHead,
    AdjacentApplicationDelimitedItems, AdjacentApplicationDelimitedRoot,
    AdjacentApplicationDelimitedRule, ApplicationHead, ApplicationPayload, ApplicationRoot,
    ApplicationRule, AtomDescriptor, BorrowedFieldView, ConstructorCodec, ContextualTextualPolicy,
    DecodeFormId, EncodedConstructorId, EncodedNameResolver, EncodedTypeId, FieldRole,
    FieldVisitor, OrderedSequence, PlannedFieldValue, PlannedStructuralValue, Position,
    RuleCoproduct, SharedDescriptor, StableRoleId, StructuralEntry, StructuralEvaluator,
    StructuralPlanning, StructuralRule, StructuralVocabularyIdentity, StructureRecord,
    TableIdentityPayload, TargetLayoutIdentity, TextualRenderingPolicy, UnaryRoot, UnaryRule,
};

use super::error::{BootstrapBuildError, BootstrapReadError};

const PARENTHESIS: TriggerIdentifier = TriggerIdentifier::new(0);
const SQUARE: TriggerIdentifier = TriggerIdentifier::new(1);
const BRACE: TriggerIdentifier = TriggerIdentifier::new(2);
const APPLICATION: TriggerIdentifier = TriggerIdentifier::new(3);
const TEXT_CARRIER: TriggerIdentifier = TriggerIdentifier::new(4);
const WHITESPACE: TriggerIdentifier = TriggerIdentifier::new(5);
const ANGLE: TriggerIdentifier = TriggerIdentifier::new(7);
const GUILLEMET: TriggerIdentifier = TriggerIdentifier::new(8);
const ROOT_CONTEXT: BoundaryDiscoveryContextIdentifier = BoundaryDiscoveryContextIdentifier::new(1);
const CHILD_CONTEXT: BoundaryDiscoveryContextIdentifier =
    BoundaryDiscoveryContextIdentifier::new(2);

macro_rules! bootstrap_role {
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
        pub(crate) struct $name;

        impl FieldRole for $name {
            const STABLE_ID: u16 = $id;
        }
    };
}

bootstrap_role!(DocumentRootRole, 5101);
bootstrap_role!(DelimitedRootRole, 5102);
bootstrap_role!(DelimitedItemsRole, 5103);
bootstrap_role!(HeaderSyntaxRole, 5104);
bootstrap_role!(ImportsSyntaxRole, 5105);
bootstrap_role!(BodySyntaxRole, 5106);

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
struct DocumentRecord {
    root: Position<DocumentRootRole, BootstrapLanguage>,
    header: Position<HeaderSyntaxRole, BootstrapLanguage>,
    imports: Position<ImportsSyntaxRole, BootstrapLanguage>,
    body: Position<BodySyntaxRole, BootstrapLanguage>,
}

impl DocumentRecord {
    fn new(
        syntax: &EncodedTypeId<BootstrapLanguage>,
    ) -> Result<Self, structural_codec::AuthoringError> {
        let sequence = OrderedSequence::try_new::<HeaderSyntaxRole>()?
            .then::<ImportsSyntaxRole>()?
            .then::<BodySyntaxRole>()?;
        Ok(Self {
            root: Position::try_new(SharedDescriptor::OrderedSequence(sequence))?,
            header: Position::try_new(delegate(syntax))?,
            imports: Position::try_new(delegate(syntax))?,
            body: Position::try_new(delegate(syntax))?,
        })
    }
}

struct DocumentRecordView<'a>(&'a DocumentRecord);

impl BorrowedFieldView<BootstrapLanguage> for DocumentRecordView<'_> {
    fn expose<Visitor: FieldVisitor<BootstrapLanguage>>(&self, visitor: &mut Visitor) {
        visitor.field(&self.0.root);
        visitor.field(&self.0.header);
        visitor.field(&self.0.imports);
        visitor.field(&self.0.body);
    }
}

impl StructureRecord<BootstrapLanguage> for DocumentRecord {
    type View<'record> = DocumentRecordView<'record>;

    fn root_role(&self) -> StableRoleId {
        self.root.role()
    }

    fn fields(&self) -> Self::View<'_> {
        DocumentRecordView(self)
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
struct DelimitedRecord {
    root: Position<DelimitedRootRole, BootstrapLanguage>,
    items: Position<DelimitedItemsRole, BootstrapLanguage>,
}

impl DelimitedRecord {
    fn new(
        boundary: TriggerIdentifier,
        syntax: &EncodedTypeId<BootstrapLanguage>,
    ) -> Result<Self, structural_codec::AuthoringError> {
        let items = Position::try_new(SharedDescriptor::Repeated {
            minimum: 0,
            maximum: None,
            element: Box::new(delegate(syntax)),
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

struct DelimitedRecordView<'a>(&'a DelimitedRecord);

impl BorrowedFieldView<BootstrapLanguage> for DelimitedRecordView<'_> {
    fn expose<Visitor: FieldVisitor<BootstrapLanguage>>(&self, visitor: &mut Visitor) {
        visitor.field(&self.0.root);
        visitor.field(&self.0.items);
    }
}

impl StructureRecord<BootstrapLanguage> for DelimitedRecord {
    type View<'record> = DelimitedRecordView<'record>;

    fn root_role(&self) -> StableRoleId {
        self.root.role()
    }

    fn fields(&self) -> Self::View<'_> {
        DelimitedRecordView(self)
    }
}

type BootstrapRule = RuleCoproduct<
    DocumentRecord,
    RuleCoproduct<DelimitedRecord, StructuralRule<BootstrapLanguage>>,
>;

fn document_rule(rule: DocumentRecord) -> BootstrapRule {
    RuleCoproduct::Left(rule)
}

fn delimited_rule(rule: DelimitedRecord) -> BootstrapRule {
    RuleCoproduct::Right(RuleCoproduct::Left(rule))
}

fn structural_rule(rule: StructuralRule<BootstrapLanguage>) -> BootstrapRule {
    RuleCoproduct::Right(RuleCoproduct::Right(rule))
}

fn delegate(target: &EncodedTypeId<BootstrapLanguage>) -> SharedDescriptor<BootstrapLanguage> {
    SharedDescriptor::Delegate {
        target: target.clone(),
        payload: None,
    }
}

/// Every addressed structural identity required by the reader. Allocation is
/// external; the grammar derives no hierarchical child identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapGrammarIdentities {
    pub document: EncodedName,
    pub syntax: EncodedName,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Delimiter {
    Brace,
    Square,
    Parenthesis,
    Guillemets,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SyntaxNode {
    Atom {
        text: String,
        bound: SourceBound,
    },
    Application {
        head: String,
        head_bound: SourceBound,
        payload: Box<SyntaxNode>,
        bound: SourceBound,
    },
    AngleApplication {
        head: String,
        head_bound: SourceBound,
        arguments: Vec<SyntaxNode>,
        bound: SourceBound,
    },
    Delimited {
        delimiter: Delimiter,
        items: Vec<SyntaxNode>,
        bound: SourceBound,
    },
}

impl SyntaxNode {
    pub(crate) const fn bound(&self) -> SourceBound {
        match self {
            Self::Atom { bound, .. }
            | Self::Application { bound, .. }
            | Self::AngleApplication { bound, .. }
            | Self::Delimited { bound, .. } => *bound,
        }
    }

    pub(crate) const fn kind_name(&self) -> &'static str {
        match self {
            Self::Atom { .. } => "atom",
            Self::Application { .. } => "application",
            Self::AngleApplication { .. } => "Shape application",
            Self::Delimited { delimiter, .. } => match delimiter {
                Delimiter::Brace => "brace product",
                Delimiter::Square => "square vector",
                Delimiter::Parenthesis => "parenthesized product",
                Delimiter::Guillemets => "Trait requirement",
            },
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct StructuralDocumentPlan {
    pub(crate) roots: Vec<SyntaxNode>,
}

/// The single shared structural evaluator used for all three body roots.
#[derive(Clone, Debug)]
pub(crate) struct BootstrapGrammar {
    document: EncodedTypeId<BootstrapLanguage>,
    constructors: SyntaxConstructors,
    table: AddressedStructuralTable<BootstrapLanguage, BootstrapRule>,
}

#[derive(Clone, Debug)]
struct SyntaxConstructors {
    atom: EncodedConstructorId<BootstrapLanguage>,
    application: EncodedConstructorId<BootstrapLanguage>,
    angle: EncodedConstructorId<BootstrapLanguage>,
    brace: EncodedConstructorId<BootstrapLanguage>,
    square: EncodedConstructorId<BootstrapLanguage>,
    parenthesis: EncodedConstructorId<BootstrapLanguage>,
    guillemet: EncodedConstructorId<BootstrapLanguage>,
}

impl BootstrapGrammar {
    pub(crate) fn build(
        identities: BootstrapGrammarIdentities,
    ) -> Result<Self, BootstrapBuildError> {
        if identities.document == identities.syntax {
            return Err(BootstrapBuildError::DuplicateGrammarIdentity);
        }
        let profile = bootstrap_profile()?;
        let document = EncodedTypeId::new(identities.document);
        let syntax = EncodedTypeId::new(identities.syntax);
        let constructors = SyntaxConstructors {
            atom: EncodedConstructorId::under(&syntax, 0),
            application: EncodedConstructorId::under(&syntax, 1),
            angle: EncodedConstructorId::under(&syntax, 2),
            brace: EncodedConstructorId::under(&syntax, 3),
            square: EncodedConstructorId::under(&syntax, 4),
            parenthesis: EncodedConstructorId::under(&syntax, 5),
            guillemet: EncodedConstructorId::under(&syntax, 6),
        };

        let document_entry = typed_entry(
            document.clone(),
            vec![(0, document_rule(DocumentRecord::new(&syntax)?))],
        );
        let syntax_entry = typed_entry(
            syntax.clone(),
            vec![
                (
                    0,
                    structural_rule(StructuralRule::Unary(UnaryRule::new(
                        SharedDescriptor::Reference(AtomDescriptor::any_case()),
                    )?)),
                ),
                (
                    1,
                    structural_rule(StructuralRule::Application(ApplicationRule::new(
                        APPLICATION,
                        SharedDescriptor::Reference(AtomDescriptor::any_case()),
                        delegate(&syntax),
                    )?)),
                ),
                (
                    2,
                    structural_rule(StructuralRule::AdjacentApplicationDelimited(
                        AdjacentApplicationDelimitedRule::new(
                            ANGLE,
                            SharedDescriptor::Reference(AtomDescriptor::any_case()),
                            delegate(&syntax),
                            1,
                            None,
                        )?,
                    )),
                ),
                (3, delimited_rule(DelimitedRecord::new(BRACE, &syntax)?)),
                (4, delimited_rule(DelimitedRecord::new(SQUARE, &syntax)?)),
                (
                    5,
                    delimited_rule(DelimitedRecord::new(PARENTHESIS, &syntax)?),
                ),
                (6, delimited_rule(DelimitedRecord::new(GUILLEMET, &syntax)?)),
            ],
        );
        let table = AddressedStructuralTable::seal(
            TableIdentityPayload::new(
                TargetLayoutIdentity::derive(b"core-ethos bootstrap structural tree v1"),
                profile.identity(),
                StructuralVocabularyIdentity::language(b"core-ethos bootstrap syntax v1"),
                bootstrap_discovery(),
                TextualRenderingPolicy::new(vec![
                    ContextualTextualPolicy::new(
                        ROOT_CONTEXT,
                        Some(WHITESPACE),
                        Some(TEXT_CARRIER),
                    ),
                    ContextualTextualPolicy::new(
                        CHILD_CONTEXT,
                        Some(WHITESPACE),
                        Some(TEXT_CARRIER),
                    ),
                ]),
                vec![document_entry, syntax_entry],
            ),
            &profile,
        )
        .map_err(|error| BootstrapBuildError::Table(Box::new(error)))?;
        Ok(Self {
            document,
            constructors,
            table,
        })
    }

    pub(crate) fn plan(&self, source: &str) -> Result<StructuralDocumentPlan, BootstrapReadError> {
        let evaluator = StructuralEvaluator::new(&self.table)?;
        let plan = StructuralPlanning::plan_text(&evaluator, &self.document, source, &NoNames)?;
        let roots = [
            self.syntax_from_field(field::<HeaderSyntaxRole>(&plan, "header")?)?,
            self.syntax_from_field(field::<ImportsSyntaxRole>(&plan, "imports")?)?,
            self.syntax_from_field(field::<BodySyntaxRole>(&plan, "body")?)?,
        ]
        .into_iter()
        .collect();
        Ok(StructuralDocumentPlan { roots })
    }

    fn syntax_from_field(
        &self,
        planned_field: &PlannedFieldValue<BootstrapLanguage>,
    ) -> Result<SyntaxNode, BootstrapReadError> {
        let value = delegated(planned_field, "syntax node")?;
        let bound = value
            .field_bound::<UnaryRoot>()
            .or_else(|| value.field_bound::<ApplicationRoot>())
            .or_else(|| value.field_bound::<AdjacentApplicationDelimitedRoot>())
            .or_else(|| value.field_bound::<DelimitedRootRole>())
            .unwrap_or_else(|| SourceBound::whole(""));
        if value.constructor() == &self.constructors.atom {
            let (text, atom_bound) = scalar_text::<UnaryRoot>(value, "atom")?;
            return Ok(SyntaxNode::Atom {
                text: text.to_owned(),
                bound: atom_bound,
            });
        }
        if value.constructor() == &self.constructors.application {
            let (head, head_bound) = scalar_text::<ApplicationHead>(value, "application head")?;
            let payload =
                self.syntax_from_field(field::<ApplicationPayload>(value, "application payload")?)?;
            return Ok(SyntaxNode::Application {
                head: head.to_owned(),
                head_bound,
                payload: Box::new(payload),
                bound,
            });
        }
        if value.constructor() == &self.constructors.angle {
            let (head, head_bound) =
                scalar_text::<AdjacentApplicationDelimitedHead>(value, "Shape head")?;
            let arguments =
                repeated::<AdjacentApplicationDelimitedItems>(value, "Shape arguments")?
                    .iter()
                    .map(|field| self.syntax_from_field(field))
                    .collect::<Result<Vec<_>, _>>()?;
            return Ok(SyntaxNode::AngleApplication {
                head: head.to_owned(),
                head_bound,
                arguments,
                bound,
            });
        }
        let delimiter = if value.constructor() == &self.constructors.brace {
            Delimiter::Brace
        } else if value.constructor() == &self.constructors.square {
            Delimiter::Square
        } else if value.constructor() == &self.constructors.parenthesis {
            Delimiter::Parenthesis
        } else if value.constructor() == &self.constructors.guillemet {
            Delimiter::Guillemets
        } else {
            return Err(unexpected(value, "known syntax constructor"));
        };
        let items = repeated::<DelimitedItemsRole>(value, "delimited items")?
            .iter()
            .map(|field| self.syntax_from_field(field))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SyntaxNode::Delimited {
            delimiter,
            items,
            bound,
        })
    }
}

fn typed_entry(
    type_id: EncodedTypeId<BootstrapLanguage>,
    rules: Vec<(u16, BootstrapRule)>,
) -> StructuralEntry<BootstrapLanguage, BootstrapRule> {
    StructuralEntry::new(
        type_id.clone(),
        rules
            .into_iter()
            .map(|(local, rule)| {
                ConstructorCodec::new(
                    EncodedConstructorId::under(&type_id, local),
                    vec![AcceptedDecodeForm::new(DecodeFormId::new(0), rule.clone())],
                    rule,
                )
            })
            .collect(),
    )
}

fn bootstrap_profile() -> Result<SealedTokenProfile, raw_discovery::TokenProfileError> {
    let definitions = vec![
        boundary(0, "(", ")"),
        boundary(1, "[", "]"),
        boundary(2, "{", "}"),
        TriggerDefinition {
            identifier: APPLICATION,
            trigger: Trigger::Application {
                glyph: ".".to_owned(),
            },
        },
        TriggerDefinition {
            identifier: TEXT_CARRIER,
            trigger: Trigger::CurlyText,
        },
        TriggerDefinition {
            identifier: WHITESPACE,
            trigger: Trigger::Whitespace {
                canonical_spelling: " ".to_owned(),
            },
        },
        TriggerDefinition {
            identifier: TriggerIdentifier::new(6),
            trigger: Trigger::LineComment {
                opening: ";;".to_owned(),
            },
        },
        boundary(7, "<", ">"),
        boundary(8, "«", "»"),
    ];
    TokenProfileData::new(
        ProfileRevision::new(3),
        definitions,
        TriggerSet::new((0..=8).map(TriggerIdentifier::new).collect()),
        CharacterSet::from_text("\"“”<>|$«»"),
    )
    .seal()
}

fn boundary(identifier: u16, opening: &str, closing: &str) -> TriggerDefinition {
    TriggerDefinition {
        identifier: TriggerIdentifier::new(identifier),
        trigger: Trigger::Boundary {
            opening: opening.to_owned(),
            closing: closing.to_owned(),
        },
    }
}

fn bootstrap_discovery() -> BlockTreeDiscoveryConfiguration {
    let active = TriggerSet::new(vec![
        PARENTHESIS,
        SQUARE,
        BRACE,
        TEXT_CARRIER,
        WHITESPACE,
        ANGLE,
        GUILLEMET,
    ]);
    let mut transitions = Vec::new();
    for context in [ROOT_CONTEXT, CHILD_CONTEXT] {
        for boundary in [PARENTHESIS, SQUARE, BRACE, ANGLE, GUILLEMET] {
            transitions.push(BoundaryDiscoveryTransition::new(
                context,
                boundary,
                CHILD_CONTEXT,
            ));
        }
    }
    BlockTreeDiscoveryConfiguration::new(
        BoundaryDiscoveryConfiguration::new(
            ROOT_CONTEXT,
            vec![
                BoundaryDiscoveryContext::new(ROOT_CONTEXT, active.clone()),
                BoundaryDiscoveryContext::new(CHILD_CONTEXT, active),
            ],
            transitions,
        ),
        vec![],
    )
}

struct NoNames;

impl EncodedNameResolver for NoNames {
    fn resolve(&self, _encoded_name: &EncodedName) -> Option<&TextualName> {
        None
    }
}

fn field<'a, Role: FieldRole>(
    value: &'a PlannedStructuralValue<BootstrapLanguage>,
    expected: &'static str,
) -> Result<&'a PlannedFieldValue<BootstrapLanguage>, BootstrapReadError> {
    value
        .field::<Role>()
        .ok_or_else(|| unexpected(value, expected))
}

fn delegated<'a>(
    field: &'a PlannedFieldValue<BootstrapLanguage>,
    expected: &'static str,
) -> Result<&'a PlannedStructuralValue<BootstrapLanguage>, BootstrapReadError> {
    match field {
        PlannedFieldValue::Delegated(value) => Ok(value),
        _ => Err(BootstrapReadError::UnexpectedStructure {
            expected,
            found: "non-delegated field",
            start: 0,
        }),
    }
}

fn repeated<'a, Role: FieldRole>(
    value: &'a PlannedStructuralValue<BootstrapLanguage>,
    expected: &'static str,
) -> Result<&'a [PlannedFieldValue<BootstrapLanguage>], BootstrapReadError> {
    match field::<Role>(value, expected)? {
        PlannedFieldValue::Repeated(items) => Ok(items),
        _ => Err(unexpected(value, expected)),
    }
}

fn scalar_text<'a, Role: FieldRole>(
    value: &'a PlannedStructuralValue<BootstrapLanguage>,
    expected: &'static str,
) -> Result<(&'a str, SourceBound), BootstrapReadError> {
    match field::<Role>(value, expected)? {
        PlannedFieldValue::Reference(name) => Ok((
            name.spelling().as_str(),
            value
                .field_bound::<Role>()
                .unwrap_or_else(|| SourceBound::whole("")),
        )),
        _ => Err(unexpected(value, expected)),
    }
}

fn unexpected(
    value: &PlannedStructuralValue<BootstrapLanguage>,
    expected: &'static str,
) -> BootstrapReadError {
    let start = value
        .field_bound::<UnaryRoot>()
        .or_else(|| value.field_bound::<ApplicationRoot>())
        .or_else(|| value.field_bound::<AdjacentApplicationDelimitedRoot>())
        .or_else(|| value.field_bound::<DelimitedRootRole>())
        .map_or(0, SourceBound::start);
    BootstrapReadError::UnexpectedStructure {
        expected,
        found: "structural field",
        start,
    }
}
