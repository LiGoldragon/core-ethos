//! Types-only Ethos round trips over translator-issued identities.

use std::cell::RefCell;
use std::collections::BTreeMap;

use core_ethos::{
    DecodedEncodedIdPosition, EthosCodec, EthosDecodeError, EthosGrammarError,
    EthosGrammarIdPosition, EthosGrammarIds, WholeEthos, WholeEthosBuiltinPriorError,
    WholeEthosBuiltinPriorPosition, WholeEthosBuiltinPriors, WholeEthosItem,
    WholeEthosReferencePriorPosition, WholeEthosTypeReference, WholeEthosVariantPayload,
    WholeEthosVisibility,
};
use encoded_name_table::Name;
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::{
    DeclarationAssignment, DecodeError, DecodeNameBindings, EncodedNameResolver, LocalEncodedId,
    NameOccurrence, ResolvedReference,
};

// `[assumption primary-pjm-A1 — types-only root shape]`: every authored
// fixture is only the sole brace-delimited types position; the expected root
// identity selects the file kind without an authored tag or repeated empty slots.
const SOURCE: &str = "{CommitSequence.Integer}";
const BREADTH_SOURCE: &str =
    "{Identifiers.Vector.Integer Status.{Pending Ready.{Integer} Batch.{Vector.Integer Integer}}}";
const DOMAIN_FORMS_SOURCE: &str =
    "{Root.[All Leaf.LeafKind] LeafKind.[One Two] Collection.Vector.Root}";

fn encoded(root: VocabularyRoot, chain: &[u16]) -> VocabularyEncodedId {
    VocabularyEncodedId::new(
        root,
        chain.iter().copied().map(LocalEncodedId::new).collect(),
    )
    .expect("test IDs are complete")
}

fn grammar_ids() -> EthosGrammarIds {
    EthosGrammarIds::new(
        encoded(VocabularyRoot::Universal, &[200]),
        encoded(VocabularyRoot::Universal, &[201]),
        encoded(VocabularyRoot::Universal, &[202]),
        encoded(VocabularyRoot::Universal, &[203]),
        encoded(VocabularyRoot::Universal, &[204]),
    )
    .expect("Universal grammar IDs")
}

fn builtin_priors() -> WholeEthosBuiltinPriors {
    WholeEthosBuiltinPriors::new(
        encoded(VocabularyRoot::Universal, &[3]),
        encoded(VocabularyRoot::Universal, &[4]),
    )
    .expect("Universal builtin priors")
}

