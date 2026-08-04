//! Psyche-reviewed Spirit documents through the complete composite codec.

use std::collections::BTreeMap;

use core_ethos::{
    EmptyEnumerationVariants, EmptyOperatorPayload, EmptyStructFields, EmptyTraitMethods,
    EmptyTypeArguments, EthosCodec, EthosDecodeError, EthosGrammarIdentities, EthosGrammarIds,
    WholeEthos, WholeEthosArchiveError, WholeEthosAttributes, WholeEthosBody,
    WholeEthosBuiltinPriors, WholeEthosEnumeration, WholeEthosFileKind, WholeEthosHeader,
    WholeEthosItem, WholeEthosNexusBody, WholeEthosOperatorApplication, WholeEthosStruct,
    WholeEthosTrait, WholeEthosTypeApplication, WholeEthosVisibility,
};
use encoded_name_table::Name;
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::{
    DeclarationAssignment, DecodeNameBindings, EncodedNameResolver, LocalEncodedId, NameOccurrence,
    ResolvedReference,
};

const INTERFACE: &str = include_str!("fixtures/spirit/interface.ethos");
const NEXUS: &str = include_str!("fixtures/spirit/nexus.ethos");
const SEMA: &str = include_str!("fixtures/spirit/sema.ethos");

// This fixed vocabulary is the fixture naming authority. Its order is part of
// the test data; the codec receives assignments through DecodeNameBindings and
// never allocates or continues an identity itself.
const FIXTURE_TRANSLATOR_VOCABULARY: &[&str] = &[
    "Record",
    "Entry",
    "Observe",
    "Query",
    "Register",
    "ReferentRegistration",
    "Recorded",
    "RecordIdentifier",
    "Observed",
    "RecordSet",
    "Registered",
    "ReferentIdentifier",
    "GuardianRejection",
    "GuardianReason",
    "Explanation",
    "ReferentRejection",
    "Topic",
    "Text",
    "Topics",
    "Vector",
    "Description",
    "Kind",
    "Decision",
    "Principle",
    "Correction",
    "Clarification",
    "Constraint",
    "Magnitude",
    "Minimum",
    "VeryLow",
    "Low",
    "Medium",
    "High",
    "VeryHigh",
    "Maximum",
    "Referent",
    "Aliases",
    "Integer",
    "NotSpirit",
    "PrivateContent",
    "MeaningUnclear",
    "Duplicate",
    "ObserverFilter",
    "ObserverSubscription",
    "SubscriptionToken",
    "ObservationEvent",
    "Stream",
    "Observer",
    "IntentFilter",
    "IntentSubscription",
    "IntentEvent",
    "Intent",
    "AdmissionDecision",
    "Accepted",
    "Rejected",
    "GuardianDecision",
    "Admit",
    "Refuse",
    "SignalAdmission",
    "admit",
    "recordDecision",
    "Unit",
    "AgentGuardian",
    "guard",
    "guardReferent",
    "StoredRecord",
    "StoredReferent",
    "SourceSchemaVersion",
    "MigratedRecordCount",
    "MigratedReferentCount",
    "Migration",
    "records",
    "referents",
    "migrations",
    "Domain",
    "Result",
    "Ordered",
    "Error",
    "Name",
    "Transformer",
];

fn encoded(local: u16) -> VocabularyEncodedId {
    VocabularyEncodedId::new(VocabularyRoot::Universal, vec![LocalEncodedId::new(local)])
        .expect("fixture identity is complete")
}

fn grammar_ids() -> EthosGrammarIds {
    let grammar = EthosGrammarIdentities {
        interface_document: encoded(200),
        nexus_document: encoded(201),
        sema_document: encoded(202),
        header: encoded(203),
        imports: encoded(204),
        import_entry: encoded(205),
        interface_body: encoded(206),
        nexus_body: encoded(207),
        sema_body: encoded(208),
        newtype_list: encoded(209),
        struct_list: encoded(210),
        item_list: encoded(211),
        trait_list: encoded(212),
        table_list: encoded(213),
        newtype_declaration: encoded(214),
        struct_declaration: encoded(215),
        item: encoded(216),
        variant: encoded(217),
        type_reference: encoded(218),
        operator_payload: encoded(219),
        trait_declaration: encoded(220),
        method: encoded(221),
        table: encoded(222),
    };
    EthosGrammarIds::new(grammar).expect("Universal grammar identities")
}

