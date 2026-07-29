//! The first complete six-slot Ethos decode over translator-issued identities.

use std::cell::RefCell;
use std::collections::BTreeMap;

use core_ethos::{
    DecodedEncodedIdPosition, SixSlotDecodeError, SixSlotEthosCodec, SixSlotGrammarError,
    SixSlotGrammarIdPosition, SixSlotGrammarIds, SliceOneBuiltinPriorError,
    SliceOneBuiltinPriorPosition, SliceOneBuiltinPriors, WholeEthos, WholeEthosItem,
    WholeEthosTypeReference, WholeEthosVariantPayload, WholeEthosVisibility,
};
use encoded_name_table::Name;
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use slice_structural_codec::{
    DeclarationAssignment, DecodeError, DecodeNameBindings, EncodedNameResolver, LocalEncodedId,
    NameOccurrence, ResolvedReference,
};

const SOURCE: &str = "  {}\n []  []\n {CommitSequence.Integer}\n {}\t{}  ";
const BREADTH_SOURCE: &str = "{} [] [] {Identifiers.Vector.Integer Status.{Pending Ready.{Integer} Batch.{Vector.Integer Integer}}} {} {}";

fn encoded(root: VocabularyRoot, chain: &[u16]) -> VocabularyEncodedId {
    VocabularyEncodedId::new(
        root,
        chain.iter().copied().map(LocalEncodedId::new).collect(),
    )
    .expect("test IDs are complete")
}

fn grammar_ids() -> SixSlotGrammarIds {
    SixSlotGrammarIds::new(
        encoded(VocabularyRoot::Universal, &[200]),
        encoded(VocabularyRoot::Universal, &[201]),
        encoded(VocabularyRoot::Universal, &[202]),
        encoded(VocabularyRoot::Universal, &[203]),
        encoded(VocabularyRoot::Universal, &[204]),
        encoded(VocabularyRoot::Universal, &[205]),
        encoded(VocabularyRoot::Universal, &[206]),
    )
    .expect("Universal grammar IDs")
}

fn builtin_priors() -> SliceOneBuiltinPriors {
    SliceOneBuiltinPriors::new(
        encoded(VocabularyRoot::Universal, &[3]),
        encoded(VocabularyRoot::Universal, &[4]),
    )
    .expect("Universal builtin priors")
}

