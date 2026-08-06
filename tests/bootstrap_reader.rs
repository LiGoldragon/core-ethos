use core_ethos::bootstrap::*;
use encoded_name_table::LocalEncodedId;
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};

fn id(local: u16) -> VocabularyEncodedId {
    VocabularyEncodedId::new(VocabularyRoot::Universal, vec![LocalEncodedId::new(local)])
        .expect("nonempty test identity")
}

fn rust_id(local: u16) -> VocabularyEncodedId {
    VocabularyEncodedId::new(VocabularyRoot::Rust, vec![LocalEncodedId::new(local)])
        .expect("nonempty test identity")
}

#[derive(Clone)]
struct Labels {
    interface: &'static str,
    nexus: &'static str,
    sema: &'static str,
    string: &'static str,
    integer: &'static str,
    boolean: &'static str,
    unit: &'static str,
    vector: &'static str,
    option: &'static str,
    map: &'static str,
    result: &'static str,
    stream: &'static str,
    stream_identity: &'static str,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            interface: "Interface",
            nexus: "Nexus",
            sema: "Sema",
            string: "String",
            integer: "Integer",
            boolean: "Boolean",
            unit: "Unit",
            vector: "Vector",
            option: "Option",
            map: "Map",
            result: "Result",
            stream: "Stream",
            stream_identity: "StreamIdentity",
        }
    }
}

struct Fixture {
    reader: BootstrapReader,
    base_snapshot: TextualMetadataSnapshot,
    base_schemas: IdentitySchemaCatalog,
}

fn prior_identities() -> BootstrapPriorIdentities {
    BootstrapPriorIdentities {
        interface_kind: id(1),
        nexus_kind: id(2),
        sema_kind: id(3),
        input_role: id(4),
        output_role: id(5),
        refusal_role: id(6),
        string_type: id(7),
        integer_type: id(8),
        boolean_type: id(9),
        unit_type: id(10),
        vector_shape: id(11),
        option_shape: id(12),
        map_shape: id(13),
        result_shape: id(14),
        stream_nomos: id(15),
        stream_shape: id(15),
        stream_identity_shape: id(16),
    }
}

fn make_fixture(labels: Labels, extra: Vec<(TextualMetadataRecord, IdentitySchema)>) -> Fixture {
    let prior_entries = vec![
        (
            1,
            labels.interface,
            vec![SchemaRole::FileKind(EthosKind::Interface)],
        ),
        (
            2,
            labels.nexus,
            vec![SchemaRole::FileKind(EthosKind::Nexus)],
        ),
        (3, labels.sema, vec![SchemaRole::FileKind(EthosKind::Sema)]),
        (
            4,
            "Input",
            vec![SchemaRole::InterfaceRole(InterfaceRole::Input)],
        ),
        (
            5,
            "Output",
            vec![SchemaRole::InterfaceRole(InterfaceRole::Output)],
        ),
        (
            6,
            "Refusal",
            vec![SchemaRole::InterfaceRole(InterfaceRole::Refusal)],
        ),
        (
            7,
            labels.string,
            vec![SchemaRole::Nominal { persistent: true }],
        ),
        (
            8,
            labels.integer,
            vec![SchemaRole::Nominal { persistent: true }],
        ),
        (
            9,
            labels.boolean,
            vec![SchemaRole::Nominal { persistent: true }],
        ),
        (
            10,
            labels.unit,
            vec![SchemaRole::Nominal { persistent: true }],
        ),
        (11, labels.vector, vec![SchemaRole::Shape { arity: 1 }]),
        (12, labels.option, vec![SchemaRole::Shape { arity: 1 }]),
        (13, labels.map, vec![SchemaRole::Shape { arity: 2 }]),
        (14, labels.result, vec![SchemaRole::Shape { arity: 2 }]),
        (
            15,
            labels.stream,
            vec![
                SchemaRole::Shape { arity: 1 },
                SchemaRole::Nomos(NomosSchema::StreamInitiation { arity: 2 }),
            ],
        ),
        (
            16,
            labels.stream_identity,
            vec![SchemaRole::Shape { arity: 1 }],
        ),
    ];
    let mut records = Vec::new();
    let mut schemas = Vec::new();
    for (local, spelling, roles) in prior_entries {
        let identity = id(local);
        records.push(TextualMetadataRecord {
            module_path: vec!["builtin".into()],
            visible_name: spelling.into(),
            encoded_name: identity.clone(),
        });
        schemas.push(IdentitySchema::new(identity, roles).expect("prior schema"));
    }
    for (record, schema) in extra {
        records.push(record);
        schemas.push(schema);
    }
    let base_snapshot = TextualMetadataSnapshot::new(records).expect("base metadata");
    let schemas = IdentitySchemaCatalog::new(schemas).expect("schema catalog");
    let priors = BootstrapPriorVocabulary::new(prior_identities(), &schemas, &base_snapshot)
        .expect("typed priors");
    let catalog = BootstrapCatalog::new(
        vec!["app".into()],
        base_snapshot.clone(),
        schemas.clone(),
        priors,
        BootstrapVersionPolicy::exact(EthosVersion::new(1, 0, 0)),
    )
    .expect("reader catalog");
    let reader = BootstrapReader::build(
        BootstrapGrammarIdentities {
            document: id(900),
            syntax: id(901),
        },
        catalog,
    )
    .expect("shared reader");
    Fixture {
        reader,
        base_snapshot,
        base_schemas: schemas,
    }
}

