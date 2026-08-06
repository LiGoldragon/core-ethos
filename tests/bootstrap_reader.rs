use std::collections::BTreeMap;

use core_ethos::bootstrap::*;
use encoded_name_table::LocalEncodedId;
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};

fn id(local: u16) -> VocabularyEncodedId {
    VocabularyEncodedId::new(VocabularyRoot::Universal, vec![LocalEncodedId::new(local)])
        .expect("nonempty identity")
}

fn rust_id(local: u16) -> VocabularyEncodedId {
    VocabularyEncodedId::new(VocabularyRoot::Rust, vec![LocalEncodedId::new(local)])
        .expect("nonempty identity")
}

fn authority_bytes(local: u16) -> Vec<u8> {
    let mut bytes = vec![0x51];
    bytes.extend_from_slice(&local.to_be_bytes());
    bytes
}

const AUTHORITY_PROOF: u64 = 0x0065_7468_6f73;
const AUTHORITY_IDENTITY: u64 = 0x0062_7261_6e64_2d61;

#[derive(Clone, Debug, Eq, PartialEq)]
struct FakeAuthority {
    identity: u64,
    accepted_proof: u64,
}

impl FakeAuthority {
    const fn primary() -> Self {
        Self {
            identity: AUTHORITY_IDENTITY,
            accepted_proof: AUTHORITY_PROOF,
        }
    }

    const fn alternate() -> Self {
        Self {
            identity: AUTHORITY_IDENTITY + 1,
            accepted_proof: AUTHORITY_PROOF + 1,
        }
    }

