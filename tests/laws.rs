//! The four conformance laws, exercised through structural-codec's source path.

use core_ethos::fixture::{COMMIT_SEQUENCE, DOCUMENTATION, FIELD, FLOAT, FixtureFamily};
use core_ethos::rules::EthosRule;
use name_table::{IdentifierNamespace, Name, NameTable};
use raw_discovery::Delimiter;
use structural_codec::{AddressedStructuralTable, ScopedEncodedTypeId, StructuralEvaluator};

fn standard_table() -> AddressedStructuralTable<EthosRule> {
    FixtureFamily::build()
        .standard_table()
        .expect("seal real-Encoded table")
}

#[test]
fn the_table_agrees_with_the_encoded_layout() {
    let family = FixtureFamily::build();
    let table = family.standard_table().expect("seal");
    family.universe().validate_table(&table).expect("agreement");
}

#[test]
fn law_one_round_trip_encoded() {
    let table = standard_table();
    let evaluator = StructuralEvaluator::new(&table).expect("evaluator");
    let cases: &[(ScopedEncodedTypeId, &str)] = &[
        (COMMIT_SEQUENCE, "CommitSequence.{ Integer }"),
        (FIELD, "Integer"),
        (DOCUMENTATION, "alpha.beta.gamma"),
        (FLOAT, "-122.3"),
    ];
    for (expected, source) in cases {
        let mut names = NameTable::new(IdentifierNamespace::Schema);
        let value = evaluator
            .decode_text(*expected, source, &mut names)
            .unwrap_or_else(|error| panic!("decode {source}: {error}"));
        let re_encoded = evaluator
            .encode_text(*expected, &value, &names)
            .unwrap_or_else(|error| panic!("encode {source}: {error}"));
        let mut names_again = NameTable::new(IdentifierNamespace::Schema);
        let value_again = evaluator
            .decode_text(*expected, &re_encoded, &mut names_again)
            .unwrap_or_else(|error| panic!("re-decode {source}: {error}"));
        assert_eq!(value, value_again, "law 1 for {source}");
    }
}

#[test]
fn law_two_round_trip_canonical() {
    let table = standard_table();
    let evaluator = StructuralEvaluator::new(&table).expect("evaluator");
    let cases: &[(ScopedEncodedTypeId, &str)] = &[
        (COMMIT_SEQUENCE, "CommitSequence.{Integer}"),
        (FIELD, "Integer"),
        (DOCUMENTATION, "alpha.beta.gamma"),
        (FLOAT, "-122.3"),
    ];
    for (expected, source) in cases {
        let mut names = NameTable::new(IdentifierNamespace::Schema);
        let value = evaluator
            .decode_text(*expected, source, &mut names)
            .unwrap_or_else(|error| panic!("decode {source}: {error}"));
        let encoded = evaluator
            .encode_text(*expected, &value, &names)
            .unwrap_or_else(|error| panic!("encode {source}: {error}"));
        assert_eq!(encoded, *source, "law 2 for {source}");
    }
}

#[test]
fn law_three_interning_atomicity() {
    let table = standard_table();
    let evaluator = StructuralEvaluator::new(&table).expect("evaluator");
    let mut names = NameTable::new(IdentifierNamespace::Schema);
    names
        .intern(Name::new("PriorName"))
        .expect("test name fits its namespace");
    let bytes_before = names.to_archive_bytes().expect("before").as_ref().to_vec();
    let identity_before = names.identity().expect("identity before");
    assert!(
        evaluator
            .decode_text(COMMIT_SEQUENCE, "notADeclaration", &mut names)
            .is_err()
    );
    let bytes_after = names.to_archive_bytes().expect("after").as_ref().to_vec();
    let identity_after = names.identity().expect("identity after");
    assert_eq!(bytes_before, bytes_after, "archived bytes unchanged");
    assert_eq!(
        identity_before, identity_after,
        "content identity unchanged"
    );
}

#[test]
fn law_four_identity_preserving_across_revisions() {
    let family = FixtureFamily::build();
    let table_old = family.table(Delimiter::Brace).expect("old table");
    let table_new = family.table(Delimiter::Parenthesis).expect("new table");
    assert_ne!(table_old.identity(), table_new.identity());
    let evaluator_old = StructuralEvaluator::new(&table_old).expect("old evaluator");
    let evaluator_new = StructuralEvaluator::new(&table_new).expect("new evaluator");
    let mut names_old = NameTable::new(IdentifierNamespace::Schema);
    let value_old = evaluator_old
        .decode_text(
            COMMIT_SEQUENCE,
            "CommitSequence.{ Integer }",
            &mut names_old,
        )
        .expect("decode old text with old table");
    let mut names_new = NameTable::new(IdentifierNamespace::Schema);
    let value_new = evaluator_new
        .decode_text(
            COMMIT_SEQUENCE,
            "CommitSequence.( Integer )",
            &mut names_new,
        )
        .expect("decode new text with new table");
    assert_eq!(value_old, value_new, "the structural value never moved");
    assert_eq!(
        value_old.content_identity().expect("identity old"),
        value_new.content_identity().expect("identity new"),
        "the value's content identity never moved"
    );
    let re_encoded = evaluator_new
        .encode_text(COMMIT_SEQUENCE, &value_old, &names_old)
        .expect("encode old value with new table");
    assert_eq!(re_encoded, "CommitSequence.(Integer)");
}