struct SealInputs {
    assignments: NamingAssignments,
    generated: GeneratedStreamAssignments,
    snapshot: TextualMetadataSnapshot,
    authored_ids: Vec<VocabularyEncodedId>,
    generated_ids: Vec<(VocabularyEncodedId, VocabularyEncodedId)>,
}

fn inputs_for(plan: &BootstrapReadPlan, base: &TextualMetadataSnapshot) -> SealInputs {
    let mut records = base.records().to_vec();
    let mut authored_ids = Vec::new();
    let mut assignments = Vec::new();
    let mut generated = Vec::new();
    let mut generated_ids = Vec::new();
    let mut next_generated = 500u16;
    for (index, declaration) in plan.declarations().iter().enumerate() {
        let encoded_name = id(100 + index as u16);
        records.push(TextualMetadataRecord {
            module_path: vec!["app".into()],
            visible_name: declaration.spelling().into(),
            encoded_name: encoded_name.clone(),
        });
        assignments.push(NamingAssignment {
            occurrence: declaration.occurrence(),
            encoded_name: encoded_name.clone(),
        });
        authored_ids.push(encoded_name);
        if declaration.purpose() == DeclarationPurpose::StreamInitiation {
            let initiation = id(next_generated);
            let termination = id(next_generated + 1);
            next_generated += 2;
            records.extend([
                TextualMetadataRecord {
                    module_path: vec!["app".into()],
                    visible_name: format!("PreparedStart{index}"),
                    encoded_name: initiation.clone(),
                },
                TextualMetadataRecord {
                    module_path: vec!["app".into()],
                    visible_name: format!("PreparedStop{index}"),
                    encoded_name: termination.clone(),
                },
            ]);
            generated.push(GeneratedStreamAssignment {
                source: declaration.occurrence(),
                initiation: initiation.clone(),
                termination: termination.clone(),
            });
            generated_ids.push((initiation, termination));
        }
    }
    SealInputs {
        assignments: NamingAssignments::new(assignments).expect("authored assignments"),
        generated: GeneratedStreamAssignments::new(generated).expect("generated assignments"),
        snapshot: TextualMetadataSnapshot::new(records).expect("post-operation snapshot"),
        authored_ids,
        generated_ids,
    }
}

fn seal(
    fixture: &Fixture,
    source: &str,
) -> (BootstrapReadPlan, SealInputs, PreparedBootstrapTransaction) {
    let plan = fixture.reader.plan(source).expect("plan");
    let inputs = inputs_for(&plan, &fixture.base_snapshot);
    let transaction = fixture
        .reader
        .seal(
            &plan,
            &inputs.assignments,
            &inputs.generated,
            &inputs.snapshot,
        )
        .expect("seal");
    (plan, inputs, transaction)
}