    const fn receipt_seal(&self) -> u64 {
        self.identity.rotate_left(17) ^ self.accepted_proof
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FakeAuthorityReceipt {
    authority_identity: u64,
    transaction: PreparedBootstrapDraft,
    seal: u64,
}

impl BootstrapNamingAuthority for FakeAuthority {
    type Proof = u64;
    type Receipt = FakeAuthorityReceipt;

    fn authorize(
        &self,
        request: BootstrapNamingAuthorityRequest<'_>,
        proof: &Self::Proof,
    ) -> Option<Self::Receipt> {
        (*proof == self.accepted_proof).then(|| FakeAuthorityReceipt {
            authority_identity: self.identity,
            transaction: request.transaction().clone(),
            seal: self.receipt_seal(),
        })
    }

    fn verify_receipt(
        &self,
        request: BootstrapNamingAuthorityRequest<'_>,
        receipt: &Self::Receipt,
    ) -> bool {
        receipt.authority_identity == self.identity
            && receipt.transaction == *request.transaction()
            && receipt.seal == self.receipt_seal()
    }
}

#[derive(Clone, Debug)]
struct AlwaysTrueAuthority;

impl BootstrapNamingAuthority for AlwaysTrueAuthority {
    type Proof = ();
    // Deliberately reuse the receipt carrier: the transaction's Authority brand,
    // not carrier-type coincidence, prevents this authority from masquerading.
    type Receipt = FakeAuthorityReceipt;

    fn authorize(
        &self,
        request: BootstrapNamingAuthorityRequest<'_>,
        _proof: &Self::Proof,
    ) -> Option<Self::Receipt> {
        Some(FakeAuthorityReceipt {
            authority_identity: 0,
            transaction: request.transaction().clone(),
            seal: 0,
        })
    }

    fn verify_receipt(
        &self,
        _request: BootstrapNamingAuthorityRequest<'_>,
        _receipt: &Self::Receipt,
    ) -> bool {
        true
    }
}

type PreparedTransaction = PreparedBootstrapTransaction<FakeAuthority>;

fn record(
    module: &[&str],
    owner: Option<VocabularyEncodedId>,
    name: &str,
    identity: VocabularyEncodedId,
) -> TextualMetadataRecord {
    TextualMetadataRecord {
        address: TextualProjectionAddress {
            module_path: module.iter().map(|part| (*part).to_owned()).collect(),
            lexical_owner: owner,
            visible_name: name.to_owned(),
        },
        encoded_name: identity,
    }
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

#[derive(Clone)]
struct Extra {
    metadata: TextualMetadataRecord,
    schema: IdentitySchema,
    canonical_bytes: Vec<u8>,
}

#[derive(Clone)]
struct Fixture {
    reader: BootstrapReader<FakeAuthority>,
    authority: FakeAuthority,
    snapshot: TextualMetadataSnapshot,
    schemas: IdentitySchemaCatalog,
}

fn make_fixture(current_module: &[&str], extras: Vec<Extra>) -> Fixture {
    make_fixture_with_authority(current_module, extras, FakeAuthority::primary())
}

fn make_fixture_with_authority(
    current_module: &[&str],
    extras: Vec<Extra>,
    authority: FakeAuthority,
) -> Fixture {
    let prior_specs = [
        (
            1,
            "Interface",
            vec![SchemaRole::FileKind(EthosKind::Interface)],
        ),
        (2, "Nexus", vec![SchemaRole::FileKind(EthosKind::Nexus)]),
        (3, "Sema", vec![SchemaRole::FileKind(EthosKind::Sema)]),
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
        (7, "String", vec![SchemaRole::Nominal { persistent: true }]),
        (8, "Integer", vec![SchemaRole::Nominal { persistent: true }]),
        (9, "Boolean", vec![SchemaRole::Nominal { persistent: true }]),
        (10, "Unit", vec![SchemaRole::Nominal { persistent: true }]),
        (11, "Vector", vec![SchemaRole::Shape { arity: 1 }]),
        (12, "Option", vec![SchemaRole::Shape { arity: 1 }]),
        (13, "Map", vec![SchemaRole::Shape { arity: 2 }]),
        (14, "Result", vec![SchemaRole::Shape { arity: 2 }]),
        (
            15,
            "Stream",
            vec![
                SchemaRole::Shape { arity: 1 },
                SchemaRole::Nomos(NomosSchema::StreamInitiation { arity: 2 }),
            ],
        ),
        (16, "StreamIdentity", vec![SchemaRole::Shape { arity: 1 }]),
    ];
    let mut records = Vec::new();
    let mut schemas = Vec::new();
    let mut ordering = Vec::new();
    for (local, name, roles) in prior_specs {
        let identity = id(local);
        records.push(record(&["builtin"], None, name, identity.clone()));
        schemas.push(IdentitySchema::new(identity.clone(), roles).unwrap());
        ordering.push((identity, authority_bytes(local)));
    }
    for extra in extras {
        ordering.push((extra.metadata.encoded_name.clone(), extra.canonical_bytes));
        records.push(extra.metadata);
        schemas.push(extra.schema);
    }
    let snapshot = TextualMetadataSnapshot::new(records).unwrap();
    let schemas = IdentitySchemaCatalog::new(schemas).unwrap();
    let order = CanonicalIdentityOrder::new(ordering).unwrap();
    fixture_from_parts(current_module, snapshot, schemas, order, authority)
}

fn fixture_from_parts(
    current_module: &[&str],
    snapshot: TextualMetadataSnapshot,
    schemas: IdentitySchemaCatalog,
    order: CanonicalIdentityOrder,
    authority: FakeAuthority,
) -> Fixture {
    let priors = BootstrapPriorVocabulary::new(prior_identities(), &schemas, &snapshot).unwrap();
    let catalog = BootstrapCatalog::new(
        current_module
            .iter()
            .map(|part| (*part).to_owned())
            .collect(),
        snapshot.clone(),
        schemas.clone(),
        priors,
        BootstrapVersionPolicy::exact(EthosVersion::new(1, 0, 0)),
        order.clone(),
    )
    .unwrap();
    let reader = BootstrapReader::build(
        BootstrapGrammarIdentities {
            document: id(900),
            syntax: id(901),
        },
        catalog,
        authority.clone(),
    )
    .unwrap();
    Fixture {
        reader,
        authority,
        snapshot,
        schemas,
    }
}

fn restarted(
    prior: &Fixture,
    transaction: &PreparedTransaction,
    current_module: &[&str],
) -> Fixture {
    let schemas = IdentitySchemaCatalog::new(
        prior
            .schemas
            .entries()
            .chain(transaction.schema_additions().entries())
            .cloned()
            .collect(),
    )
    .unwrap();
    fixture_from_parts(
        current_module,
        transaction.naming_transition().after().clone(),
        schemas,
        transaction.canonical_order().clone(),
        prior.authority.clone(),
    )
}

#[derive(Clone)]
struct SealInputs {
    assignments: NamingAssignments,
    generated: GeneratedStreamAssignments,
    transition: TextualMetadataTransition,
    authored: Vec<VocabularyEncodedId>,
    stream_generated: BTreeMap<VocabularyEncodedId, (VocabularyEncodedId, VocabularyEncodedId)>,
}

fn new_inputs(plan: &BootstrapReadPlan, before: &TextualMetadataSnapshot) -> SealInputs {
    let mut records = before.records().to_vec();
    let mut identity_by_occurrence = BTreeMap::new();
    let mut authored = Vec::new();
    let mut raw_assignments = Vec::new();
    for (index, declaration) in plan.declarations().iter().enumerate() {
        let identity = id(100 + index as u16);
        identity_by_occurrence.insert(declaration.occurrence(), identity.clone());
        authored.push(identity.clone());
        raw_assignments.push(NamingAssignment {
            occurrence: declaration.occurrence(),
            encoded_name: identity,
            disposition: IdentityDisposition::New {
                canonical_bytes: vec![0x80, index as u8],
            },
        });
    }
    for (declaration, assignment) in plan.declarations().iter().zip(&raw_assignments) {
        let owner = match declaration.scope() {
            PlannedScope::Module => None,
            PlannedScope::Enum(owner) | PlannedScope::Trait(owner) => {
                Some(identity_by_occurrence[&owner].clone())
            }
        };
        records.push(record(
            &["app"],
            owner,
            declaration.spelling(),
            assignment.encoded_name.clone(),
        ));
    }
    let mut generated = Vec::new();
    let mut stream_generated = BTreeMap::new();
    let mut next = 500u16;
    for declaration in plan
        .declarations()
        .iter()
        .filter(|item| item.purpose() == DeclarationPurpose::StreamInitiation)
    {
        let output = identity_by_occurrence[&declaration.occurrence()].clone();
        let initiation = id(next);
        let termination = id(next + 1);
        next += 2;
        records.extend([
            record(
                &["app"],
                None,
                &format!("Start{}", declaration.spelling()),
                initiation.clone(),
            ),
            record(
                &["app"],
                None,
                &format!("Stop{}", declaration.spelling()),
                termination.clone(),
            ),
        ]);
        generated.push(GeneratedStreamAssignment {
            source: declaration.occurrence(),
            initiation: AssignedIdentity {
                encoded_name: initiation.clone(),
                disposition: IdentityDisposition::New {
                    canonical_bytes: vec![0x90, (next - 2) as u8],
                },
            },
            termination: AssignedIdentity {
                encoded_name: termination.clone(),
                disposition: IdentityDisposition::New {
                    canonical_bytes: vec![0x90, (next - 1) as u8],
                },
            },
        });
        stream_generated.insert(output, (initiation, termination));
    }
    let after = TextualMetadataSnapshot::new(records).unwrap();
    SealInputs {
        assignments: NamingAssignments::new(raw_assignments).unwrap(),
        generated: GeneratedStreamAssignments::new(generated).unwrap(),
        transition: TextualMetadataTransition::new(before.clone(), after),
        authored,
        stream_generated,
    }
}

fn existing_inputs(
    plan: &BootstrapReadPlan,
    before: &TextualMetadataSnapshot,
    after: TextualMetadataSnapshot,
    module: &[&str],
    generated_by_output: &BTreeMap<VocabularyEncodedId, (VocabularyEncodedId, VocabularyEncodedId)>,
) -> SealInputs {
    let module = module
        .iter()
        .map(|part| (*part).to_owned())
        .collect::<Vec<_>>();
    let mut identity_by_occurrence = BTreeMap::new();
    let mut authored = Vec::new();
    let mut raw = Vec::new();
    for declaration in plan.declarations() {
        let owner = match declaration.scope() {
            PlannedScope::Module => None,
            PlannedScope::Enum(owner) | PlannedScope::Trait(owner) => {
                Some(&identity_by_occurrence[&owner])
            }
        };
        let identity = after
            .identity_at(&module, owner, declaration.spelling())
            .unwrap()
            .clone();
        identity_by_occurrence.insert(declaration.occurrence(), identity.clone());
        authored.push(identity.clone());
        raw.push(NamingAssignment {
            occurrence: declaration.occurrence(),
            encoded_name: identity,
            disposition: IdentityDisposition::Existing,
        });
    }
    let generated = plan
        .declarations()
        .iter()
        .filter(|item| item.purpose() == DeclarationPurpose::StreamInitiation)
        .map(|declaration| {
            let output = &identity_by_occurrence[&declaration.occurrence()];
            let (initiation, termination) = &generated_by_output[output];
            GeneratedStreamAssignment {
                source: declaration.occurrence(),
                initiation: AssignedIdentity {
                    encoded_name: initiation.clone(),
                    disposition: IdentityDisposition::Existing,
                },
                termination: AssignedIdentity {
                    encoded_name: termination.clone(),
                    disposition: IdentityDisposition::Existing,
                },
            }
        })
        .collect();
    SealInputs {
        assignments: NamingAssignments::new(raw).unwrap(),
        generated: GeneratedStreamAssignments::new(generated).unwrap(),
        transition: TextualMetadataTransition::new(before.clone(), after),
        authored,
        stream_generated: generated_by_output.clone(),
    }
}

fn seal_new(
    fixture: &Fixture,
    source: &str,
) -> (BootstrapReadPlan, SealInputs, PreparedTransaction) {
    let plan = fixture.reader.plan(source).unwrap();
    let inputs = new_inputs(&plan, &fixture.snapshot);
    let transaction = fixture
        .reader
        .seal(
            &plan,
            &inputs.assignments,
            &inputs.generated,
            &inputs.transition,
            &AUTHORITY_PROOF,
        )
        .unwrap();
    (plan, inputs, transaction)
}

#[test]
fn stable_edits_rename_move_delete_and_restart_without_reminting() {
    let base = make_fixture(&["app"], vec![]);
    let source =
        "Interface.{1 0 0}\n[]\n{[] [] [] [Keep.String Drop.Integer Flow.Stream.(String Integer)]}";
    let (_, first_inputs, first) = seal_new(&base, source);
    assert!(
        first
            .identity_dispositions()
            .values()
            .all(|disposition| matches!(disposition, IdentityDisposition::New { .. }))
    );

    let committed = restarted(&base, &first, &["app"]);
    let canonical = base.reader.write(&first).unwrap();
    let unchanged_plan = committed.reader.plan(&canonical).unwrap();
    let unchanged_inputs = existing_inputs(
        &unchanged_plan,
        &committed.snapshot,
        committed.snapshot.clone(),
        &["app"],
        &first_inputs.stream_generated,
    );
    let unchanged = committed
        .reader
        .seal(
            &unchanged_plan,
            &unchanged_inputs.assignments,
            &unchanged_inputs.generated,
            &unchanged_inputs.transition,
            &AUTHORITY_PROOF,
        )
        .unwrap();
    assert_eq!(unchanged.decoded(), first.decoded());
    assert_eq!(unchanged.schema_additions().entries().count(), 0);
    assert!(
        unchanged
            .identity_dispositions()
            .values()
            .all(|disposition| disposition == &IdentityDisposition::Existing)
    );

    let keep = first_inputs.authored[0].clone();
    let drop_id = first_inputs.authored[1].clone();
    let flow = first_inputs.authored[2].clone();
    let mut renamed_records = committed.snapshot.records().to_vec();
    for metadata in &mut renamed_records {
        if metadata.encoded_name == keep {
            metadata.address.visible_name = "Renamed".into();
        }
    }
    let renamed_snapshot = TextualMetadataSnapshot::new(renamed_records).unwrap();
    let renamed_plan = committed
        .reader
        .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Renamed.String Drop.Integer Flow.Stream.(String Integer)]}")
        .unwrap();
    let renamed_inputs = existing_inputs(
        &renamed_plan,
        &committed.snapshot,
        renamed_snapshot,
        &["app"],
        &first_inputs.stream_generated,
    );
    let renamed = committed
        .reader
        .seal(
            &renamed_plan,
            &renamed_inputs.assignments,
            &renamed_inputs.generated,
            &renamed_inputs.transition,
            &AUTHORITY_PROOF,
        )
        .unwrap();
    assert_eq!(
        renamed_inputs.authored,
        vec![keep.clone(), drop_id.clone(), flow]
    );

    let renamed_committed = restarted(&committed, &renamed, &["moved"]);
    let mut moved_records = renamed.naming_transition().after().records().to_vec();
    for metadata in &mut moved_records {
        if metadata.address.module_path == ["app"]
            && renamed
                .identity_dispositions()
                .contains_key(&metadata.encoded_name)
        {
            metadata.address.module_path = vec!["moved".into()];
        }
    }
    moved_records.retain(|metadata| metadata.encoded_name != drop_id);
    let moved_after = TextualMetadataSnapshot::new(moved_records).unwrap();
    let moved_plan = renamed_committed
        .reader
        .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Renamed.String Flow.Stream.(String Integer)]}")
        .unwrap();
    let moved_inputs = existing_inputs(
        &moved_plan,
        renamed.naming_transition().after(),
        moved_after,
        &["moved"],
        &first_inputs.stream_generated,
    );
    let moved = renamed_committed
        .reader
        .seal(
            &moved_plan,
            &moved_inputs.assignments,
            &moved_inputs.generated,
            &moved_inputs.transition,
            &AUTHORITY_PROOF,
        )
        .unwrap();
    assert!(moved.naming_transition().after().record(&drop_id).is_none());
    assert!(
        moved
            .identity_dispositions()
            .values()
            .all(|disposition| disposition == &IdentityDisposition::Existing)
    );
}

