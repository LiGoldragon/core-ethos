//! Authority assignments preserve the complete supplied Schema NameTable and every
//! encoded identifier. The bridge never resolves a name merely to re-intern it into
//! another namespace.

use content_identity::PortableArchive;
use core_ethos::declaration::{EncodedEnum, EncodedField, EncodedStruct, EncodedType};
use core_ethos::{
    AssignedKind, AssignedMember, BuiltinReference, EncodedDeclaration, EncodedNewtype,
    EncodedReference, EncodedUniverse, EncodedUniverseBuilder, ScalarSlot,
    SingleTypeReferenceProjection, StructuralRedefinition, UniverseError,
};
use name_table::{Identifier, IdentifierNamespace, Name, NameTable};
use structural_codec::{EncodedLanguage, ScopedEncodedTypeId};

fn ethos_id(local: u16) -> ScopedEncodedTypeId {
    ScopedEncodedTypeId::schema(local)
}

fn ethos_table(names: &[&str]) -> (NameTable, Vec<Identifier>) {
    let mut table = NameTable::new(IdentifierNamespace::Schema);
    let identifiers = names
        .iter()
        .map(|name| table.intern(Name::new(*name)).expect("fixture fits"))
        .collect();
    (table, identifiers)
}

/// The authority boundary transfers its complete table and stored identifiers
/// verbatim. In particular, declarations, field names, and Plain targets are not
/// converted by resolving their spelling.
#[test]
fn authority_assignment_preserves_ethos_identifiers_and_complete_table() {
    let (names, identifiers) = ethos_table(&["Record", "label", "Target", "Integer"]);
    let [record, label, target, integer] = identifiers.as_slice() else {
        panic!("fixture identifiers")
    };
    let declaration = EncodedDeclaration::public(EncodedType::Struct(EncodedStruct::new(
        *record,
        vec![EncodedField::new(*label, EncodedReference::Plain(*target))],
    )));
    let target_declaration = EncodedDeclaration::public(EncodedType::Newtype(EncodedNewtype::new(
        *target,
        EncodedReference::Integer,
    )));

    let universe = EncodedUniverse::from_assignment(
        EncodedLanguage::Schema,
        vec![
            AssignedMember::new(9, *target, AssignedKind::Declaration(target_declaration)),
            AssignedMember::new(3, *record, AssignedKind::Declaration(declaration.clone())),
            AssignedMember::new(
                1,
                *integer,
                AssignedKind::ScalarPrimitive(ScalarSlot::Integer),
            ),
        ],
        names,
    )
    .expect("Schema-home assignment is accepted");

    let stored = universe
        .encoded_type(ethos_id(3))
        .expect("record declaration");
    assert_eq!(
        stored,
        declaration.value(),
        "stored declaration is unmodified"
    );
    assert_eq!(
        universe.names().resolve(*target).unwrap().as_str(),
        "Target",
        "the supplied Schema home slice moved intact",
    );
}

/// A completed foreign slice remains borrowed by the moved Schema-home table; it is
/// not copied, flattened, or renumbered while the EncodedEthos member retains its own
/// Schema identifier.
#[test]
fn assignment_transfers_complete_composed_name_table() {
    let mut logos = NameTable::new(IdentifierNamespace::Logos);
    let foreign = logos.intern(Name::new("LogosToken")).expect("fixture fits");
    let mut ethos = NameTable::new(IdentifierNamespace::Schema);
    let record = ethos.intern(Name::new("Record")).expect("fixture fits");
    let integer = ethos.intern(Name::new("Integer")).expect("fixture fits");
    let composed = ethos.compose(&logos).expect("borrow Logos slice");

    let universe = EncodedUniverse::from_assignment(
        EncodedLanguage::Schema,
        vec![
            AssignedMember::new(
                0,
                record,
                AssignedKind::Declaration(EncodedDeclaration::public(EncodedType::Newtype(
                    EncodedNewtype::new(record, EncodedReference::Integer),
                ))),
            ),
            AssignedMember::new(
                1,
                integer,
                AssignedKind::ScalarPrimitive(ScalarSlot::Integer),
            ),
        ],
        composed,
    )
    .expect("Schema member with borrowed foreign slice is valid");

    assert_eq!(
        universe.names().resolve(foreign).unwrap().as_str(),
        "LogosToken",
        "the complete borrowed Logos slice is retained"
    );
    assert_eq!(foreign, Identifier::Logos(0));
}