fn reseal_with_same_ids(
    fixture: &Fixture,
    source: &str,
    authored_ids: &[VocabularyEncodedId],
    generated_ids: &[(VocabularyEncodedId, VocabularyEncodedId)],
    snapshot: &TextualMetadataSnapshot,
) -> PreparedBootstrapTransaction {
    let plan = fixture.reader.plan(source).expect("replan canonical text");
    assert_eq!(plan.declarations().len(), authored_ids.len());
    let assignments = NamingAssignments::new(
        plan.declarations()
            .iter()
            .zip(authored_ids)
            .map(|(declaration, identity)| NamingAssignment {
                occurrence: declaration.occurrence(),
                encoded_name: identity.clone(),
            })
            .collect(),
    )
    .expect("same authored identities");
    let generated = GeneratedStreamAssignments::new(
        plan.declarations()
            .iter()
            .filter(|declaration| declaration.purpose() == DeclarationPurpose::StreamInitiation)
            .zip(generated_ids)
            .map(
                |(declaration, (initiation, termination))| GeneratedStreamAssignment {
                    source: declaration.occurrence(),
                    initiation: initiation.clone(),
                    termination: termination.clone(),
                },
            )
            .collect(),
    )
    .expect("same generated identities");
    fixture
        .reader
        .seal(&plan, &assignments, &generated, snapshot)
        .expect("semantic reseal")
}

#[test]
fn interface_stream_is_authored_nomos_and_prepares_exact_atomic_relations() {
    let fixture = make_fixture(Labels::default(), vec![]);
    let source = include_str!("fixtures/bootstrap/interface.ethos");
    let (plan, inputs, transaction) = seal(&fixture, source);
    assert_eq!(plan.declarations().len(), 5);
    assert_eq!(
        plan.declarations()
            .iter()
            .filter(|item| item.purpose() == DeclarationPurpose::StreamInitiation)
            .count(),
        1
    );
    assert_eq!(transaction.generated_streams.len(), 1);
    let BootstrapBody::Interface(body) = &transaction.decoded.document.body else {
        panic!("Interface")
    };
    let Declaration::Nomos(NomosDeclaration::StreamInitiation(authored)) = &body.types[2] else {
        panic!("authored Stream Nomos")
    };
    let prepared = &transaction.generated_streams[0];
    assert_eq!(prepared.output.name, authored.name);
    assert_eq!(prepared.initiation.query, authored.query);
    assert_eq!(
        prepared.output.stream_of_event.arguments.as_slice(),
        std::slice::from_ref(&authored.event)
    );
    assert_eq!(prepared.termination.stream_handle, authored.name);
    assert_eq!(
        prepared
            .role_relations
            .iter()
            .map(|relation| relation.role)
            .collect::<Vec<_>>(),
        [
            InterfaceRole::Input,
            InterfaceRole::Output,
            InterfaceRole::Input
        ]
    );
    assert_eq!(
        transaction.archive_status(),
        BootstrapArchiveStatus::NotYetArchived
    );

    let canonical = fixture
        .reader
        .write(&transaction)
        .expect("canonical writer");
    let resealed = reseal_with_same_ids(
        &fixture,
        &canonical,
        &inputs.authored_ids,
        &inputs.generated_ids,
        &inputs.snapshot,
    );
    assert_eq!(resealed, transaction);
}

#[test]
fn textual_ambiguity_is_refused_before_schema_class_can_forge_a_choice() {
    let nominal = id(70);
    let trait_id = id(71);
    let extra = vec![
        (
            TextualMetadataRecord {
                module_path: vec!["dep".into()],
                visible_name: "Clash".into(),
                encoded_name: nominal.clone(),
            },
            IdentitySchema::new(nominal, [SchemaRole::Nominal { persistent: false }]).unwrap(),
        ),
        (
            TextualMetadataRecord {
                module_path: vec!["dep".into()],
                visible_name: "Clash".into(),
                encoded_name: trait_id.clone(),
            },
            IdentitySchema::new(trait_id, [SchemaRole::Trait]).unwrap(),
        ),
    ];
    let fixture = make_fixture(Labels::default(), extra);
    let error = fixture
        .reader
        .plan("Interface.{1 0 0}\n[dep.[Clash]]\n{[Clash] [] [] []}")
        .expect_err("path/name ambiguity is not class-filtered");
    assert!(matches!(
        error,
        BootstrapReadError::AmbiguousReference { .. }
    ));
}