#[test]
fn exact_projection_addresses_are_unique_while_nested_siblings_reuse_spellings() {
    let first = id(70);
    let second = id(71);
    assert!(matches!(
        TextualMetadataSnapshot::new(vec![
            record(&["dep"], None, "Same", first),
            record(&["dep"], None, "Same", second),
        ]),
        Err(BootstrapReadError::DuplicateMetadataProjectionAddress {
            module_path,
            lexical_owner: None,
            name,
        }) if module_path == ["dep"] && name == "Same"
    ));

    let fixture = make_fixture(&["app"], vec![]);
    let (_, _, transaction) = seal_new(
        &fixture,
        "Nexus.{1 0 0}\n[]\n{[First.{same.{Unit}} Second.{same.{Unit}}] [One.[Same] Two.[Same]]}",
    );
    let snapshot = transaction.naming_transition().after();
    let nested_same = snapshot
        .records()
        .iter()
        .filter(|metadata| metadata.address.visible_name == "Same")
        .collect::<Vec<_>>();
    assert_eq!(nested_same.len(), 2);
    assert_ne!(
        nested_same[0].address.lexical_owner,
        nested_same[1].address.lexical_owner
    );
    let nested_methods = snapshot
        .records()
        .iter()
        .filter(|metadata| metadata.address.visible_name == "same")
        .collect::<Vec<_>>();
    assert_eq!(nested_methods.len(), 2);
    assert_ne!(
        nested_methods[0].address.lexical_owner,
        nested_methods[1].address.lexical_owner
    );
}