struct FixtureBindings {
    by_spelling: BTreeMap<&'static str, VocabularyEncodedId>,
    spellings: BTreeMap<VocabularyEncodedId, Name>,
}

impl FixtureBindings {
    fn new() -> Self {
        let mut by_spelling = BTreeMap::new();
        let mut spellings = BTreeMap::new();
        let mut local = 1000_u16;
        for spelling in FIXTURE_TRANSLATOR_VOCABULARY {
            let identity = encoded(local);
            by_spelling.insert(*spelling, identity.clone());
            spellings.insert(identity, Name::new(*spelling));
            local = local
                .checked_add(1)
                .expect("fixture vocabulary identity fits u16");
        }
        Self {
            by_spelling,
            spellings,
        }
    }

    fn identity(&self, spelling: &str) -> VocabularyEncodedId {
        self.by_spelling
            .get(spelling)
            .unwrap_or_else(|| panic!("fixture vocabulary contains {spelling}"))
            .clone()
    }

    fn priors(&self) -> WholeEthosBuiltinPriors {
        let mut priors =
            WholeEthosBuiltinPriors::new(self.identity("Integer"), self.identity("Vector"))
                .expect("Universal builtins")
                .with_object_application_head(self.identity("Stream"))
                .expect("Universal Stream operator")
                .with_application_head(self.identity("Result"))
                .expect("Universal Result type head");
        for spelling in FIXTURE_TRANSLATOR_VOCABULARY {
            priors = priors
                .with_identity(self.identity(spelling))
                .expect("Universal fixture identity");
        }
        priors
    }
}

impl EncodedNameResolver<VocabularyRoot> for FixtureBindings {
    fn resolve(&self, encoded_id: &VocabularyEncodedId) -> Option<&Name> {
        self.spellings.get(encoded_id)
    }
}

impl DecodeNameBindings<VocabularyRoot> for FixtureBindings {
    fn declaration_assignment(
        &self,
        occurrence: NameOccurrence<'_>,
    ) -> Option<DeclarationAssignment<VocabularyRoot>> {
        self.by_spelling
            .get(occurrence.spelling())
            .cloned()
            .map(DeclarationAssignment::new)
    }

    fn reference_resolution(
        &self,
        occurrence: NameOccurrence<'_>,
    ) -> Option<ResolvedReference<VocabularyRoot>> {
        self.by_spelling
            .get(occurrence.spelling())
            .cloned()
            .map(ResolvedReference::new)
    }
}

fn codec(bindings: &FixtureBindings) -> EthosCodec {
    EthosCodec::build(grammar_ids(), bindings.priors()).expect("composite Ethos table")
}

struct GoldenFixture {
    source: &'static str,
    kind: WholeEthosFileKind,
    imports: &'static [(&'static str, &'static [&'static str])],
}