#[test]
fn schema_and_prior_catalogs_enforce_role_family_data_and_every_typed_prior() {
    assert!(matches!(
        IdentitySchema::new(
            id(40),
            [
                SchemaRole::Shape { arity: 1 },
                SchemaRole::Shape { arity: 2 }
            ]
        ),
        Err(BootstrapReadError::ConflictingSchemaRoles { .. })
    ));
    let fixture = make_fixture(Labels::default(), vec![]);
    let mut wrong = prior_identities();
    wrong.string_type = id(11);
    assert!(matches!(
        BootstrapPriorVocabulary::new(wrong, &fixture.base_schemas, &fixture.base_snapshot),
        Err(BootstrapReadError::InvalidPriorRole {
            position: "string_type",
            ..
        })
    ));
    assert_eq!(
        fixture.reader.archive_status(),
        BootstrapArchiveStatus::NotYetArchived
    );
}

#[test]
fn all_prior_and_root_spellings_are_metadata_driven() {
    let labels = Labels {
        interface: "Contract",
        nexus: "Behavior",
        sema: "Storage",
        string: "Text",
        integer: "Whole",
        boolean: "Truth",
        unit: "Void",
        vector: "Sequence",
        option: "Maybe",
        map: "Dictionary",
        result: "Outcome",
        stream: "Flow",
        stream_identity: "FlowIdentity",
    };
    let fixture = make_fixture(labels, vec![]);
    let source =
        "Contract.{1 0 0}\n[]\n{[] [] [] [Thing.Text Observer.Flow.(Thing Sequence<Thing>)]}";
    let (_, _, transaction) = seal(&fixture, source);
    let canonical = fixture.reader.write(&transaction).expect("renamed writer");
    assert!(canonical.starts_with("Contract.{1 0 0}"));
    assert!(canonical.contains("Thing.Text"));
    assert!(canonical.contains("Observer.Flow.(Thing Sequence<Thing>)"));
    assert!(!canonical.contains("Interface"));
    assert!(!canonical.contains("String"));
    assert!(!canonical.contains("Stream"));
}