#[test]
fn import_ambiguity_comes_from_distinct_valid_paths_and_is_namespace_local() {
    let left = id(70);
    let right = id(71);
    let fixture = make_fixture(
        &["app"],
        vec![
            Extra {
                metadata: record(&["left"], None, "Clash", left.clone()),
                schema: IdentitySchema::new(left, [SchemaRole::Nominal { persistent: false }])
                    .unwrap(),
                canonical_bytes: vec![0x21],
            },
            Extra {
                metadata: record(&["right"], None, "Clash", right.clone()),
                schema: IdentitySchema::new(right, [SchemaRole::Nominal { persistent: false }])
                    .unwrap(),
                canonical_bytes: vec![0x22],
            },
        ],
    );
    let plan = fixture
        .reader
        .plan("Interface.{1 0 0}\n[left.[Clash] right.[Clash]]\n{[Clash] [] [] []}")
        .unwrap();
    let inputs = new_inputs(&plan, &fixture.snapshot);
    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &inputs.assignments,
            &inputs.generated,
            &inputs.transition,
            &AUTHORITY_PROOF,
        ),
        Err(BootstrapReadError::AmbiguousReference { name, identities })
            if name == "Clash" && identities == vec![id(70), id(71)]
    ));

    let kind_collision = id(72);
    let fixture = make_fixture(
        &["app"],
        vec![Extra {
            metadata: record(&["dep"], None, "Interface", kind_collision.clone()),
            schema: IdentitySchema::new(
                kind_collision,
                [SchemaRole::Nominal { persistent: false }],
            )
            .unwrap(),
            canonical_bytes: vec![0x23],
        }],
    );
    seal_new(
        &fixture,
        "Interface.{1 0 0}\n[dep.[Interface]]\n{[Interface] [] [] []}",
    );
}

#[test]
fn shapes_and_nomos_heads_are_closed_prior_vocabulary() {
    let imported_shape = id(70);
    let imported_nomos = id(71);
    let fixture = make_fixture(
        &["app"],
        vec![
            Extra {
                metadata: record(&["dep"], None, "ForeignShape", imported_shape.clone()),
                schema: IdentitySchema::new(
                    imported_shape.clone(),
                    [SchemaRole::Shape { arity: 1 }],
                )
                .unwrap(),
                canonical_bytes: vec![0x31],
            },
            Extra {
                metadata: record(&["dep"], None, "ForeignNomos", imported_nomos.clone()),
                schema: IdentitySchema::new(
                    imported_nomos.clone(),
                    [SchemaRole::Nomos(NomosSchema::StreamInitiation {
                        arity: 2,
                    })],
                )
                .unwrap(),
                canonical_bytes: vec![0x32],
            },
        ],
    );
    assert!(matches!(
        fixture.reader.plan(
            "Interface.{1 0 0}\n[dep.[ForeignShape]]\n{[] [] [] [Bad.ForeignShape<String>]}"
        ),
        Err(BootstrapReadError::NonPriorShapeIdentity { identity }) if identity == imported_shape
    ));
    assert!(matches!(
        fixture.reader.plan(
            "Interface.{1 0 0}\n[dep.[ForeignNomos]]\n{[] [] [] [Bad.ForeignNomos.(String String)]}"
        ),
        Err(BootstrapReadError::NonPriorNomosIdentity { identity }) if identity == imported_nomos
    ));
}

#[test]
fn schema_roles_use_an_explicit_allowlist_and_priors_have_exact_relationships() {
    assert!(matches!(
        CanonicalIdentityOrder::new([(id(80), vec![1]), (id(81), vec![1])]),
        Err(BootstrapReadError::DuplicateCanonicalIdentityBytes { first, second })
            if first == id(80) && second == id(81)
    ));
    for roles in [
        vec![SchemaRole::Nominal { persistent: false }, SchemaRole::Trait],
        vec![SchemaRole::Shape { arity: 1 }, SchemaRole::Trait],
        vec![
            SchemaRole::Shape { arity: 1 },
            SchemaRole::Shape { arity: 2 },
        ],
    ] {
        assert!(matches!(
            IdentitySchema::new(id(70), roles),
            Err(BootstrapReadError::IncompatibleSchemaRoles { identity, .. }) if identity == id(70)
        ));
    }
    IdentitySchema::new(
        id(70),
        [
            SchemaRole::Shape { arity: 1 },
            SchemaRole::Nomos(NomosSchema::StreamInitiation { arity: 2 }),
        ],
    )
    .unwrap();

    let fixture = make_fixture(&["app"], vec![]);
    let mut wrong = prior_identities();
    wrong.stream_nomos = id(14);
    assert!(matches!(
        BootstrapPriorVocabulary::new(wrong, &fixture.schemas, &fixture.snapshot),
        Err(BootstrapReadError::InvalidPriorIdentityRelationship(
            "stream_nomos and stream_shape must be the same identity"
        ))
    ));
    let mut duplicate = prior_identities();
    duplicate.option_shape = duplicate.vector_shape.clone();
    assert!(matches!(
        BootstrapPriorVocabulary::new(duplicate, &fixture.schemas, &fixture.snapshot),
        Err(BootstrapReadError::DuplicatePriorIdentity {
            first: "vector_shape",
            second: "option_shape",
            ..
        })
    ));
}