/// A Logos identifier remains Logos even in a composed Schema table. The authority
/// boundary rejects it rather than turning its spelling into a Schema identifier.
#[test]
fn logos_identifier_is_never_silently_converted_to_ethos() {
    let mut logos = NameTable::new(IdentifierNamespace::Logos);
    let logos_record = logos.intern(Name::new("Record")).expect("fixture fits");
    assert_eq!(logos_record, Identifier::Logos(0));

    let ethos = NameTable::new(IdentifierNamespace::Schema);
    let composed = ethos.compose(&logos).expect("borrow Logos slice");
    let declaration = EncodedDeclaration::public(EncodedType::Newtype(EncodedNewtype::new(
        logos_record,
        EncodedReference::Integer,
    )));
    let error = EncodedUniverse::from_assignment(
        EncodedLanguage::Schema,
        vec![AssignedMember::new(
            0,
            logos_record,
            AssignedKind::Declaration(declaration),
        )],
        composed,
    )
    .expect_err("foreign Encoded identifier is rejected");

    assert!(matches!(
        error,
        UniverseError::WrongEthosIdentifier(Identifier::Logos(0))
    ));
}

#[test]
fn non_ethos_name_table_home_is_rejected() {
    let mut logos = NameTable::new(IdentifierNamespace::Logos);
    let identifier = logos.intern(Name::new("Record")).expect("fixture fits");
    let error = EncodedUniverse::from_assignment(
        EncodedLanguage::Schema,
        vec![AssignedMember::new(
            0,
            identifier,
            AssignedKind::LeafPrimitive,
        )],
        logos,
    )
    .expect_err("EncodedEthos owns a Schema-home table");
    assert!(matches!(
        error,
        UniverseError::WrongNameTableHome {
            actual: IdentifierNamespace::Logos
        }
    ));
}

#[test]
fn declaration_identifier_must_match_assigned_identifier() {
    let (names, identifiers) = ethos_table(&["Assigned", "Stored"]);
    let error = EncodedUniverse::from_assignment(
        EncodedLanguage::Schema,
        vec![AssignedMember::new(
            0,
            identifiers[0],
            AssignedKind::Declaration(EncodedDeclaration::public(EncodedType::Newtype(
                EncodedNewtype::new(identifiers[1], EncodedReference::Integer),
            ))),
        )],
        names,
    )
    .expect_err("mismatched authority and declaration identities are rejected");
    assert!(matches!(
        error,
        UniverseError::AssignedDeclarationIdentifierMismatch { .. }
    ));
}

#[test]
fn duplicate_assigned_identity_is_rejected() {
    let (names, identifiers) = ethos_table(&["Alpha", "Beta"]);
    let clash = EncodedUniverse::from_assignment(
        EncodedLanguage::Schema,
        vec![
            AssignedMember::new(3, identifiers[0], AssignedKind::LeafPrimitive),
            AssignedMember::new(3, identifiers[1], AssignedKind::LeafPrimitive),
        ],
        names,
    );
    assert!(matches!(
        clash,
        Err(UniverseError::DuplicateMemberIdentity(id)) if id == ethos_id(3)
    ));
}

/// The universal seal checks every builder path before any registry map exists:
/// NameTable home and resolution, Schema ownership, and both registry keys cannot
/// be bypassed by direct builder use.
#[test]
fn direct_builder_seal_rejects_wrong_home_foreign_unresolved_and_duplicate_members() {
    let wrong_home =
        EncodedUniverseBuilder::from_name_table(NameTable::new(IdentifierNamespace::Logos))
            .build(EncodedLanguage::Schema);
    assert!(matches!(
        wrong_home,
        Err(UniverseError::WrongNameTableHome {
            actual: IdentifierNamespace::Logos
        })
    ));

    let mut foreign_builder = EncodedUniverseBuilder::new();
    foreign_builder.primitive_at(ethos_id(0), Identifier::Logos(0), ScalarSlot::Integer);
    assert!(matches!(
        foreign_builder.build(EncodedLanguage::Schema),
        Err(UniverseError::WrongEthosIdentifier(Identifier::Logos(0)))
    ));

    let mut unresolved_builder = EncodedUniverseBuilder::new();
    unresolved_builder.leaf_at(ethos_id(0), Identifier::Schema(99));
    assert!(matches!(
        unresolved_builder.build(EncodedLanguage::Schema),
        Err(UniverseError::Names(_))
    ));

    let mut duplicate_id_builder = EncodedUniverseBuilder::new();
    let alpha = duplicate_id_builder.intern("Alpha").unwrap();
    let beta = duplicate_id_builder.intern("Beta").unwrap();
    let duplicate_id = ethos_id(0);
    duplicate_id_builder.leaf_at(duplicate_id, alpha);
    duplicate_id_builder.leaf_at(duplicate_id, beta);
    assert!(matches!(
        duplicate_id_builder.build(EncodedLanguage::Schema),
        Err(UniverseError::DuplicateMemberIdentity(id)) if id == duplicate_id
    ));

    let mut duplicate_name_builder = EncodedUniverseBuilder::new();
    let alpha = duplicate_name_builder.intern("Alpha").unwrap();
    duplicate_name_builder.leaf_at(ethos_id(0), alpha);
    duplicate_name_builder.leaf_at(ethos_id(1), alpha);
    assert!(matches!(
        duplicate_name_builder.build(EncodedLanguage::Schema),
        Err(UniverseError::DuplicateMemberName(name)) if name == alpha
    ));
}