#[test]
fn all_three_spirit_goldens_decode_emit_and_redecode_identical_encoded_meaning() {
    let bindings = FixtureBindings::new();
    let codec = codec(&bindings);
    let fixtures = [
        GoldenFixture {
            source: INTERFACE,
            kind: WholeEthosFileKind::Interface,
            imports: &[],
        },
        GoldenFixture {
            source: NEXUS,
            kind: WholeEthosFileKind::Nexus,
            imports: &[(
                "interface",
                &["Entry", "Referent", "RecordSet", "GuardianReason"],
            )],
        },
        GoldenFixture {
            source: SEMA,
            kind: WholeEthosFileKind::Sema,
            imports: &[
                (
                    "interface",
                    &["Entry", "Referent", "Aliases", "RecordIdentifier"],
                ),
                ("signal-domain", &["Domain"]),
            ],
        },
    ];

    for fixture in fixtures {
        let decoded = codec
            .decode(fixture.source, &bindings)
            .expect("psyche-reviewed fixture decodes");
        assert_eq!(decoded.ethos().header().kind(), fixture.kind);
        assert_eq!(decoded.ethos().header().version(), 1);
        assert_eq!(decoded.imports().entries().len(), fixture.imports.len());
        for (actual, (source, names)) in decoded.imports().entries().iter().zip(fixture.imports) {
            assert_eq!(actual.source(), *source);
            assert_eq!(
                actual
                    .names()
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                *names
            );
        }

        let emitted = codec
            .encode(&decoded, &bindings)
            .expect("typed encoded meaning reflects through the shared writer");
        assert_ne!(
            emitted.as_bytes(),
            fixture.source.as_bytes(),
            "presentation layout is deliberately not retained"
        );
        let redecoded = codec
            .decode(&emitted, &bindings)
            .expect("canonical shared-writer output decodes");
        assert_eq!(redecoded, decoded);

        let archive = decoded.ethos().to_archive_bytes().expect("archive fixture");
        let redecoded_archive = redecoded
            .ethos()
            .to_archive_bytes()
            .expect("archive redecoded fixture");
        assert_eq!(redecoded_archive, archive);
        assert_eq!(
            WholeEthos::from_archive_bytes(&archive).expect("restore fixture"),
            *decoded.ethos()
        );
    }
}

#[test]
fn presentation_variants_decode_to_the_same_typed_document_and_imports() {
    let bindings = FixtureBindings::new();
    let codec = codec(&bindings);
    let reviewed = codec.decode(NEXUS, &bindings).expect("reviewed Nexus");
    let canonical = codec
        .encode(&reviewed, &bindings)
        .expect("canonical Nexus text");
    assert_ne!(canonical, NEXUS);
    assert_eq!(
        codec
            .decode(&canonical, &bindings)
            .expect("canonical Nexus"),
        reviewed
    );
}

#[test]
fn source_only_import_text_preserves_meaningful_whitespace_through_canonical_emission() {
    let bindings = FixtureBindings::new();
    let codec = codec(&bindings);
    let source = "Interface.1\n[“source module”.“Imported Name”]\n{[] [] [] []}\n";
    let decoded = codec
        .decode(source, &bindings)
        .expect("carried import text");
    assert_eq!(decoded.imports().entries()[0].source(), "source module");
    assert_eq!(decoded.imports().entries()[0].names(), &["Imported Name"]);

    let emitted = codec
        .encode(&decoded, &bindings)
        .expect("carried import text emits");
    assert!(emitted.contains("“source module”.“Imported Name”"));
    assert_eq!(
        codec.decode(&emitted, &bindings).expect("reemitted import"),
        decoded
    );
}

#[test]
fn type_references_require_strict_adjacent_angles_and_preserve_nested_shape() {
    let bindings = FixtureBindings::new();
    let codec = codec(&bindings);
    let source = "Interface.1\n[]\n{[Name.Result<Vector<Ordered> Error>] [] [] []}\n";
    let decoded = codec
        .decode(source, &bindings)
        .expect("strict nested angle type reference");
    let emitted = codec
        .encode(&decoded, &bindings)
        .expect("strict nested angle type reference emits");
    assert!(emitted.contains("Name.Result<Vector<Ordered> Error>"));
    assert_eq!(
        codec.decode(&emitted, &bindings).expect("reemitted type"),
        decoded
    );

    assert!(
        codec
            .decode(
                "Interface.1\n[]\n{[Name.Result.Vector.Ordered] [] [] []}\n",
                &bindings,
            )
            .is_err()
    );
    assert!(
        codec
            .decode(
                "Interface.1\n[]\n{[Name.Result<|Vector| Error>] [] [] []}\n",
                &bindings,
            )
            .is_err()
    );
    assert_eq!(
        WholeEthosTypeApplication::new(bindings.identity("Result"), vec![]),
        Err(EmptyTypeArguments)
    );
}