#[test]
fn authority_bytes_canonicalize_every_unordered_named_collection() {
    let alpha = id(70);
    let beta = id(71);
    let fixture = make_fixture(
        &["app"],
        vec![
            Extra {
                metadata: record(&["traits"], None, "Alpha", alpha.clone()),
                schema: IdentitySchema::new(alpha.clone(), [SchemaRole::Trait]).unwrap(),
                canonical_bytes: vec![0x42],
            },
            Extra {
                metadata: record(&["traits"], None, "Beta", beta.clone()),
                schema: IdentitySchema::new(beta.clone(), [SchemaRole::Trait]).unwrap(),
                canonical_bytes: vec![0x41],
            },
        ],
    );
    let first = "Nexus.{1 0 0}\n[traits.[Alpha Beta]]\n{[Behavior.{z.{«Alpha Beta»} a.{Unit}}] [Second.[Y X] First.String]}";
    let second = "Nexus.{1 0 0}\n[traits.[Beta Alpha]]\n{[Behavior.{a.{Unit} z.{«Beta Alpha»}}] [First.String Second.[X Y]]}";
    let plan_a = fixture.reader.plan(first).unwrap();
    let plan_b = fixture.reader.plan(second).unwrap();

    let stable = [
        ("Behavior", id(100), vec![0x70]),
        ("z", id(101), vec![0x74]),
        ("a", id(102), vec![0x73]),
        ("Second", id(103), vec![0x72]),
        ("Y", id(104), vec![0x76]),
        ("X", id(105), vec![0x75]),
        ("First", id(106), vec![0x71]),
    ];
    let make = |plan: &BootstrapReadPlan| {
        let mut owner_by_occurrence = BTreeMap::new();
        let mut assignments = Vec::new();
        let mut records = fixture.snapshot.records().to_vec();
        for declaration in plan.declarations() {
            let (_, identity, bytes) = stable
                .iter()
                .find(|(name, _, _)| *name == declaration.spelling())
                .unwrap();
            owner_by_occurrence.insert(declaration.occurrence(), identity.clone());
            assignments.push(NamingAssignment {
                occurrence: declaration.occurrence(),
                encoded_name: identity.clone(),
                disposition: IdentityDisposition::New {
                    canonical_bytes: bytes.clone(),
                },
            });
        }
        for declaration in plan.declarations() {
            let identity = &owner_by_occurrence[&declaration.occurrence()];
            let owner = match declaration.scope() {
                PlannedScope::Module => None,
                PlannedScope::Enum(owner) | PlannedScope::Trait(owner) => {
                    Some(owner_by_occurrence[&owner].clone())
                }
            };
            records.push(record(
                &["app"],
                owner,
                declaration.spelling(),
                identity.clone(),
            ));
        }
        (
            NamingAssignments::new(assignments).unwrap(),
            TextualMetadataTransition::new(
                fixture.snapshot.clone(),
                TextualMetadataSnapshot::new(records).unwrap(),
            ),
        )
    };
    let (assignments_a, transition_a) = make(&plan_a);
    let (assignments_b, transition_b) = make(&plan_b);
    assert_eq!(transition_a.after(), transition_b.after());
    let empty = GeneratedStreamAssignments::new(vec![]).unwrap();
    let transaction_a = fixture
        .reader
        .seal(
            &plan_a,
            &assignments_a,
            &empty,
            &transition_a,
            &AUTHORITY_PROOF,
        )
        .unwrap();
    let transaction_b = fixture
        .reader
        .seal(
            &plan_b,
            &assignments_b,
            &empty,
            &transition_b,
            &AUTHORITY_PROOF,
        )
        .unwrap();
    assert_eq!(transaction_a, transaction_b);
    let BootstrapBody::Nexus(body) = &transaction_a.decoded().document.body else {
        panic!("Nexus")
    };
    assert_eq!(
        body.types[0].clone(),
        Declaration::Type(TypeDeclaration {
            name: id(106),
            body: TypeBody::Newtype(TypeExpression::Reference(id(7))),
        })
    );
    assert_eq!(body.traits[0].methods[0].name, id(102));
    let Declaration::Type(second) = &body.types[1] else {
        panic!("Second")
    };
    let TypeBody::Enum(variants) = &second.body else {
        panic!("enum")
    };
    assert_eq!(
        variants
            .iter()
            .map(|item| item.name.clone())
            .collect::<Vec<_>>(),
        vec![id(105), id(104)]
    );
}

#[test]
fn role_entry_keeps_plain_types_and_references_but_nomos_only_support_types() {
    let fixture = make_fixture(&["app"], vec![]);
    seal_new(
        &fixture,
        "Interface.{1 0 0}\n[]\n{[Inline.String] [Inline] [] [Flow.Stream.(String Integer)]}",
    );
    assert!(matches!(
        fixture
            .reader
            .plan("Interface.{1 0 0}\n[]\n{[Flow.Stream.(String Integer)] [] [] []}"),
        Err(BootstrapReadError::StreamOutsideInterfaceTypes)
    ));
}

#[test]
fn exact_plan_errors_cover_method_stream_shape_table_and_versions() {
    let fixture = make_fixture(&["app"], vec![]);
    assert!(matches!(
        fixture
            .reader
            .plan("Nexus.{1 0 0}\n[]\n{[Bad.{call.{}}] []}"),
        Err(BootstrapReadError::UnexpectedStructure {
            expected: "mandatory method return",
            ..
        })
    ));
    assert!(matches!(
        fixture.reader.plan(
            "Interface.{1 0 0}\n[]\n{[] [] [] [Bad.Stream.(String)]}"
        ),
        Err(BootstrapReadError::NomosArity {
            identity,
            expected: 2,
            found: 1,
        }) if identity == id(15)
    ));
    assert!(matches!(
        fixture.reader.plan(
            "Interface.{1 0 0}\n[]\n{[] [] [] [Bad.Map<String>]}"
        ),
        Err(BootstrapReadError::ShapeArity {
            identity,
            expected: 2,
            found: 1,
        }) if identity == id(13)
    ));
    assert!(matches!(
        fixture.reader.plan(
            "Interface.{1 0 0}\n[]\n{[] [] [] [Bad.Stream<String Integer>]}"
        ),
        Err(BootstrapReadError::ShapeArity {
            identity,
            expected: 1,
            found: 2,
        }) if identity == id(15)
    ));
    assert!(matches!(
        fixture
            .reader
            .plan("Nexus.{1 0 0}\n[]\n{[] [Bad.Stream.(String Integer)]}"),
        Err(BootstrapReadError::StreamOutsideInterfaceTypes)
    ));
    assert!(matches!(
        fixture.reader.plan("Sema.{1 0 0}\n[]\n{[] [bad.{String}]}"),
        Err(BootstrapReadError::UnexpectedStructure {
            expected: "exactly RecordType then KeyType",
            ..
        })
    ));
    assert!(matches!(
        fixture.reader.plan("Interface.{2 0 0}\n[]\n{[] [] [] []}"),
        Err(BootstrapReadError::UnsupportedVersion {
            found: EthosVersion {
                major: 2,
                minor: 0,
                patch: 0
            },
            ..
        })
    ));
    assert!(matches!(
        fixture
            .reader
            .plan("Interface.{01 0 0}\n[]\n{[] [] [] []}"),
        Err(BootstrapReadError::InvalidVersionComponent(component)) if component == "01"
    ));
    assert!(matches!(
        fixture.reader.plan("Interface.{1 0}\n[]\n{[] [] [] []}"),
        Err(BootstrapReadError::UnexpectedStructure {
            expected: "exactly three version components",
            ..
        })
    ));
}

#[test]
fn table_leaves_are_exactly_persistent_nominals() {
    let shape = id(70);
    let fixture = make_fixture(
        &["app"],
        vec![Extra {
            metadata: record(&["dep"], None, "ShapeOnly", shape.clone()),
            schema: IdentitySchema::new(shape, [SchemaRole::Shape { arity: 1 }]).unwrap(),
            canonical_bytes: vec![0x33],
        }],
    );
    let plan = fixture
        .reader
        .plan("Sema.{1 0 0}\n[dep.[ShapeOnly]]\n{[Record.String] [bad.{Record ShapeOnly}]}")
        .unwrap();
    let inputs = new_inputs(&plan, &fixture.snapshot);
    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &inputs.assignments,
            &inputs.generated,
            &inputs.transition,
            &AUTHORITY_PROOF,
        ),
        Err(BootstrapReadError::UnresolvedReference { name }) if name == "ShapeOnly"
    ));
}