#[test]
fn assignments_must_be_universal_fresh_and_collision_free_across_both_channels() {
    let fixture = make_fixture(Labels::default(), vec![]);
    let plan = fixture
        .reader
        .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Thing.String]}")
        .unwrap();
    let assignment = |identity| {
        NamingAssignments::new(vec![NamingAssignment {
            occurrence: plan.declarations()[0].occurrence(),
            encoded_name: identity,
        }])
        .unwrap()
    };
    let empty_generated = GeneratedStreamAssignments::new(vec![]).unwrap();
    let mut records = fixture.base_snapshot.records().to_vec();
    records.push(TextualMetadataRecord {
        module_path: vec!["app".into()],
        visible_name: "Thing".into(),
        encoded_name: rust_id(40),
    });
    let rust_snapshot = TextualMetadataSnapshot::new(records).unwrap();
    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &assignment(rust_id(40)),
            &empty_generated,
            &rust_snapshot
        ),
        Err(BootstrapReadError::NonUniversalAssignment { .. })
    ));

    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &assignment(id(7)),
            &empty_generated,
            &fixture.base_snapshot
        ),
        Err(BootstrapReadError::AssignedIdentityCollision { .. })
            | Err(BootstrapReadError::MetadataProjectionMismatch { .. })
    ));

    let stream_plan = fixture
        .reader
        .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Observer.Stream.(String String)]}")
        .unwrap();
    let mut stream_inputs = inputs_for(&stream_plan, &fixture.base_snapshot);
    let source = stream_plan.declarations()[0].occurrence();
    stream_inputs.generated = GeneratedStreamAssignments::new(vec![GeneratedStreamAssignment {
        source,
        initiation: stream_inputs.authored_ids[0].clone(),
        termination: id(501),
    }])
    .unwrap();
    assert!(matches!(
        fixture.reader.seal(
            &stream_plan,
            &stream_inputs.assignments,
            &stream_inputs.generated,
            &stream_inputs.snapshot
        ),
        Err(BootstrapReadError::AssignedIdentityCollision { .. })
    ));

    let external = id(70);
    let fixture_with_import = make_fixture(
        Labels::default(),
        vec![(
            TextualMetadataRecord {
                module_path: vec!["dep".into()],
                visible_name: "Existing".into(),
                encoded_name: external.clone(),
            },
            IdentitySchema::new(
                external.clone(),
                [SchemaRole::Nominal { persistent: false }],
            )
            .unwrap(),
        )],
    );
    let collision_plan = fixture_with_import
        .reader
        .plan("Interface.{1 0 0}\n[dep.[Existing]]\n{[] [] [] [Thing.Existing]}")
        .unwrap();
    let collision_assignment = NamingAssignments::new(vec![NamingAssignment {
        occurrence: collision_plan.declarations()[0].occurrence(),
        encoded_name: external,
    }])
    .unwrap();
    assert!(matches!(
        fixture_with_import.reader.seal(
            &collision_plan,
            &collision_assignment,
            &GeneratedStreamAssignments::new(vec![]).unwrap(),
            &fixture_with_import.base_snapshot,
        ),
        Err(BootstrapReadError::AssignedIdentityCollision { .. })
    ));
}

#[test]
fn scopes_allow_sibling_reuse_but_refuse_duplicates_in_the_exact_scope() {
    let fixture = make_fixture(Labels::default(), vec![]);
    let valid =
        "Nexus.{1 0 0}\n[]\n{[First.{same.{Unit}} Second.{same.{Unit}}] [One.[Same] Two.[Same]]}";
    fixture.reader.plan(valid).expect("sibling scope reuse");
    for source in [
        "Interface.{1 0 0}\n[]\n{[Thing.String] [] [] [Thing.String]}",
        "Nexus.{1 0 0}\n[]\n{[Trait.{same.{Unit} same.{Unit}}] []}",
        "Interface.{1 0 0}\n[]\n{[] [] [] [One.[Same Same]]}",
    ] {
        assert!(matches!(
            fixture.reader.plan(source),
            Err(BootstrapReadError::DuplicateDeclaration { .. })
        ));
    }
}

#[test]
fn trait_vectors_normalize_by_explicit_encoded_bytes_and_named_binders_survive_roundtrip() {
    let a = id(80);
    let b = id(79);
    let extra = vec![
        (
            TextualMetadataRecord {
                module_path: vec!["traits".into()],
                visible_name: "Alpha".into(),
                encoded_name: a.clone(),
            },
            IdentitySchema::new(a.clone(), [SchemaRole::Trait]).unwrap(),
        ),
        (
            TextualMetadataRecord {
                module_path: vec!["traits".into()],
                visible_name: "Beta".into(),
                encoded_name: b.clone(),
            },
            IdentitySchema::new(b.clone(), [SchemaRole::Trait]).unwrap(),
        ),
    ];
    let fixture = make_fixture(Labels::default(), extra);
    let source = "Nexus.{1 0 0}\n[traits.[Alpha Beta]]\n{[] [Pair.{«Left.Alpha Beta» «Right.Beta Alpha» «Alpha Beta» «Beta Alpha»}]}";
    let (_, inputs, transaction) = seal(&fixture, source);
    let BootstrapBody::Nexus(body) = &transaction.decoded.document.body else {
        panic!("Nexus")
    };
    let Declaration::Type(pair) = &body.types[0] else {
        panic!("Pair")
    };
    let TypeBody::Struct(fields) = &pair.body else {
        panic!("Pair struct")
    };
    let requirements = fields
        .iter()
        .map(|field| match field {
            TypeExpression::TraitRequirement(requirement) => requirement,
            _ => panic!("requirement"),
        })
        .collect::<Vec<_>>();
    assert_eq!(requirements[0].required_traits(), [b.clone(), a.clone()]);
    assert_ne!(
        requirements[0].binder().parameter(),
        requirements[1].binder().parameter()
    );
    assert_eq!(
        requirements[2].binder().parameter(),
        requirements[3].binder().parameter()
    );
    let canonical = fixture.reader.write(&transaction).unwrap();
    assert!(canonical.contains("«Left.Beta Alpha»"));
    assert!(canonical.contains("«Right.Beta Alpha»"));
    let resealed = reseal_with_same_ids(
        &fixture,
        &canonical,
        &inputs.authored_ids,
        &inputs.generated_ids,
        &inputs.snapshot,
    );
    assert_eq!(resealed, transaction);

    assert!(matches!(
        fixture
            .reader
            .plan("Nexus.{1 0 0}\n[traits.[Alpha]]\n{[] [Bad.{«Alpha Alpha»}]}"),
        Err(BootstrapReadError::DuplicateTraitProjection(_))
    ));
}

