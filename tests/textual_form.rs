//! `TextualSchema` is the reference instance of the shared `TextualForm` textual
//! interface. This witness proves its generalized `view` / `unview` reproduce schema's
//! own single-declaration `encode` / `decode` byte-for-byte and value-for-value, on both
//! the newtype and struct golden. The interface was generalized out of schema, so
//! schema's existing behavior proves the shared view fits without change.

use core_schema::SchemaLanguage;
use core_schema::TextualSchema;
use core_schema::declaration::{EncodedNewtype, EncodedType};
use core_schema::fixture::{COMMIT_SEQUENCE, DATABASE_MARKER};
use core_schema::{EncodedReference, TextualError};
use name_table::{IdentifierNamespace, Name, NameTable};
use structural_codec::{Textual, TextualForm};

#[test]
fn view_and_unview_reproduce_encode_and_decode() {
    let goldens: [(_, &str); 2] = [
        (COMMIT_SEQUENCE, "CommitSequence.{ Integer }"),
        (
            DATABASE_MARKER,
            "DatabaseMarker.{ CommitSequence StateDigest StateDigest }",
        ),
    ];

    for (expected, source) in goldens {
        let textual = TextualSchema::fixture().expect("build textual schema");

        // The inherent single-declaration path (schema's own decode/encode).
        let mut inherent_names = NameTable::new(IdentifierNamespace::Schema);
        let decoded: EncodedType = textual
            .decode(expected, source, &mut inherent_names)
            .expect("inherent decode");
        let encoded: String = textual
            .encode(expected, &decoded, &mut inherent_names)
            .expect("inherent encode");

        // The shared textual interface uses the same structuretree and nametree.
        // Data crosses the boundary only as a `TextualForm<SchemaLanguage>` value.
        let mut textual_names = NameTable::new(IdentifierNamespace::Schema);
        let source_view: TextualForm<SchemaLanguage> = TextualForm::single(source.to_string());
        let unviewed: EncodedType = textual
            .unview(expected, &source_view, &mut textual_names)
            .expect("shared unview");
        let viewed_form: TextualForm<SchemaLanguage> = textual
            .view(expected, &unviewed, &textual_names)
            .expect("shared view");
        let viewed: String = viewed_form.sole_text().expect("sole view text").to_string();

        assert_eq!(decoded, unviewed, "unview reproduces decode for `{source}`");
        assert_eq!(encoded, viewed, "view reproduces encode for `{source}`");
        println!("witness `{source}` => shared textual view: {viewed}");
    }
}

fn newtype_value(names: &mut NameTable) -> EncodedType {
    let name = names
        .intern(Name::new("CommitSequence"))
        .expect("fixture name");
    EncodedType::Newtype(EncodedNewtype::new(name, EncodedReference::Integer))
}

#[test]
fn reflection_is_lookup_only_for_inherent_and_shared_textual_routes() {
    let textual = TextualSchema::fixture().expect("build textual schema");
    let mut names = NameTable::new(IdentifierNamespace::Schema);
    let value = newtype_value(&mut names);
    names
        .intern(Name::new("Integer"))
        .expect("preload scalar spelling");
    let bytes_before = names.to_archive_bytes().expect("before").as_ref().to_vec();
    let identity_before = names.identity().expect("identity before");

    let inherent = textual
        .encode(COMMIT_SEQUENCE, &value, &mut names)
        .expect("inherent encode");
    let shared = textual
        .view(COMMIT_SEQUENCE, &value, &names)
        .expect("shared view")
        .sole_text()
        .expect("one textual chunk")
        .to_owned();

    assert_eq!(inherent, shared, "direct and shared reflection agree");
    assert_eq!(inherent, "CommitSequence.{Integer}");
    assert_eq!(
        names.to_archive_bytes().expect("after").as_ref(),
        bytes_before.as_slice(),
        "inherent reflection does not mutate the archive"
    );
    assert_eq!(
        names.identity().expect("identity after"),
        identity_before,
        "inherent and shared reflection preserve identity"
    );
}

#[test]
fn reflection_reports_missing_spellings_without_mutating_names() {
    let textual = TextualSchema::fixture().expect("build textual schema");
    let mut names = NameTable::new(IdentifierNamespace::Schema);
    let value = newtype_value(&mut names);
    let bytes_before = names.to_archive_bytes().expect("before").as_ref().to_vec();
    let identity_before = names.identity().expect("identity before");

    let inherent = textual
        .encode(COMMIT_SEQUENCE, &value, &mut names)
        .expect_err("Integer is not preloaded");
    let shared = textual
        .view(COMMIT_SEQUENCE, &value, &names)
        .expect_err("Integer is not preloaded for shared view");

    assert!(matches!(
        inherent,
        TextualError::ReflectionNameAbsent {
            spelling: "Integer"
        }
    ));
    assert!(matches!(
        shared,
        TextualError::ReflectionNameAbsent {
            spelling: "Integer"
        }
    ));
    assert_eq!(
        names.to_archive_bytes().expect("after").as_ref(),
        bytes_before.as_slice(),
        "missing-name failure leaves the archive unchanged"
    );
    assert_eq!(names.identity().expect("identity after"), identity_before);
}