fn codec() -> EthosCodec {
    EthosCodec::build(grammar_ids(), builtin_priors()).expect("typed Ethos table")
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Query {
    Declaration {
        spelling: String,
        start: usize,
        end: usize,
    },
    Reference {
        spelling: String,
        start: usize,
        end: usize,
    },
}

struct Bindings {
    declarations: BTreeMap<(usize, usize), (String, VocabularyEncodedId)>,
    references: BTreeMap<(usize, usize), (String, VocabularyEncodedId)>,
    spellings: BTreeMap<VocabularyEncodedId, Name>,
    queries: RefCell<Vec<Query>>,
}

impl Bindings {
    fn empty() -> Self {
        Self {
            declarations: BTreeMap::new(),
            references: BTreeMap::new(),
            spellings: BTreeMap::new(),
            queries: RefCell::new(Vec::new()),
        }
    }

    fn bind_declaration(&mut self, spelling: &str, encoded_id: VocabularyEncodedId, source: &str) {
        self.bind_declaration_occurrence(spelling, 0, encoded_id, source);
    }

    fn bind_declaration_occurrence(
        &mut self,
        spelling: &str,
        occurrence: usize,
        encoded_id: VocabularyEncodedId,
        source: &str,
    ) {
        let start = source
            .match_indices(spelling)
            .nth(occurrence)
            .expect("declaration spelling")
            .0;
        let end = start + spelling.len();
        self.spellings
            .insert(encoded_id.clone(), Name::new(spelling));
        self.declarations
            .insert((start, end), (spelling.to_owned(), encoded_id));
    }

    fn bind_reference(&mut self, spelling: &str, encoded_id: VocabularyEncodedId, source: &str) {
        self.bind_reference_occurrence(spelling, 0, encoded_id, source);
    }

    fn bind_reference_occurrence(
        &mut self,
        spelling: &str,
        occurrence: usize,
        encoded_id: VocabularyEncodedId,
        source: &str,
    ) {
        let start = source
            .match_indices(spelling)
            .nth(occurrence)
            .expect("reference spelling")
            .0;
        let end = start + spelling.len();
        self.spellings
            .insert(encoded_id.clone(), Name::new(spelling));
        self.references
            .insert((start, end), (spelling.to_owned(), encoded_id));
    }
}

#[test]
fn square_enums_payload_variants_and_declared_type_references_decode_structurally() {
    let integer = encoded(VocabularyRoot::Universal, &[3]);
    let vector = encoded(VocabularyRoot::Universal, &[4]);
    let root = encoded(VocabularyRoot::Universal, &[42, 1]);
    let all = encoded(VocabularyRoot::Universal, &[42, 1, 1]);
    let leaf = encoded(VocabularyRoot::Universal, &[42, 1, 2]);
    let leaf_kind = encoded(VocabularyRoot::Universal, &[42, 2]);
    let one = encoded(VocabularyRoot::Universal, &[42, 2, 1]);
    let two = encoded(VocabularyRoot::Universal, &[42, 2, 2]);
    let collection = encoded(VocabularyRoot::Universal, &[42, 3]);
    let priors = WholeEthosBuiltinPriors::new(integer, vector.clone())
        .expect("Universal builtin priors")
        .with_identity(root.clone())
        .expect("Universal declared reference")
        .with_identity(leaf_kind.clone())
        .expect("Universal declared reference");
    let codec = EthosCodec::build(grammar_ids(), priors).expect("typed Ethos table");
    let mut bindings = Bindings::empty();
    for (spelling, occurrence, identity) in [
        ("Root", 0, root.clone()),
        ("All", 0, all.clone()),
        ("Leaf", 0, leaf.clone()),
        ("LeafKind", 1, leaf_kind.clone()),
        ("One", 0, one.clone()),
        ("Two", 0, two.clone()),
        ("Collection", 0, collection.clone()),
    ] {
        bindings.bind_declaration_occurrence(spelling, occurrence, identity, DOMAIN_FORMS_SOURCE);
    }
    bindings.bind_reference_occurrence("LeafKind", 0, leaf_kind.clone(), DOMAIN_FORMS_SOURCE);
    bindings.bind_reference_occurrence("Vector", 0, vector.clone(), DOMAIN_FORMS_SOURCE);
    bindings.bind_reference_occurrence("Root", 1, root.clone(), DOMAIN_FORMS_SOURCE);

    let decoded = codec
        .decode(DOMAIN_FORMS_SOURCE, &bindings)
        .expect("domain fixture forms");
    let [
        WholeEthosItem::Enumeration(root_enum),
        WholeEthosItem::Enumeration(leaf_enum),
        WholeEthosItem::Newtype(collection_newtype),
    ] = decoded.ethos().items()
    else {
        panic!("two square enums followed by one Vector newtype")
    };
    assert_eq!(root_enum.name(), &root);
    assert_eq!(root_enum.variants()[0].name(), &all);
    assert_eq!(
        root_enum.variants()[0].payload(),
        &WholeEthosVariantPayload::Unit
    );
    assert_eq!(root_enum.variants()[1].name(), &leaf);
    let WholeEthosVariantPayload::Tuple(fields) = root_enum.variants()[1].payload() else {
        panic!("compact payload variant has one positional field")
    };
    assert_eq!(
        fields.fields(),
        &[WholeEthosTypeReference::Identity(leaf_kind)]
    );
    assert_eq!(leaf_enum.variants().len(), 2);
    assert!(
        leaf_enum
            .variants()
            .iter()
            .all(|variant| variant.payload() == &WholeEthosVariantPayload::Unit)
    );
    assert_eq!(
        collection_newtype.wrapped_field().reference(),
        &WholeEthosTypeReference::Application(core_ethos::WholeEthosTypeApplication::new(
            vector,
            WholeEthosTypeReference::Identity(root),
        ))
    );
}

#[test]
fn enum_unit_tuple_and_application_references_decode_structurally() {
    let codec = codec();
    let identifiers = encoded(VocabularyRoot::Universal, &[42, 8]);
    let status = encoded(VocabularyRoot::Universal, &[42, 9]);
    let pending = encoded(VocabularyRoot::Universal, &[42, 9, 1]);
    let ready = encoded(VocabularyRoot::Universal, &[42, 9, 2]);
    let batch = encoded(VocabularyRoot::Universal, &[42, 9, 3]);
    let integer = encoded(VocabularyRoot::Universal, &[3]);
    let vector = encoded(VocabularyRoot::Universal, &[4]);
    let mut bindings = Bindings::empty();
    for (spelling, identity) in [
        ("Identifiers", identifiers.clone()),
        ("Status", status.clone()),
        ("Pending", pending.clone()),
        ("Ready", ready.clone()),
        ("Batch", batch.clone()),
    ] {
        bindings.bind_declaration(spelling, identity, BREADTH_SOURCE);
    }
    bindings.bind_reference_occurrence("Vector", 0, vector.clone(), BREADTH_SOURCE);
    bindings.bind_reference_occurrence("Vector", 1, vector.clone(), BREADTH_SOURCE);
    bindings.bind_reference_occurrence("Integer", 0, integer.clone(), BREADTH_SOURCE);
    bindings.bind_reference_occurrence("Integer", 1, integer.clone(), BREADTH_SOURCE);
    bindings.bind_reference_occurrence("Integer", 2, integer.clone(), BREADTH_SOURCE);
    bindings.bind_reference_occurrence("Integer", 3, integer.clone(), BREADTH_SOURCE);

    let decoded = codec
        .decode(BREADTH_SOURCE, &bindings)
        .expect("bounded fixture breadth");
    let [
        WholeEthosItem::Newtype(newtype),
        WholeEthosItem::Enumeration(enumeration),
    ] = decoded.ethos().items()
    else {
        panic!("one application newtype followed by one enumeration")
    };
    assert_eq!(newtype.name(), &identifiers);
    assert_eq!(
        newtype.wrapped_field().reference(),
        &WholeEthosTypeReference::Application(core_ethos::WholeEthosTypeApplication::new(
            vector.clone(),
            WholeEthosTypeReference::Identity(integer.clone()),
        ))
    );
    assert_eq!(enumeration.name(), &status);
    assert_eq!(enumeration.variants().len(), 3);
    assert_eq!(enumeration.variants()[0].name(), &pending);
    assert_eq!(
        enumeration.variants()[0].payload(),
        &WholeEthosVariantPayload::Unit
    );
    assert_eq!(enumeration.variants()[1].name(), &ready);
    let WholeEthosVariantPayload::Tuple(ready_fields) = enumeration.variants()[1].payload() else {
        panic!("Ready has a tuple payload")
    };
    assert_eq!(
        ready_fields.fields(),
        &[WholeEthosTypeReference::Identity(integer.clone())]
    );
    let WholeEthosVariantPayload::Tuple(batch_fields) = enumeration.variants()[2].payload() else {
        panic!("Batch has a tuple payload")
    };
    assert_eq!(
        batch_fields.fields(),
        &[
            WholeEthosTypeReference::Application(core_ethos::WholeEthosTypeApplication::new(
                vector,
                WholeEthosTypeReference::Identity(integer.clone()),
            )),
            WholeEthosTypeReference::Identity(integer),
        ]
    );

    let archive = decoded.ethos().to_archive_bytes().expect("archive breadth");
    assert_eq!(
        WholeEthos::from_archive_bytes(&archive).expect("restore breadth"),
        *decoded.ethos()
    );
}

impl EncodedNameResolver<VocabularyRoot> for Bindings {
    fn resolve(&self, encoded_id: &VocabularyEncodedId) -> Option<&Name> {
        self.spellings.get(encoded_id)
    }
}

impl DecodeNameBindings<VocabularyRoot> for Bindings {
    fn declaration_assignment(
        &self,
        occurrence: NameOccurrence<'_>,
    ) -> Option<DeclarationAssignment<VocabularyRoot>> {
        let bound = occurrence.bound();
        self.queries.borrow_mut().push(Query::Declaration {
            spelling: occurrence.spelling().to_owned(),
            start: bound.start(),
            end: bound.end(),
        });
        self.declarations
            .get(&(bound.start(), bound.end()))
            .filter(|(spelling, _)| spelling == occurrence.spelling())
            .map(|(_, encoded_id)| DeclarationAssignment::new(encoded_id.clone()))
    }

    fn reference_resolution(
        &self,
        occurrence: NameOccurrence<'_>,
    ) -> Option<ResolvedReference<VocabularyRoot>> {
        let bound = occurrence.bound();
        self.queries.borrow_mut().push(Query::Reference {
            spelling: occurrence.spelling().to_owned(),
            start: bound.start(),
            end: bound.end(),
        });
        self.references
            .get(&(bound.start(), bound.end()))
            .filter(|(spelling, _)| spelling == occurrence.spelling())
            .map(|(_, encoded_id)| ResolvedReference::new(encoded_id.clone()))
    }
}

fn complete_bindings(
    declaration_root: VocabularyRoot,
    reference_root: VocabularyRoot,
) -> (Bindings, VocabularyEncodedId, VocabularyEncodedId) {
    let declaration = encoded(declaration_root, &[42, 7, 9]);
    let integer = encoded(reference_root, &[3]);
    let mut bindings = Bindings::empty();
    bindings.bind_declaration("CommitSequence", declaration.clone(), SOURCE);
    bindings.bind_reference("Integer", integer.clone(), SOURCE);
    (bindings, declaration, integer)
}

#[test]
fn types_only_root_round_trips_one_private_field_newtype() {
    let codec = codec();
    let (bindings, declaration, integer) =
        complete_bindings(VocabularyRoot::Universal, VocabularyRoot::Universal);

    let decoded = codec.decode(SOURCE, &bindings).expect("complete document");
    let [WholeEthosItem::Newtype(newtype)] = decoded.ethos().items() else {
        panic!("one newtype item")
    };
    assert_eq!(newtype.name(), &declaration);
    assert_eq!(newtype.visibility(), &WholeEthosVisibility::Public);
    assert!(newtype.attributes().is_empty());
    assert_eq!(
        newtype.wrapped_field().visibility(),
        &WholeEthosVisibility::Private
    );
    assert_eq!(
        newtype.wrapped_field().reference(),
        &WholeEthosTypeReference::Identity(integer)
    );

    let types_source_bound = decoded.types_source_bound();
    assert_eq!(
        &SOURCE[types_source_bound.start()..types_source_bound.end()],
        SOURCE
    );

    let declaration_start = SOURCE.find("CommitSequence").unwrap();
    let reference_start = SOURCE.find("Integer").unwrap();
    assert_eq!(
        bindings.queries.borrow().as_slice(),
        [
            Query::Declaration {
                spelling: "CommitSequence".to_owned(),
                start: declaration_start,
                end: declaration_start + "CommitSequence".len(),
            },
            Query::Reference {
                spelling: "Integer".to_owned(),
                start: reference_start,
                end: reference_start + "Integer".len(),
            },
        ]
    );

    let bytes = decoded.ethos().to_archive_bytes().expect("archive");
    assert_eq!(
        WholeEthos::from_archive_bytes(&bytes).expect("validated restore"),
        *decoded.ethos()
    );

    // `[assumption primary-pjm-A2 — decoded-mirror round trip]`: the runtime
    // structural mirror retained beside WholeEthos is the encoded witness used
    // for canonical rendering in this slice.
    // `[assumption primary-pjm-A3 — neutral API naming]`: this witness uses the
    // arity-neutral EthosCodec API, not a parallel per-file-kind parser.
    let canonical = codec
        .encode(&decoded, &bindings)
        .expect("render through the same structural evaluator");
    assert_eq!(canonical, SOURCE);
    let round_trip = codec
        .decode(&canonical, &bindings)
        .expect("decode the canonical types-only file again");
    assert_eq!(round_trip, decoded);
}

#[test]
fn unresolved_integer_is_lookup_only_and_changes_no_binding_state() {
    let codec = codec();
    let declaration = encoded(VocabularyRoot::Universal, &[42, 7, 9]);
    let mut bindings = Bindings::empty();
    bindings.bind_declaration("CommitSequence", declaration, SOURCE);
    let declarations_before = bindings.declarations.clone();
    let references_before = bindings.references.clone();
    let reference_start = SOURCE.find("Integer").unwrap();

    assert!(matches!(
        codec.decode(SOURCE, &bindings),
        Err(EthosDecodeError::Structural(
            DecodeError::UnresolvedReference { bound }
        )) if bound.start() == reference_start
            && bound.end() == reference_start + "Integer".len()
    ));
    assert_eq!(bindings.declarations, declarations_before);
    assert_eq!(bindings.references, references_before);
    assert_eq!(
        bindings
            .queries
            .borrow()
            .iter()
            .filter(|query| matches!(query, Query::Reference { .. }))
            .count(),
        1
    );
}

#[test]
fn root_arity_position_and_empty_enumeration_shape_fail_structurally() {
    let codec = codec();
    let bindings = Bindings::empty();

    assert!(matches!(
        codec.decode("{} {}", &bindings),
        Err(EthosDecodeError::Structural(
            DecodeError::ProductArityMismatch {
                expected: 1,
                found: 2
            }
        ))
    ));
    assert!(matches!(
        codec.decode("[]", &bindings),
        Err(EthosDecodeError::Structural(
            DecodeError::ProductPositionMismatch { position: 0, .. }
        ))
    ));

    let malformed = "{CommitSequence.[]}";
    let mut malformed_bindings = Bindings::empty();
    malformed_bindings.bind_declaration(
        "CommitSequence",
        encoded(VocabularyRoot::Universal, &[42, 7, 9]),
        malformed,
    );
    assert!(matches!(
        codec.decode(malformed, &malformed_bindings),
        Err(EthosDecodeError::Structural(_))
    ));
}

#[test]
fn decoded_declaration_and_builtin_prior_must_both_be_universal() {
    let codec = codec();
    let (foreign_declaration, _, _) =
        complete_bindings(VocabularyRoot::Rust, VocabularyRoot::Universal);
    assert!(matches!(
        codec.decode(SOURCE, &foreign_declaration),
        Err(EthosDecodeError::NonUniversalEncodedId {
            position: DecodedEncodedIdPosition::Declaration,
            root: VocabularyRoot::Rust,
        })
    ));

    let (foreign_reference, _, _) =
        complete_bindings(VocabularyRoot::Universal, VocabularyRoot::Rust);
    assert!(matches!(
        codec.decode(SOURCE, &foreign_reference),
        Err(EthosDecodeError::NonUniversalEncodedId {
            position: DecodedEncodedIdPosition::Reference,
            root: VocabularyRoot::Rust,
        })
    ));
}

#[test]
fn identity_resolution_must_belong_to_the_registered_lookup_only_priors() {
    let integer = encoded(VocabularyRoot::Universal, &[3]);
    let found = encoded(VocabularyRoot::Universal, &[19, 4]);
    let codec = EthosCodec::build(
        grammar_ids(),
        WholeEthosBuiltinPriors::new(integer, encoded(VocabularyRoot::Universal, &[4]))
            .expect("Universal builtin priors"),
    )
    .expect("typed Ethos table");
    let declaration = encoded(VocabularyRoot::Universal, &[42, 7, 9]);
    let mut bindings = Bindings::empty();
    bindings.bind_declaration("CommitSequence", declaration, SOURCE);
    bindings.bind_reference("Integer", found.clone(), SOURCE);

    match codec.decode(SOURCE, &bindings) {
        Err(EthosDecodeError::UnregisteredReferencePrior {
            position: WholeEthosReferencePriorPosition::Identity,
            found: rejected_found,
        }) => {
            assert_eq!(rejected_found, found);
        }
        result => panic!("expected exact builtin-prior refusal, got {result:?}"),
    }
}

#[test]
fn grammar_identities_are_supplied_and_universal() {
    let error = EthosGrammarIds::new(
        encoded(VocabularyRoot::Rust, &[200]),
        encoded(VocabularyRoot::Universal, &[201]),
        encoded(VocabularyRoot::Universal, &[202]),
        encoded(VocabularyRoot::Universal, &[203]),
        encoded(VocabularyRoot::Universal, &[204]),
    )
    .expect_err("a Rust-root grammar identity is refused");
    assert!(matches!(
        error,
        EthosGrammarError::NonUniversal {
            position: EthosGrammarIdPosition::Root,
            root: VocabularyRoot::Rust,
        }
    ));
}

#[test]
fn builtin_prior_is_supplied_and_universal() {
    assert!(matches!(
        WholeEthosBuiltinPriors::new(
            encoded(VocabularyRoot::Rust, &[3]),
            encoded(VocabularyRoot::Universal, &[4]),
        ),
        Err(WholeEthosBuiltinPriorError::NonUniversal {
            position: WholeEthosBuiltinPriorPosition::Integer,
            root: VocabularyRoot::Rust,
        })
    ));
}

#[test]
fn whole_path_has_no_positional_split_or_local_name_allocation_surface() {
    let source = include_str!("../src/whole.rs");
    for forbidden in [
        "NameInterner",
        ".intern(",
        "LocalEncodedId",
        "NameTable::new",
    ] {
        assert!(
            !source.contains(forbidden),
            "the whole path must not contain local allocation surface `{forbidden}`"
        );
    }
    for forbidden in ["DOCUMENT_SLOTS", "source.trim", "roots["] {
        assert!(
            !source.contains(forbidden),
            "the whole path must not contain the legacy positional splitter surface `{forbidden}`"
        );
    }
}