#[test]
fn direct_builder_seal_rejects_member_from_another_language() {
    let expected = EncodedLanguage::Schema;
    let actual = EncodedLanguage::Logos;
    let mut builder = EncodedUniverseBuilder::new();
    let alpha = builder.intern("Alpha").unwrap();
    let foreign_member = ScopedEncodedTypeId::logos(0);
    builder.leaf_at(foreign_member, alpha);

    assert!(matches!(
        builder.build(expected),
        Err(UniverseError::UniverseScopeMismatch {
            expected: mismatch_expected,
            actual: mismatch_actual,
            member,
        }) if mismatch_expected == expected && mismatch_actual == actual && member == foreign_member
    ));
}

#[test]
fn direct_builder_seal_rejects_nested_plain_reference_from_another_language() {
    let expected = EncodedLanguage::Schema;
    let actual = EncodedLanguage::Logos;
    let mut builder = EncodedUniverseBuilder::new();
    let record = builder.intern("Record").unwrap();
    let target = builder.intern("Target").unwrap();
    let foreign_target = ScopedEncodedTypeId::logos(1);
    builder.declaration(
        ethos_id(0),
        EncodedDeclaration::public(EncodedType::Newtype(EncodedNewtype::new(
            record,
            EncodedReference::SingleTypeApplication {
                projection: SingleTypeReferenceProjection::Optional,
                argument: Box::new(EncodedReference::Plain(target)),
            },
        ))),
    );
    builder.leaf_at(foreign_target, target);

    assert!(matches!(
        builder.build(expected),
        Err(UniverseError::UniverseScopeMismatch {
            expected: mismatch_expected,
            actual: mismatch_actual,
            member,
        }) if mismatch_expected == expected && mismatch_actual == actual && member == foreign_target
    ));
}

#[test]
fn direct_builder_seal_rejects_nested_scalar_reference_from_another_language() {
    let expected = EncodedLanguage::Schema;
    let actual = EncodedLanguage::Logos;
    let mut builder = EncodedUniverseBuilder::new();
    let record = builder.intern("Record").unwrap();
    let integer = builder.intern("Integer").unwrap();
    let foreign_integer = ScopedEncodedTypeId::logos(1);
    builder.declaration(
        ethos_id(0),
        EncodedDeclaration::public(EncodedType::Newtype(EncodedNewtype::new(
            record,
            EncodedReference::SingleTypeApplication {
                projection: SingleTypeReferenceProjection::Optional,
                argument: Box::new(EncodedReference::Integer),
            },
        ))),
    );
    builder.primitive_at(foreign_integer, integer, ScalarSlot::Integer);

    assert!(matches!(
        builder.build(expected),
        Err(UniverseError::UniverseScopeMismatch {
            expected: mismatch_expected,
            actual: mismatch_actual,
            member,
        }) if mismatch_expected == expected && mismatch_actual == actual && member == foreign_integer
    ));
}

#[test]
fn direct_builder_seal_rejects_an_absent_scalar_slot() {
    let language = EncodedLanguage::Schema;
    let mut builder = EncodedUniverseBuilder::new();
    let record = builder.intern("Record").unwrap();
    builder.declaration(
        ethos_id(0),
        EncodedDeclaration::public(EncodedType::Newtype(EncodedNewtype::new(
            record,
            EncodedReference::Integer,
        ))),
    );

    assert!(matches!(
        builder.build(language),
        Err(UniverseError::MissingScalarSlot {
            slot: ScalarSlot::Integer,
            reference: EncodedReference::Integer,
        })
    ));
}

