use std::collections::BTreeMap;

use core_ethos::bootstrap::{
    BootstrapBody, BootstrapCatalog, BootstrapGrammarIdentity, BootstrapPriorIdentities,
    BootstrapPriorVocabulary, BootstrapReadError, BootstrapReader, Declaration, EthosKind,
    NamingAssignment, NamingAssignments, TextualMetadataEntry, TypeBody, TypeExpression,
};
use encoded_name_table::{EncodedId, LocalEncodedId, Name};
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::EncodedNameResolver;

fn id(local: u16) -> VocabularyEncodedId {
    VocabularyEncodedId::new(VocabularyRoot::Universal, vec![LocalEncodedId::new(local)])
        .expect("nonempty test identity")
}

fn priors() -> BootstrapPriorVocabulary {
    BootstrapPriorVocabulary::new(BootstrapPriorIdentities {
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
        stream_shape: id(16),
        stream_identity_shape: id(17),
    })
    .expect("Universal priors")
}

fn reader(entries: Vec<TextualMetadataEntry>) -> BootstrapReader {
    BootstrapReader::build(
        BootstrapGrammarIdentity(id(900)),
        BootstrapCatalog::new(priors(), entries).expect("catalog"),
    )
    .expect("shared reader")
}

fn assignments(
    plan: &core_ethos::bootstrap::BootstrapReadPlan,
) -> (NamingAssignments, BTreeMap<VocabularyEncodedId, Name>) {
    let mut spellings = BTreeMap::new();
    let assignments = plan
        .declarations()
        .iter()
        .enumerate()
        .map(|(index, declaration)| {
            let encoded_name = id(100 + index as u16);
            spellings.insert(encoded_name.clone(), Name::new(declaration.spelling()));
            NamingAssignment {
                occurrence: declaration.occurrence(),
                encoded_name,
            }
        })
        .collect();
    (
        NamingAssignments::new(assignments).expect("exact unique assignments"),
        spellings,
    )
}

struct Names(BTreeMap<VocabularyEncodedId, Name>);

impl EncodedNameResolver<VocabularyRoot> for Names {
    fn resolve(&self, encoded_id: &EncodedId<VocabularyRoot>) -> Option<&Name> {
        self.0.get(encoded_id)
    }
}

fn names(mut declarations: BTreeMap<VocabularyEncodedId, Name>) -> Names {
    let prior_names = [
        (7, "String"),
        (8, "Integer"),
        (9, "Boolean"),
        (10, "Unit"),
        (11, "Vector"),
        (12, "Option"),
        (13, "Map"),
        (14, "Result"),
        (15, "Stream"),
        (16, "Stream"),
        (17, "StreamIdentity"),
    ];
    declarations.extend(
        prior_names
            .into_iter()
            .map(|(local, spelling)| (id(local), Name::new(spelling))),
    );
    Names(declarations)
}

#[test]
fn interface_plan_seal_and_writer_preserve_the_strict_stream_transaction() {
    let source = include_str!("fixtures/bootstrap/interface.ethos");
    let reader = reader(vec![]);
    let plan = reader
        .plan(source)
        .expect("structurally and semantically planned");
    assert_eq!(plan.header().kind, EthosKind::Interface);
    assert_eq!(plan.declarations().len(), 7);
    assert_eq!(plan.declarations()[4].spelling(), "Observer");
    assert_eq!(plan.declarations()[5].spelling(), "ObserverInitiation");
    assert_eq!(plan.declarations()[6].spelling(), "ObserverTermination");
    let observer_bound = plan.declarations()[4].bound();
    assert_eq!(
        &source[observer_bound.start()..observer_bound.end()],
        "Observer"
    );
    assert_eq!(plan.declarations()[5].bound(), observer_bound);
    assert_eq!(plan.declarations()[6].bound(), observer_bound);

    let (assignments, declaration_names) = assignments(&plan);
    let decoded = reader
        .seal(&plan, &assignments)
        .expect("exact assignment seal");
    let BootstrapBody::Interface(body) = &decoded.document.body else {
        panic!("Interface body")
    };
    assert_eq!(body.memberships.len(), 6);
    let Declaration::Stream(stream) = &body.types[2] else {
        panic!("strict Stream arm")
    };
    assert_eq!(stream.output.name, stream.termination.stream_handle);
    assert!(body.memberships.iter().any(|membership| {
        membership.role == core_ethos::bootstrap::InterfaceRole::Output
            && membership.target == stream.output.name
    }));
    assert!(matches!(
        stream.output.stream_of_event.arguments.as_slice(),
        [TypeExpression::Reference(_)]
    ));

    let canonical = reader
        .write(&decoded, &names(declaration_names))
        .expect("canonical text");
    let second_plan = reader.plan(&canonical).expect("writer output plans");
    assert_eq!(second_plan.declarations().len(), plan.declarations().len());
}