fn codec() -> SixSlotEthosCodec {
    SixSlotEthosCodec::build(grammar_ids(), builtin_priors()).expect("typed six-slot table")
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
        let start = source.find(spelling).expect("declaration spelling");
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
fn six_typed_slots_decode_one_private_field_newtype_with_absolute_bounds() {
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

    let bounds = decoded.source_bounds();
    assert_eq!(
        &SOURCE[bounds.imports().start()..bounds.imports().end()],
        "{}"
    );
    assert_eq!(&SOURCE[bounds.input().start()..bounds.input().end()], "[]");
    assert_eq!(
        &SOURCE[bounds.output().start()..bounds.output().end()],
        "[]"
    );
    assert_eq!(
        &SOURCE[bounds.types().start()..bounds.types().end()],
        "{CommitSequence.Integer}"
    );
    assert_eq!(
        &SOURCE[bounds.generics().start()..bounds.generics().end()],
        "{}"
    );
    assert_eq!(&SOURCE[bounds.impls().start()..bounds.impls().end()], "{}");

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
        Err(SixSlotDecodeError::Structural(
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
fn arity_slot_and_newtype_shape_fail_structurally() {
    let codec = codec();
    let bindings = Bindings::empty();

    assert!(matches!(
        codec.decode("{} [] [] {} {}", &bindings),
        Err(SixSlotDecodeError::Structural(
            DecodeError::ProductArityMismatch {
                expected: 6,
                found: 5
            }
        ))
    ));
    assert!(matches!(
        codec.decode("[] [] [] {} {} {}", &bindings),
        Err(SixSlotDecodeError::Structural(
            DecodeError::ProductPositionMismatch { position: 0, .. }
        ))
    ));

    let malformed = "{} [] [] {CommitSequence.[Integer]} {} {}";
    let mut malformed_bindings = Bindings::empty();
    malformed_bindings.bind_declaration(
        "CommitSequence",
        encoded(VocabularyRoot::Universal, &[42, 7, 9]),
        malformed,
    );
    assert!(matches!(
        codec.decode(malformed, &malformed_bindings),
        Err(SixSlotDecodeError::Structural(
            DecodeError::ProductPositionMismatch { position: 3, .. }
        ))
    ));
}

#[test]
fn decoded_declaration_and_builtin_prior_must_both_be_universal() {
    let codec = codec();
    let (foreign_declaration, _, _) =
        complete_bindings(VocabularyRoot::Rust, VocabularyRoot::Universal);
    assert!(matches!(
        codec.decode(SOURCE, &foreign_declaration),
        Err(SixSlotDecodeError::NonUniversalEncodedId {
            position: DecodedEncodedIdPosition::Declaration,
            root: VocabularyRoot::Rust,
        })
    ));

    let (foreign_reference, _, _) =
        complete_bindings(VocabularyRoot::Universal, VocabularyRoot::Rust);
    assert!(matches!(
        codec.decode(SOURCE, &foreign_reference),
        Err(SixSlotDecodeError::NonUniversalEncodedId {
            position: DecodedEncodedIdPosition::Reference,
            root: VocabularyRoot::Rust,
        })
    ));
}

#[test]
fn integer_resolution_must_equal_the_registered_lookup_only_prior() {
    let expected = encoded(VocabularyRoot::Universal, &[3]);
    let found = encoded(VocabularyRoot::Universal, &[19, 4]);
    let codec = SixSlotEthosCodec::build(
        grammar_ids(),
        SliceOneBuiltinPriors::new(expected.clone(), encoded(VocabularyRoot::Universal, &[4]))
            .expect("Universal builtin priors"),
    )
    .expect("typed six-slot table");
    let declaration = encoded(VocabularyRoot::Universal, &[42, 7, 9]);
    let mut bindings = Bindings::empty();
    bindings.bind_declaration("CommitSequence", declaration, SOURCE);
    bindings.bind_reference("Integer", found.clone(), SOURCE);

    match codec.decode(SOURCE, &bindings) {
        Err(SixSlotDecodeError::BuiltinPriorMismatch {
            expected: rejected_expected,
            found: rejected_found,
        }) => {
            assert_eq!(rejected_expected, expected);
            assert_eq!(rejected_found, found);
        }
        result => panic!("expected exact builtin-prior refusal, got {result:?}"),
    }
}

#[test]
fn grammar_identities_are_supplied_and_universal() {
    let error = SixSlotGrammarIds::new(
        encoded(VocabularyRoot::Rust, &[200]),
        encoded(VocabularyRoot::Universal, &[201]),
        encoded(VocabularyRoot::Universal, &[202]),
        encoded(VocabularyRoot::Universal, &[203]),
        encoded(VocabularyRoot::Universal, &[204]),
        encoded(VocabularyRoot::Universal, &[205]),
        encoded(VocabularyRoot::Universal, &[206]),
    )
    .expect_err("a Rust-root document grammar identity is refused");
    assert!(matches!(
        error,
        SixSlotGrammarError::NonUniversal {
            position: SixSlotGrammarIdPosition::Document,
            root: VocabularyRoot::Rust,
        }
    ));
}

#[test]
fn builtin_prior_is_supplied_and_universal() {
    assert!(matches!(
        SliceOneBuiltinPriors::new(
            encoded(VocabularyRoot::Rust, &[3]),
            encoded(VocabularyRoot::Universal, &[4]),
        ),
        Err(SliceOneBuiltinPriorError::NonUniversal {
            position: SliceOneBuiltinPriorPosition::Integer,
            root: VocabularyRoot::Rust,
        })
    ));
}

#[test]
fn slice_path_has_no_positional_split_or_local_name_allocation_surface() {
    let source = include_str!("../src/slice_one.rs");
    for forbidden in [
        "NameInterner",
        ".intern(",
        "LocalEncodedId",
        "NameTable::new",
    ] {
        assert!(
            !source.contains(forbidden),
            "the slice path must not contain local allocation surface `{forbidden}`"
        );
    }
    for forbidden in ["DOCUMENT_SLOTS", "source.trim", "roots["] {
        assert!(
            !source.contains(forbidden),
            "the slice path must not contain the legacy positional splitter surface `{forbidden}`"
        );
    }
}