#[test]
fn public_constructors_reject_unsupported_headers_mismatched_bodies_and_empty_grammar_forms() {
    let name = encoded(900);
    assert!(matches!(
        WholeEthosHeader::new(WholeEthosFileKind::Interface, 2),
        Err(WholeEthosArchiveError::UnsupportedVersion {
            kind: WholeEthosFileKind::Interface,
            found: 2,
            supported: 1,
        })
    ));

    let header = WholeEthosHeader::new(WholeEthosFileKind::Interface, 1)
        .expect("supported Interface header");
    assert!(matches!(
        WholeEthos::new(
            header,
            WholeEthosBody::Nexus(WholeEthosNexusBody::new(vec![], vec![])),
        ),
        Err(WholeEthosArchiveError::HeaderBodyKindMismatch {
            header: WholeEthosFileKind::Interface,
            body: WholeEthosFileKind::Nexus,
        })
    ));

    assert_eq!(
        WholeEthosStruct::new(name.clone(), vec![]),
        Err(EmptyStructFields)
    );
    assert_eq!(
        WholeEthosEnumeration::new(
            name.clone(),
            WholeEthosVisibility::Public,
            WholeEthosAttributes::empty(),
            vec![],
        ),
        Err(EmptyEnumerationVariants)
    );
    assert_eq!(
        WholeEthosOperatorApplication::new(name.clone(), name.clone(), vec![]),
        Err(EmptyOperatorPayload)
    );
    assert_eq!(WholeEthosTrait::new(name, vec![]), Err(EmptyTraitMethods));
}

#[test]
fn fixtures_reify_every_kind_specific_body_position() {
    let bindings = FixtureBindings::new();
    let codec = codec(&bindings);

    let interface = codec.decode(INTERFACE, &bindings).expect("interface");
    let WholeEthosBody::Interface(body) = interface.ethos().body() else {
        panic!("Interface header selects Interface body")
    };
    assert_eq!(body.inputs().len(), 3);
    assert_eq!(body.outputs().len(), 3);
    assert_eq!(body.refusals().len(), 2);
    assert_eq!(body.types().len(), 23);
    assert_eq!(
        body.types()
            .iter()
            .filter(|item| matches!(item, WholeEthosItem::OperatorApplication(_)))
            .count(),
        2
    );

    let nexus = codec.decode(NEXUS, &bindings).expect("nexus");
    let WholeEthosBody::Nexus(body) = nexus.ethos().body() else {
        panic!("Nexus header selects Nexus body")
    };
    assert_eq!(body.types().len(), 2);
    assert_eq!(body.traits().len(), 2);
    assert_eq!(body.traits()[0].methods().len(), 2);
    assert!(body.traits()[0].methods()[1].parameters().len() == 1);

    let sema = codec.decode(SEMA, &bindings).expect("sema");
    let WholeEthosBody::Sema(body) = sema.ethos().body() else {
        panic!("Sema header selects Sema body")
    };
    assert_eq!(body.record_types().len(), 6);
    assert_eq!(body.tables().len(), 3);
}

#[test]
fn unknown_kind_and_version_mismatch_are_typed_refusals() {
    let bindings = FixtureBindings::new();
    let codec = codec(&bindings);

    assert!(matches!(
        codec.decode("Unknown.1\n[]\n{}\n", &bindings),
        Err(EthosDecodeError::UnknownFileKind { found }) if found == "Unknown"
    ));
    assert!(matches!(
        codec.decode("Interface.2\n[]\n{}\n", &bindings),
        Err(EthosDecodeError::UnsupportedVersion {
            kind: WholeEthosFileKind::Interface,
            found: 2,
            supported: 1,
        })
    ));
}

#[test]
fn whole_path_has_no_legacy_slots_or_local_name_allocation_surface() {
    let source = include_str!("../src/whole.rs");
    for forbidden in [
        "NameInterner",
        ".intern(",
        "LocalEncodedId",
        "NameTable::new",
        "DOCUMENT_SLOTS",
        "roots[",
        "SixSlot",
    ] {
        assert!(
            !source.contains(forbidden),
            "Whole-Ethos must not contain forbidden surface `{forbidden}`"
        );
    }
}