#[test]
fn direct_builder_seal_rejects_a_name_table_only_plain_target() {
    let language = EncodedLanguage::Schema;
    let mut builder = EncodedUniverseBuilder::new();
    let record = builder.intern("Record").unwrap();
    let target = builder.intern("TableOnlyTarget").unwrap();
    builder.declaration(
        ethos_id(0),
        EncodedDeclaration::public(EncodedType::Newtype(EncodedNewtype::new(
            record,
            EncodedReference::Plain(target),
        ))),
    );

    assert!(matches!(
        builder.build(language),
        Err(UniverseError::ReferenceTargetUnregistered {
            identifier,
            reference: EncodedReference::Plain(reference),
        }) if identifier == target && reference == target
    ));
}

#[test]
fn direct_builder_seal_rejects_nested_missing_scalar_and_member_references() {
    let language = EncodedLanguage::Schema;
    let mut scalar_builder = EncodedUniverseBuilder::new();
    let record = scalar_builder.intern("Record").unwrap();
    scalar_builder.declaration(
        ethos_id(0),
        EncodedDeclaration::public(EncodedType::Newtype(EncodedNewtype::new(
            record,
            EncodedReference::SingleTypeApplication {
                projection: SingleTypeReferenceProjection::Optional,
                argument: Box::new(EncodedReference::Integer),
            },
        ))),
    );
    assert!(matches!(
        scalar_builder.build(language),
        Err(UniverseError::MissingScalarSlot {
            slot: ScalarSlot::Integer,
            reference: EncodedReference::Integer,
        })
    ));

    let mut member_builder = EncodedUniverseBuilder::new();
    let record = member_builder.intern("Record").unwrap();
    let target = member_builder.intern("TableOnlyTarget").unwrap();
    member_builder.declaration(
        ethos_id(0),
        EncodedDeclaration::public(EncodedType::Newtype(EncodedNewtype::new(
            record,
            EncodedReference::SingleTypeApplication {
                projection: SingleTypeReferenceProjection::Optional,
                argument: Box::new(EncodedReference::Plain(target)),
            },
        ))),
    );
    assert!(matches!(
        member_builder.build(language),
        Err(UniverseError::ReferenceTargetUnregistered {
            identifier,
            reference: EncodedReference::Plain(reference),
        }) if identifier == target && reference == target
    ));
}