#[test]
fn nexus_traits_are_first_and_named_trait_binders_corefer_only_inside_the_type() {
    let source = include_str!("fixtures/bootstrap/nexus.ethos");
    let reader = reader(vec![]);
    let plan = reader.plan(source).expect("Nexus plan");
    let (assignments, declaration_names) = assignments(&plan);
    let decoded = reader.seal(&plan, &assignments).expect("Nexus seal");
    let BootstrapBody::Nexus(body) = &decoded.document.body else {
        panic!("Nexus body")
    };
    assert_eq!(body.traits.len(), 2);
    assert_eq!(body.types.len(), 1);
    let Declaration::Type(pair) = &body.types[0] else {
        panic!("plain Pair")
    };
    let TypeBody::Struct(fields) = &pair.body else {
        panic!("Pair product")
    };
    let (TypeExpression::TraitRequirement(left), TypeExpression::TraitRequirement(right)) =
        (&fields[0], &fields[1])
    else {
        panic!("Trait requirements")
    };
    assert_eq!(left.parameter, right.parameter);
    let canonical = reader
        .write(&decoded, &names(declaration_names))
        .expect("canonical Nexus");
    assert!(canonical.contains("Pair.{«Left.Sortable» «Left.Sortable»}"));
}

#[test]
fn sema_tables_accept_only_persistent_nominal_leaves() {
    let source = include_str!("fixtures/bootstrap/sema.ethos");
    let external = TextualMetadataEntry {
        module_path: vec!["dependency".into(), "domain".into()],
        visible_name: "External".into(),
        encoded_name: id(80),
        class: core_ethos::bootstrap::NameClass::PersistentNominal,
    };
    let reader = reader(vec![external]);
    let plan = reader.plan(source).expect("Sema plan");
    let (assignments, _) = assignments(&plan);
    let decoded = reader.seal(&plan, &assignments).expect("Sema seal");
    let BootstrapBody::Sema(body) = decoded.document.body else {
        panic!("Sema body")
    };
    assert_eq!(body.record_types.len(), 2);
    assert_eq!(body.tables.len(), 1);
}

#[test]
fn seal_requires_no_missing_or_extra_assignment() {
    let reader = reader(vec![]);
    let plan = reader
        .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Thing.String]}")
        .expect("plan");
    let missing = NamingAssignments::new(vec![]).expect("empty set");
    assert!(matches!(
        reader.seal(&plan, &missing),
        Err(BootstrapReadError::MissingAssignment(0))
    ));

    let extra = NamingAssignments::new(vec![NamingAssignment {
        occurrence: plan.declarations()[0].occurrence(),
        encoded_name: id(200),
    }])
    .expect("one exact assignment");
    let decoded = reader.seal(&plan, &extra).expect("exact set succeeds");
    assert!(matches!(decoded.document.body, BootstrapBody::Interface(_)));

    let other_plan = reader
        .plan("Interface.{1 0 0}\n[]\n{[] [] [] [Other.String]}")
        .expect("separate plan");
    let with_extra = NamingAssignments::new(vec![
        NamingAssignment {
            occurrence: plan.declarations()[0].occurrence(),
            encoded_name: id(201),
        },
        NamingAssignment {
            occurrence: other_plan.declarations()[0].occurrence(),
            encoded_name: id(202),
        },
    ])
    .expect("distinct plan-local handles");
    assert!(matches!(
        reader.seal(&plan, &with_extra),
        Err(BootstrapReadError::ExtraAssignment(0))
    ));
}

#[test]
fn canonical_version_rejects_signs_leading_zero_and_wrong_arity() {
    let reader = reader(vec![]);
    for source in [
        "Interface.{01 0 0}\n[]\n{[] [] [] []}",
        "Interface.{1 0}\n[]\n{[] [] [] []}",
        "Interface.{1 -1 0}\n[]\n{[] [] [] []}",
    ] {
        assert!(reader.plan(source).is_err(), "{source}");
    }
}