#[test]
fn interface_reference_membership_is_root_owned_and_one_type_can_have_multiple_roles() {
    let fixture = make_fixture(Labels::default(), vec![]);
    let source = "Interface.{1 0 0}\n[]\n{[Shared] [Shared] [] [Shared.String]}";
    let (_, _, transaction) = seal(&fixture, source);
    let BootstrapBody::Interface(body) = &transaction.decoded.document.body else {
        panic!("Interface")
    };
    assert_eq!(body.memberships.len(), 2);
    assert_eq!(body.memberships[0].target, body.memberships[1].target);
    assert_eq!(body.memberships[0].role, InterfaceRole::Input);
    assert_eq!(body.memberships[1].role, InterfaceRole::Output);
}

#[test]
fn nexus_requires_one_final_return_and_sema_requires_persistent_nominal_leaves() {
    let fixture = make_fixture(Labels::default(), vec![]);
    assert!(
        fixture
            .reader
            .plan("Nexus.{1 0 0}\n[]\n{[Broken.{call.{}}] []}")
            .is_err()
    );
    seal(
        &fixture,
        "Nexus.{1 0 0}\n[]\n{[Marker.{} Valid.{call.{Unit}}] []}",
    );

    let shape = id(70);
    let external = id(71);
    let fixture = make_fixture(
        Labels::default(),
        vec![
            (
                TextualMetadataRecord {
                    module_path: vec!["dep".into()],
                    visible_name: "ShapeOnly".into(),
                    encoded_name: shape.clone(),
                },
                IdentitySchema::new(shape, [SchemaRole::Shape { arity: 1 }]).unwrap(),
            ),
            (
                TextualMetadataRecord {
                    module_path: vec!["dep".into()],
                    visible_name: "ExternalRecord".into(),
                    encoded_name: external.clone(),
                },
                IdentitySchema::new(external, [SchemaRole::Nominal { persistent: true }]).unwrap(),
            ),
        ],
    );
    let source = "Sema.{1 0 0}\n[dep.[ShapeOnly ExternalRecord]]\n{[Key.Integer] [bad.{ExternalRecord ShapeOnly}]}";
    let plan = fixture.reader.plan(source).expect("table structure plans");
    let inputs = inputs_for(&plan, &fixture.base_snapshot);
    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &inputs.assignments,
            &inputs.generated,
            &inputs.snapshot
        ),
        Err(BootstrapReadError::WrongSchemaRole { .. })
    ));
    assert!(matches!(
        fixture
            .reader
            .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Bad.Map<String>]}"),
        Err(BootstrapReadError::ShapeArity {
            expected: 2,
            found: 1,
            ..
        })
    ));
    assert!(matches!(
        fixture
            .reader
            .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Bad.StreamIdentity<String String>]}"),
        Err(BootstrapReadError::ShapeArity {
            expected: 1,
            found: 2,
            ..
        })
    ));
}