#[test]
fn direct_builder_seal_resolves_registered_scalar_and_plain_targets() {
    let language = EncodedLanguage::Schema;
    let mut builder = EncodedUniverseBuilder::new();
    let record = builder.intern("Record").unwrap();
    let target = builder.intern("Target").unwrap();
    let integer = builder.intern("Integer").unwrap();
    let target_id = ethos_id(1);
    let integer_id = ethos_id(2);
    builder.declaration(
        ethos_id(0),
        EncodedDeclaration::public(EncodedType::Newtype(EncodedNewtype::new(
            record,
            EncodedReference::Plain(target),
        ))),
    );
    builder.declaration(
        target_id,
        EncodedDeclaration::public(EncodedType::Newtype(EncodedNewtype::new(
            target,
            EncodedReference::Integer,
        ))),
    );
    builder.primitive_at(integer_id, integer, ScalarSlot::Integer);

    let universe = builder
        .build(language)
        .expect("registered scalar and member references satisfy the seal");
    assert_eq!(
        universe
            .resolve_reference(&EncodedReference::Plain(target))
            .unwrap(),
        target_id
    );
    assert_eq!(
        universe
            .resolve_reference(&EncodedReference::Integer)
            .unwrap(),
        integer_id
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BuiltinWitnessMember {
    Scalar(ScalarSlot),
    LeafPrimitive,
    FieldMeta,
    Declaration,
}

impl BuiltinWitnessMember {
    const ALL: [Self; 7] = [
        Self::Scalar(ScalarSlot::Integer),
        Self::Scalar(ScalarSlot::Text),
        Self::Scalar(ScalarSlot::Boolean),
        Self::Scalar(ScalarSlot::Bytes),
        Self::LeafPrimitive,
        Self::FieldMeta,
        Self::Declaration,
    ];
}

fn builtin_scalar_slot(builtin: BuiltinReference) -> Option<ScalarSlot> {
    match builtin {
        BuiltinReference::Integer => Some(ScalarSlot::Integer),
        BuiltinReference::String => Some(ScalarSlot::Text),
        BuiltinReference::Boolean => Some(ScalarSlot::Boolean),
        BuiltinReference::Bytes => Some(ScalarSlot::Bytes),
        BuiltinReference::Vector | BuiltinReference::Optional | BuiltinReference::ScopeOf => None,
    }
}

fn is_sanctioned_builtin_member(builtin: BuiltinReference, member: BuiltinWitnessMember) -> bool {
    matches!(member, BuiltinWitnessMember::Scalar(slot) if Some(slot) == builtin_scalar_slot(builtin))
}

fn builtin_declaration(identifier: Identifier) -> EncodedDeclaration {
    EncodedDeclaration::public(EncodedType::Enumeration(EncodedEnum::new(
        identifier,
        Vec::new(),
    )))
}

fn assert_archiveable_redefinition(
    result: Result<EncodedUniverse, UniverseError>,
    identifier: Identifier,
    builtin: BuiltinReference,
) {
    let UniverseError::Redefinition(redefinition) =
        result.expect_err("a nonsanctioned builtin member cannot seal")
    else {
        panic!(
            "{} must reject with StructuralRedefinition",
            builtin.spelling()
        );
    };
    assert_eq!(redefinition.identifier(), identifier);
    assert_eq!(redefinition.builtin(), builtin);
    let bytes = redefinition
        .to_archive_bytes()
        .expect("archive typed redefinition");
    assert_eq!(
        StructuralRedefinition::from_archive_bytes(&bytes).expect("load typed redefinition"),
        redefinition,
    );
}

fn direct_builder_builtin_member(
    language: EncodedLanguage,
    builtin: BuiltinReference,
    member: BuiltinWitnessMember,
) -> (Identifier, Result<EncodedUniverse, UniverseError>) {
    let mut builder = EncodedUniverseBuilder::new();
    let identifier = builder.intern(builtin.spelling()).expect("intern builtin");
    let id = ethos_id(0);
    match member {
        BuiltinWitnessMember::Scalar(slot) => builder.primitive_at(id, identifier, slot),
        BuiltinWitnessMember::LeafPrimitive => builder.leaf_at(id, identifier),
        BuiltinWitnessMember::FieldMeta => builder.field_meta_at(id, identifier),
        BuiltinWitnessMember::Declaration => {
            builder.declaration(id, builtin_declaration(identifier))
        }
    }
    (identifier, builder.build(language))
}

fn assigned_builtin_member(identifier: Identifier, member: BuiltinWitnessMember) -> AssignedMember {
    let kind = match member {
        BuiltinWitnessMember::Scalar(slot) => AssignedKind::ScalarPrimitive(slot),
        BuiltinWitnessMember::LeafPrimitive => AssignedKind::LeafPrimitive,
        BuiltinWitnessMember::FieldMeta => AssignedKind::FieldMeta,
        BuiltinWitnessMember::Declaration => {
            AssignedKind::Declaration(builtin_declaration(identifier))
        }
    };
    AssignedMember::new(0, identifier, kind)
}

/// A builtin is a mandatory prior definition at the direct-builder seal. The only
/// admitted member under a scalar builtin spelling is its matching scalar-slot
/// realization; every other member is archiveable typed rejection data.
#[test]
fn direct_builder_rejects_every_builtin_as_an_archiveable_redefinition() {
    assert_eq!(
        BuiltinReference::ALL.len(),
        7,
        "the builtin lexicon is exhaustive"
    );
    let language = EncodedLanguage::Schema;

    for builtin in BuiltinReference::ALL {
        for member in BuiltinWitnessMember::ALL {
            let (identifier, result) = direct_builder_builtin_member(language, builtin, member);
            if is_sanctioned_builtin_member(builtin, member) {
                result.expect("the matching scalar-slot builtin realization seals");
            } else {
                assert_archiveable_redefinition(result, identifier, builtin);
            }
        }
    }
}

/// Authority-provided construction shares the exact same mandatory builtin-prior
/// seal as direct construction. Sorting or externally assigning locals cannot bypass
/// the standard-universe definitions.
#[test]
fn from_assignment_rejects_every_builtin_as_an_archiveable_redefinition() {
    let language = EncodedLanguage::Schema;

    for builtin in BuiltinReference::ALL {
        for member in BuiltinWitnessMember::ALL {
            let (names, identifiers) = ethos_table(&[builtin.spelling()]);
            let identifier = identifiers[0];
            let result = EncodedUniverse::from_assignment(
                language,
                vec![assigned_builtin_member(identifier, member)],
                names,
            );
            if is_sanctioned_builtin_member(builtin, member) {
                result.expect("the matching scalar-slot builtin realization seals");
            } else {
                assert_archiveable_redefinition(result, identifier, builtin);
            }
        }
    }
}