#[test]
fn assignment_dispositions_authority_collisions_and_cardinalities_are_exact() {
    let fixture = make_fixture(&["app"], vec![]);
    let plan = fixture
        .reader
        .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Thing.String]}")
        .unwrap();
    let inputs = new_inputs(&plan, &fixture.snapshot);
    let empty = NamingAssignments::new(vec![]).unwrap();
    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &empty,
            &inputs.generated,
            &inputs.transition,
            &AUTHORITY_PROOF,
        ),
        Err(BootstrapReadError::MissingAssignment(0))
    ));
    let duplicate = NamingAssignments::new(vec![
        NamingAssignment {
            occurrence: plan.declarations()[0].occurrence(),
            encoded_name: id(100),
            disposition: IdentityDisposition::New {
                canonical_bytes: vec![1],
            },
        },
        NamingAssignment {
            occurrence: plan.declarations()[0].occurrence(),
            encoded_name: id(101),
            disposition: IdentityDisposition::New {
                canonical_bytes: vec![2],
            },
        },
    ]);
    assert!(matches!(
        duplicate,
        Err(BootstrapReadError::DuplicateAssignment(0))
    ));

    let foreign_plan = fixture
        .reader
        .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Foreign.String]}")
        .unwrap();
    let extra_assignment = NamingAssignments::new(vec![
        NamingAssignment {
            occurrence: plan.declarations()[0].occurrence(),
            encoded_name: id(100),
            disposition: IdentityDisposition::New {
                canonical_bytes: vec![0x80, 0],
            },
        },
        NamingAssignment {
            occurrence: foreign_plan.declarations()[0].occurrence(),
            encoded_name: id(101),
            disposition: IdentityDisposition::New {
                canonical_bytes: vec![0x80, 1],
            },
        },
    ])
    .unwrap();
    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &extra_assignment,
            &inputs.generated,
            &inputs.transition,
            &AUTHORITY_PROOF,
        ),
        Err(BootstrapReadError::ExtraAssignment)
    ));

    let assignment = |identity, disposition| {
        NamingAssignments::new(vec![NamingAssignment {
            occurrence: plan.declarations()[0].occurrence(),
            encoded_name: identity,
            disposition,
        }])
        .unwrap()
    };
    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &assignment(
                id(100),
                IdentityDisposition::Existing,
            ),
            &inputs.generated,
            &inputs.transition,
            &AUTHORITY_PROOF,
        ),
        Err(BootstrapReadError::ExistingAssignmentMissing { identity }) if identity == id(100)
    ));
    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &assignment(
                id(7),
                IdentityDisposition::New { canonical_bytes: vec![0x99] },
            ),
            &inputs.generated,
            &inputs.transition,
            &AUTHORITY_PROOF,
        ),
        Err(BootstrapReadError::NewAssignmentAlreadyExists { identity }) if identity == id(7)
    ));
    let prior_existing_records = inputs.transition.after().records().to_vec();
    let prior_existing_transition = TextualMetadataTransition::new(
        fixture.snapshot.clone(),
        TextualMetadataSnapshot::new(prior_existing_records).unwrap(),
    );
    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &assignment(id(7), IdentityDisposition::Existing),
            &inputs.generated,
            &prior_existing_transition,
            &AUTHORITY_PROOF,
        ),
        Err(BootstrapReadError::ExistingAssignmentNotReusable {
            identity,
            required: SchemaRole::Nominal { persistent: false },
        }) if identity == id(7)
    ));
    let mut rust_records = fixture.snapshot.records().to_vec();
    rust_records.push(record(&["app"], None, "Thing", rust_id(40)));
    let rust_transition = TextualMetadataTransition::new(
        fixture.snapshot.clone(),
        TextualMetadataSnapshot::new(rust_records).unwrap(),
    );
    let opaque_identity_transaction = fixture
        .reader
        .seal(
            &plan,
            &assignment(
                rust_id(40),
                IdentityDisposition::New {
                    canonical_bytes: vec![0x98],
                },
            ),
            &inputs.generated,
            &rust_transition,
            &AUTHORITY_PROOF,
        )
        .unwrap();
    let BootstrapBody::Interface(body) = &opaque_identity_transaction.decoded().document.body
    else {
        panic!("Interface")
    };
    let Declaration::Type(declaration) = &body.types[0] else {
        panic!("Type")
    };
    assert_eq!(declaration.name, rust_id(40));

    let stream_plan = fixture
        .reader
        .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Flow.Stream.(String Integer)]}")
        .unwrap();
    let stream_inputs = new_inputs(&stream_plan, &fixture.snapshot);
    assert!(matches!(
        fixture.reader.seal(
            &stream_plan,
            &stream_inputs.assignments,
            &GeneratedStreamAssignments::new(vec![]).unwrap(),
            &stream_inputs.transition,
            &AUTHORITY_PROOF,
        ),
        Err(BootstrapReadError::MissingGeneratedStreamAssignment(0))
    ));
    let source = stream_plan.declarations()[0].occurrence();
    let (initiation, termination) = stream_inputs.stream_generated.values().next().unwrap();
    let generated_assignment = GeneratedStreamAssignment {
        source,
        initiation: AssignedIdentity {
            encoded_name: initiation.clone(),
            disposition: IdentityDisposition::New {
                canonical_bytes: vec![0x90, 0xf4],
            },
        },
        termination: AssignedIdentity {
            encoded_name: termination.clone(),
            disposition: IdentityDisposition::New {
                canonical_bytes: vec![0x90, 0xf5],
            },
        },
    };
    assert!(matches!(
        GeneratedStreamAssignments::new(vec![generated_assignment.clone(), generated_assignment,]),
        Err(BootstrapReadError::DuplicateGeneratedStreamAssignment(0))
    ));
    let other_plan = fixture
        .reader
        .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Other.String]}")
        .unwrap();
    let other_inputs = new_inputs(&other_plan, &fixture.snapshot);
    let extra_generated = GeneratedStreamAssignments::new(vec![GeneratedStreamAssignment {
        source: other_plan.declarations()[0].occurrence(),
        initiation: AssignedIdentity {
            encoded_name: id(700),
            disposition: IdentityDisposition::New {
                canonical_bytes: vec![0xa1],
            },
        },
        termination: AssignedIdentity {
            encoded_name: id(701),
            disposition: IdentityDisposition::New {
                canonical_bytes: vec![0xa2],
            },
        },
    }])
    .unwrap();
    assert!(matches!(
        fixture.reader.seal(
            &other_plan,
            &other_inputs.assignments,
            &extra_generated,
            &other_inputs.transition,
            &AUTHORITY_PROOF,
        ),
        Err(BootstrapReadError::ExtraGeneratedStreamAssignment)
    ));

    let collision = GeneratedStreamAssignments::new(vec![GeneratedStreamAssignment {
        source,
        initiation: AssignedIdentity {
            encoded_name: stream_inputs.authored[0].clone(),
            disposition: IdentityDisposition::New {
                canonical_bytes: vec![0xa3],
            },
        },
        termination: AssignedIdentity {
            encoded_name: termination.clone(),
            disposition: IdentityDisposition::New {
                canonical_bytes: vec![0x90, 0xf5],
            },
        },
    }])
    .unwrap();
    assert!(matches!(
        fixture.reader.seal(
            &stream_plan,
            &stream_inputs.assignments,
            &collision,
            &stream_inputs.transition,
            &AUTHORITY_PROOF,
        ),
        Err(BootstrapReadError::AssignedIdentityCollision { identity })
            if identity == stream_inputs.authored[0]
    ));
}