#[test]
fn stream_wrong_forms_and_generated_assignment_cardinality_are_refused() {
    let fixture = make_fixture(Labels::default(), vec![]);
    for source in [
        "Interface.{1 0 0}\n[]\n{[] [] [] [Bad.Stream.(String)]}",
        "Interface.{1 0 0}\n[]\n{[Bad.Stream.(String String)] [] [] []}",
        "Nexus.{1 0 0}\n[]\n{[] [Bad.Stream.(String String)]}",
    ] {
        assert!(fixture.reader.plan(source).is_err(), "{source}");
    }
    let source = "Interface.{1 0 0}\n[]\n{[] [] [] [Good.Stream.(String String)]}";
    let plan = fixture.reader.plan(source).unwrap();
    let inputs = inputs_for(&plan, &fixture.base_snapshot);
    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &inputs.assignments,
            &GeneratedStreamAssignments::new(vec![]).unwrap(),
            &inputs.snapshot
        ),
        Err(BootstrapReadError::MissingGeneratedStreamAssignment(0))
    ));
    let other = fixture
        .reader
        .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Other.Stream.(String String)]}")
        .unwrap();
    let extra = GeneratedStreamAssignments::new(vec![GeneratedStreamAssignment {
        source: other.declarations()[0].occurrence(),
        initiation: id(600),
        termination: id(601),
    }])
    .unwrap();
    assert!(matches!(
        fixture
            .reader
            .seal(&plan, &inputs.assignments, &extra, &inputs.snapshot),
        Err(BootstrapReadError::MissingGeneratedStreamAssignment(0))
            | Err(BootstrapReadError::ExtraGeneratedStreamAssignment)
    ));
}

#[test]
fn writer_revalidates_header_memberships_stream_anatomy_and_named_binders() {
    let fixture = make_fixture(Labels::default(), vec![]);
    let (_, _, transaction) = seal(&fixture, include_str!("fixtures/bootstrap/interface.ethos"));

    let mut wrong_header = transaction.clone();
    wrong_header.decoded.document.header.kind = EthosKind::Nexus;
    assert!(fixture.reader.write(&wrong_header).is_err());

    let mut wrong_memberships = transaction.clone();
    let BootstrapBody::Interface(body) = &mut wrong_memberships.decoded.document.body else {
        unreachable!()
    };
    body.memberships.pop();
    assert!(fixture.reader.write(&wrong_memberships).is_err());

    let mut wrong_stream = transaction.clone();
    wrong_stream.generated_streams[0]
        .output
        .stream_of_event
        .arguments
        .clear();
    assert!(fixture.reader.write(&wrong_stream).is_err());

    let traits = id(70);
    let fixture = make_fixture(
        Labels::default(),
        vec![(
            TextualMetadataRecord {
                module_path: vec!["dep".into()],
                visible_name: "Quality".into(),
                encoded_name: traits.clone(),
            },
            IdentitySchema::new(traits, [SchemaRole::Trait]).unwrap(),
        )],
    );
    let (_, inputs, transaction) = seal(
        &fixture,
        "Nexus.{1 0 0}\n[dep.[Quality]]\n{[] [Thing.{«Named.Quality»}]}",
    );
    let canonical = fixture.reader.write(&transaction).unwrap();
    assert!(canonical.contains("«Named.Quality»"));
    let resealed = reseal_with_same_ids(
        &fixture,
        &canonical,
        &inputs.authored_ids,
        &inputs.generated_ids,
        &inputs.snapshot,
    );
    assert_eq!(resealed, transaction);
}

#[test]
fn version_policy_is_explicit_and_reports_unsupported_versions() {
    let fixture = make_fixture(Labels::default(), vec![]);
    assert!(matches!(
        fixture.reader.plan("Interface.{2 0 0}\n[]\n{[] [] [] []}"),
        Err(BootstrapReadError::UnsupportedVersion { .. })
    ));
    for source in [
        "Interface.{01 0 0}\n[]\n{[] [] [] []}",
        "Interface.{1 0}\n[]\n{[] [] [] []}",
        "Interface.{1 -1 0}\n[]\n{[] [] [] []}",
    ] {
        assert!(fixture.reader.plan(source).is_err());
    }
}
