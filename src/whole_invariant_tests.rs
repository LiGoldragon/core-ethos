use encoded_name_table::LocalEncodedId;

use super::*;

fn identity(local: u16) -> VocabularyEncodedId {
    VocabularyEncodedId::new(VocabularyRoot::Universal, vec![LocalEncodedId::new(local)])
        .expect("test identity is complete")
}

fn interface_document(item: WholeEthosItem) -> WholeEthos {
    WholeEthos {
        header: WholeEthosHeader {
            kind: WholeEthosFileKind::Interface,
            version: SUPPORTED_VERSION,
        },
        body: WholeEthosBody::Interface(WholeEthosInterfaceBody {
            inputs: vec![],
            outputs: vec![],
            refusals: vec![],
            types: vec![item],
        }),
    }
}

macro_rules! assert_serialization_and_restore_reject {
    ($value:expr, $pattern:pat) => {{
        let value = $value;
        assert!(matches!(value.to_archive_bytes(), Err($pattern)));
        let unchecked = <WholeEthos as PortableArchive>::to_archive_bytes(&value)
            .expect("raw archive can carry the deliberately invalid test value");
        assert!(matches!(
            WholeEthos::from_archive_bytes(unchecked.as_ref()),
            Err($pattern)
        ));
    }};
}

#[test]
fn serialization_and_restore_enforce_supported_version_and_matching_kind() {
    let interface = WholeEthosInterfaceBody::new(vec![], vec![], vec![], vec![]);
    assert_serialization_and_restore_reject!(
        WholeEthos {
            header: WholeEthosHeader {
                kind: WholeEthosFileKind::Interface,
                version: 2,
            },
            body: WholeEthosBody::Interface(interface.clone()),
        },
        WholeEthosArchiveError::UnsupportedVersion {
            kind: WholeEthosFileKind::Interface,
            found: 2,
            supported: SUPPORTED_VERSION,
        }
    );
    assert_serialization_and_restore_reject!(
        WholeEthos {
            header: WholeEthosHeader {
                kind: WholeEthosFileKind::Nexus,
                version: SUPPORTED_VERSION,
            },
            body: WholeEthosBody::Interface(interface),
        },
        WholeEthosArchiveError::HeaderBodyKindMismatch {
            header: WholeEthosFileKind::Nexus,
            body: WholeEthosFileKind::Interface,
        }
    );
}

#[test]
fn serialization_and_restore_reject_all_empty_grammar_cardinalities() {
    assert_serialization_and_restore_reject!(
        interface_document(WholeEthosItem::Struct(WholeEthosStruct {
            name: identity(1),
            fields: vec![],
        })),
        WholeEthosArchiveError::EmptyStructFields
    );
    assert_serialization_and_restore_reject!(
        interface_document(WholeEthosItem::Enumeration(WholeEthosEnumeration {
            name: identity(2),
            visibility: WholeEthosVisibility::Public,
            attributes: WholeEthosAttributes::empty(),
            variants: vec![],
        })),
        WholeEthosArchiveError::EmptyEnumerationVariants
    );
    assert_serialization_and_restore_reject!(
        interface_document(WholeEthosItem::Enumeration(WholeEthosEnumeration {
            name: identity(3),
            visibility: WholeEthosVisibility::Public,
            attributes: WholeEthosAttributes::empty(),
            variants: vec![WholeEthosVariant {
                name: identity(4),
                attributes: WholeEthosAttributes::empty(),
                payload: WholeEthosVariantPayload::Tuple(WholeEthosTupleFields(vec![])),
            }],
        })),
        WholeEthosArchiveError::EmptyTupleFields
    );
}