#[test]
fn authority_transition_is_external_and_writer_uses_the_exact_after_snapshot() {
    let fixture = make_fixture(&["app"], vec![]);
    let plan = fixture
        .reader
        .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Thing.String]}")
        .unwrap();
    let inputs = new_inputs(&plan, &fixture.snapshot);
    let foreign_before = TextualMetadataSnapshot::new(vec![]).unwrap();
    let wrong_transition =
        TextualMetadataTransition::new(foreign_before, inputs.transition.after().clone());
    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &inputs.assignments,
            &inputs.generated,
            &inputs.transition,
            &(AUTHORITY_PROOF + 1),
        ),
        Err(BootstrapReadError::NamingAuthorityRejected)
    ));
    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &inputs.assignments,
            &inputs.generated,
            &wrong_transition,
            &AUTHORITY_PROOF,
        ),
        Err(BootstrapReadError::MetadataTransitionBeforeMismatch)
    ));
    let transaction = fixture
        .reader
        .seal(
            &plan,
            &inputs.assignments,
            &inputs.generated,
            &inputs.transition,
            &AUTHORITY_PROOF,
        )
        .unwrap();
    assert!(matches!(
        fixture
            .reader
            .validate_draft(transaction.to_draft(), &(AUTHORITY_PROOF + 1)),
        Err(BootstrapReadError::NamingAuthorityRejected)
    ));
    let written = fixture.reader.write(&transaction).unwrap();
    assert!(written.contains("Thing.String"));
}

#[test]
fn authority_receipts_are_exact_configuration_brands_at_store_validation_and_write() {
    fn store_accepts<Authority: BootstrapNamingAuthority>(
        authority: &Authority,
        transaction: &PreparedBootstrapTransaction<Authority>,
    ) -> Result<(), BootstrapReadError> {
        transaction.verify_naming_authority(authority)
    }

    let fixture = make_fixture(&["app"], vec![]);
    let (_, _, transaction) =
        seal_new(&fixture, "Interface.{1 0 0}\n[]\n{[] [] [] [Thing.String]}");
    let _: &FakeAuthorityReceipt = transaction.naming_authority_receipt();
    store_accepts(&fixture.authority, &transaction).unwrap();
    fixture.reader.validate_transaction(&transaction).unwrap();
    fixture.reader.write(&transaction).unwrap();

    let alternate = make_fixture_with_authority(&["app"], vec![], FakeAuthority::alternate());
    assert!(matches!(
        store_accepts(&alternate.authority, &transaction),
        Err(BootstrapReadError::NamingAuthorityReceiptRejected)
    ));
    assert!(matches!(
        alternate.reader.validate_transaction(&transaction),
        Err(BootstrapReadError::NamingAuthorityReceiptRejected)
    ));
    assert!(matches!(
        alternate.reader.write(&transaction),
        Err(BootstrapWriteError::InvalidModel(
            BootstrapReadError::NamingAuthorityReceiptRejected
        ))
    ));

    // Even sharing the exact Receipt carrier cannot erase the authority brand.
    assert_ne!(
        std::any::TypeId::of::<PreparedBootstrapTransaction<FakeAuthority>>(),
        std::any::TypeId::of::<PreparedBootstrapTransaction<AlwaysTrueAuthority>>()
    );
}

#[test]
fn seal_refuses_an_after_snapshot_that_makes_an_imported_reference_invisible() {
    let external = id(70);
    let fixture = make_fixture(
        &["app"],
        vec![Extra {
            metadata: record(&["dep"], None, "External", external.clone()),
            schema: IdentitySchema::new(external, [SchemaRole::Nominal { persistent: false }])
                .unwrap(),
            canonical_bytes: vec![0x35],
        }],
    );
    let plan = fixture
        .reader
        .plan("Interface.{1 0 0}\n[dep.[External]]\n{[] [] [] [Thing.External]}")
        .unwrap();
    let mut inputs = new_inputs(&plan, &fixture.snapshot);
    let after = TextualMetadataSnapshot::new(
        inputs
            .transition
            .after()
            .records()
            .iter()
            .filter(|metadata| metadata.address.visible_name != "External")
            .cloned()
            .collect(),
    )
    .unwrap();
    inputs.transition = TextualMetadataTransition::new(fixture.snapshot.clone(), after);
    assert!(matches!(
        fixture.reader.seal(
            &plan,
            &inputs.assignments,
            &inputs.generated,
            &inputs.transition,
            &AUTHORITY_PROOF,
        ),
        Err(BootstrapReadError::MissingTextualLookup { module_path, name })
            if module_path == ["dep"] && name == "External"
    ));
}

