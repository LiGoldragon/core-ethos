//! TextualEthos, the first real Textual form: real ethos TEXT decodes into real
//! EncodedEthos values with a real NameTable, and encodes back canonically. The
//! A struct field is a bare positional type reference — field names are illegal, so
//! same-typed fields are told apart by position alone against the real Encoded layout.

use core_ethos::TextualError;
use core_ethos::TextualEthos;
use core_ethos::declaration::EncodedType;
use core_ethos::fixture::{COMMIT_SEQUENCE, DATABASE_MARKER};
use core_ethos::reference::EncodedReference;
use name_table::{IdentifierNamespace, NameTable};
fn name_table_rows(names: &NameTable) -> String {
    (0..names.len())
        .map(|index| {
            let identifier = name_table::Identifier::Schema(u16::try_from(index).unwrap());
            format!(
                "  {index} -> {}",
                names.resolve(identifier).unwrap().as_str()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A newtype declaration decodes into a real `EncodedNewtype` and encodes back to the
/// identical canonical text.
#[test]
fn newtype_declaration_round_trips() {
    let textual = TextualEthos::fixture().expect("build textual ethos");
    let source = "CommitSequence.{ Integer }";
    let mut names = NameTable::new(IdentifierNamespace::Schema);

    let value = textual
        .decode(COMMIT_SEQUENCE, source, &mut names)
        .expect("decode CommitSequence");

    let EncodedType::Newtype(newtype) = &value else {
        panic!("expected a newtype, got {value:?}");
    };
    assert_eq!(
        names.resolve(newtype.identifier()).unwrap().as_str(),
        "CommitSequence"
    );
    assert_eq!(newtype.reference(), &EncodedReference::Integer);

    println!(
        "decoded {source}\n  => {value:?}\nNameTable:\n{}",
        name_table_rows(&names)
    );

    let re_encoded = textual
        .encode(COMMIT_SEQUENCE, &value, &mut names)
        .expect("encode CommitSequence");
    assert_eq!(
        re_encoded, "CommitSequence.{Integer}",
        "canonical text round-trips"
    );
    println!("re-encoded => {re_encoded}");
}

/// The `DatabaseMarker` struct decodes into a real `EncodedStruct` PURELY POSITIONALLY:
/// every field is a bare type reference and its name is DERIVED from that type, never
/// read from the text (field names are illegal, psyche ruling 2026-07-19). Its two
/// same-typed `StateDigest` fields therefore derive the SAME name `state_digest` and
/// are told apart by position alone. It encodes back to the identical canonical text.
#[test]
fn struct_declaration_round_trips_positionally() {
    let textual = TextualEthos::fixture().expect("build textual ethos");
    let source = "DatabaseMarker.{ CommitSequence StateDigest StateDigest }";
    let mut names = NameTable::new(IdentifierNamespace::Schema);

    let value = textual
        .decode(DATABASE_MARKER, source, &mut names)
        .expect("decode DatabaseMarker");

    let EncodedType::Struct(structure) = &value else {
        panic!("expected a struct, got {value:?}");
    };
    assert_eq!(
        names.resolve(structure.identifier()).unwrap().as_str(),
        "DatabaseMarker"
    );
    assert_eq!(structure.fields().len(), 3);

    let field_names: Vec<&str> = structure
        .fields()
        .iter()
        .map(|field| names.resolve(field.identifier()).unwrap().as_str())
        .collect();
    // Every name is derived from its type; the two `StateDigest` fields collide on
    // `state_digest` — position, not the name, distinguishes them.
    assert_eq!(
        field_names,
        vec!["commit_sequence", "state_digest", "state_digest"]
    );

    // Every field references a declared type by identifier (Plain), never a string.
    for field in structure.fields() {
        assert!(matches!(field.reference(), EncodedReference::Plain(_)));
    }

    println!(
        "decoded {source}\n  => {value:?}\nNameTable:\n{}",
        name_table_rows(&names)
    );

    let re_encoded = textual
        .encode(DATABASE_MARKER, &value, &mut names)
        .expect("encode DatabaseMarker");
    assert_eq!(
        re_encoded, "DatabaseMarker.{CommitSequence StateDigest StateDigest}",
        "canonical text round-trips"
    );
    println!("re-encoded => {re_encoded}");
}

/// Field names are illegal in every Protos surface (psyche ruling 2026-07-19: "field
/// names are now COMPLETLY ILLEGAL EVERYWHERE"). An explicit `name.Type` at a field
/// position — here `secretDigest.StateDigest` — no longer parses as a field: a field
/// is only the bare type standing at its position, so an application where a type atom
/// is expected has no valid alternative and decode rejects it. This holds for a name on
/// a same-typed field (the former "collision" case) exactly as for any other; there is
/// no longer any place a field name is legal.
#[test]
fn decode_rejects_explicit_field_name() {
    let textual = TextualEthos::fixture().expect("build textual ethos");
    let source = "DatabaseMarker.{ CommitSequence StateDigest secretDigest.StateDigest }";
    let mut names = NameTable::new(IdentifierNamespace::Schema);

    let error = textual
        .decode(DATABASE_MARKER, source, &mut names)
        .expect_err("an explicit field name is illegal Protos and must be rejected");

    assert!(
        matches!(error, TextualError::Decode(_)),
        "expected a structural decode rejection of the field-name application, got {error:?}"
    );
}