#[test]
fn validated_transaction_wrapper_refuses_every_writer_invariant_break() {
    let hidden = id(70);
    let fixture = make_fixture(
        &["app"],
        vec![Extra {
            metadata: record(&["hidden"], None, "Hidden", hidden.clone()),
            schema: IdentitySchema::new(
                hidden.clone(),
                [SchemaRole::Nominal { persistent: false }],
            )
            .unwrap(),
            canonical_bytes: vec![0x36],
        }],
    );
    let (_, inputs, transaction) = seal_new(
        &fixture,
        "Interface.{1 0 0}\n[]\n{[Shared] [] [] [Shared.String Flow.Stream.(String Integer)]}",
    );

    let mut wrong_header = transaction.to_draft();
    wrong_header.decoded.document.header.kind = EthosKind::Nexus;
    assert!(matches!(
        fixture
            .reader
            .validate_draft(wrong_header, &AUTHORITY_PROOF),
        Err(BootstrapReadError::HeaderBodyMismatch {
            header: EthosKind::Nexus,
            body: EthosKind::Interface,
        })
    ));

    let mut wrong_membership = transaction.to_draft();
    let BootstrapBody::Interface(body) = &mut wrong_membership.decoded.document.body else {
        panic!("Interface")
    };
    body.memberships.clear();
    assert!(matches!(
        fixture
            .reader
            .validate_draft(wrong_membership, &AUTHORITY_PROOF),
        Err(BootstrapReadError::InvalidPreparedModel(
            "Interface memberships do not exactly equal role entries"
        ))
    ));

    let mut wrong_stream = transaction.to_draft();
    wrong_stream.generated_streams[0]
        .output
        .stream_of_event
        .arguments
        .clear();
    assert!(matches!(
        fixture
            .reader
            .validate_draft(wrong_stream, &AUTHORITY_PROOF),
        Err(BootstrapReadError::InvalidPreparedModel(
            "generated Stream declaration anatomy"
        ))
    ));

    let mut invisible = transaction.to_draft();
    let BootstrapBody::Interface(body) = &mut invisible.decoded.document.body else {
        panic!("Interface")
    };
    let Some(Declaration::Type(shared)) = body.types.iter_mut().find(|declaration| {
        matches!(declaration, Declaration::Type(declaration) if matches!(declaration.body, TypeBody::Newtype(_)))
    }) else {
        panic!("Shared")
    };
    shared.body = TypeBody::Newtype(TypeExpression::Reference(hidden.clone()));
    assert!(matches!(
        fixture
            .reader
            .validate_draft(invisible, &AUTHORITY_PROOF),
        Err(BootstrapReadError::InvisibleOrNonRoundTrippingReference {
            identity,
            name,
        }) if identity == hidden && name == "Hidden"
    ));

    let unrelated = id(75);
    let mut extra_after = transaction.to_draft();
    let mut extra_records = extra_after.naming_transition.after().records().to_vec();
    extra_records.push(record(&["unrelated"], None, "Unrelated", unrelated.clone()));
    extra_after.naming_transition = TextualMetadataTransition::new(
        fixture.snapshot.clone(),
        TextualMetadataSnapshot::new(extra_records).unwrap(),
    );
    assert!(matches!(
        fixture
            .reader
            .validate_draft(extra_after, &AUTHORITY_PROOF),
        Err(BootstrapReadError::ExtraMetadataIdentity(identity)) if identity == unrelated
    ));

    let generated_initiation = transaction.generated_streams()[0].initiation.name.clone();
    let generated_name = transaction
        .naming_transition()
        .after()
        .spelling(&generated_initiation)
        .unwrap()
        .to_owned();
    let mut authored_generated_reference = transaction.to_draft();
    let BootstrapBody::Interface(body) = &mut authored_generated_reference.decoded.document.body
    else {
        panic!("Interface")
    };
    let Some(Declaration::Type(shared)) = body.types.iter_mut().find(|declaration| {
        matches!(declaration, Declaration::Type(declaration) if matches!(declaration.body, TypeBody::Newtype(_)))
    }) else {
        panic!("Shared")
    };
    shared.body = TypeBody::Newtype(TypeExpression::Reference(generated_initiation.clone()));
    assert!(matches!(
        fixture
            .reader
            .validate_draft(authored_generated_reference, &AUTHORITY_PROOF),
        Err(BootstrapReadError::InvisibleOrNonRoundTrippingReference { identity, name })
            if identity == generated_initiation && name == generated_name
    ));

    let canonical = fixture.reader.write(&transaction).unwrap();
    let committed = restarted(&fixture, &transaction, &["app"]);
    let reseal_plan = committed.reader.plan(&canonical).unwrap();
    let reseal_inputs = existing_inputs(
        &reseal_plan,
        &committed.snapshot,
        committed.snapshot.clone(),
        &["app"],
        &inputs.stream_generated,
    );
    committed
        .reader
        .seal(
            &reseal_plan,
            &reseal_inputs.assignments,
            &reseal_inputs.generated,
            &reseal_inputs.transition,
            &AUTHORITY_PROOF,
        )
        .unwrap();
}

#[test]
fn root_registry_is_the_observable_section_order() {
    let fixture = make_fixture(&["app"], vec![]);
    assert_eq!(
        fixture.reader.section_order(EthosKind::Interface),
        [
            BootstrapSectionSchema::Role(InterfaceRole::Input),
            BootstrapSectionSchema::Role(InterfaceRole::Output),
            BootstrapSectionSchema::Role(InterfaceRole::Refusal),
            BootstrapSectionSchema::Declarations { admit_nomos: true },
        ]
    );
    assert_eq!(
        fixture.reader.section_order(EthosKind::Nexus),
        [
            BootstrapSectionSchema::Traits,
            BootstrapSectionSchema::Declarations { admit_nomos: false },
        ]
    );
    assert_eq!(
        fixture.reader.section_order(EthosKind::Sema),
        [
            BootstrapSectionSchema::PersistentDeclarations,
            BootstrapSectionSchema::Tables,
        ]
    );
}

#[test]
fn runtime_stream_values_use_registered_identity_and_the_same_stream_handle() {
    let identities = CanonicalIdentityOrder::new([(id(70), vec![1]), (id(71), vec![2])]).unwrap();
    let handle = RuntimeStreamIdentity::<String>::new(id(70), &identities).unwrap();
    let stream = RuntimeStream::new(handle);
    let termination = RuntimeStreamTermination::new(stream.clone());
    let values =
        RuntimeStreamValues::new(RuntimeStreamInitiation::new(42u64), stream, termination).unwrap();
    assert_eq!(values.initiation().query(), &42);
    assert_eq!(values.stream(), values.termination().stream());
    assert!(matches!(
        RuntimeStreamIdentity::<String>::new(rust_id(70), &identities),
        Err(RuntimeStreamValueError::UnrecognizedHandle(identity)) if identity == rust_id(70)
    ));
    let stream =
        RuntimeStream::new(RuntimeStreamIdentity::<String>::new(id(70), &identities).unwrap());
    let other_stream =
        RuntimeStream::new(RuntimeStreamIdentity::<String>::new(id(71), &identities).unwrap());
    assert!(matches!(
        RuntimeStreamValues::new(
            RuntimeStreamInitiation::new(()),
            stream,
            RuntimeStreamTermination::new(other_stream),
        ),
        Err(RuntimeStreamValueError::MismatchedTerminationStream {
            stream,
            termination,
        }) if stream == id(70) && termination == id(71)
    ));
}

#[test]
fn archive_boundary_remains_explicitly_deferred() {
    let fixture = make_fixture(&["app"], vec![]);
    assert_eq!(
        fixture.reader.archive_status(),
        BootstrapArchiveStatus::NotYetArchived
    );
}
