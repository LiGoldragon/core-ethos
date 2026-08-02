use std::mem::{align_of, size_of};

use core_ethos::{
    WholeEthosAttributes, WholeEthosEnumeration, WholeEthosNewtype, WholeEthosTupleFields,
    WholeEthosTypeApplication, WholeEthosTypeReference, WholeEthosVariant,
    WholeEthosVariantPayload, WholeEthosVisibility, WholeEthosWrappedField,
};
use encoded_name_table::LocalEncodedId;
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};

#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
struct NamedWholeEthosNewtype {
    name: VocabularyEncodedId,
    visibility: WholeEthosVisibility,
    attributes: WholeEthosAttributes,
    wrapped_field: WholeEthosWrappedField,
}

#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
struct NamedWholeEthosWrappedField {
    visibility: WholeEthosVisibility,
    reference: WholeEthosTypeReference,
}

#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[rkyv(
    serialize_bounds(__S: rkyv::ser::Writer + rkyv::ser::Allocator, __S::Error: rkyv::rancor::Source),
    deserialize_bounds(__D::Error: rkyv::rancor::Source),
    bytecheck(bounds(__C: rkyv::validation::ArchiveContext, __C::Error: rkyv::rancor::Source)),
)]
struct NamedWholeEthosTypeApplication {
    head: VocabularyEncodedId,
    #[rkyv(omit_bounds)]
    payload: Box<WholeEthosTypeReference>,
}

#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
struct NamedWholeEthosEnumeration {
    name: VocabularyEncodedId,
    visibility: WholeEthosVisibility,
    attributes: WholeEthosAttributes,
    variants: Vec<WholeEthosVariant>,
}

#[derive(Clone, Debug, Eq, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
struct NamedWholeEthosVariant {
    name: VocabularyEncodedId,
    attributes: WholeEthosAttributes,
    payload: WholeEthosVariantPayload,
}

macro_rules! assert_archive_compatible {
    ($production_type:ty, $named_type:ty, $production:expr, $named:expr) => {{
        let production = $production;
        let named = $named;
        let production_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&production)
            .expect("archive production tuple carrier");
        let named_bytes =
            rkyv::to_bytes::<rkyv::rancor::Error>(&named).expect("archive named-field mirror");

        assert_eq!(
            production_bytes.as_slice(),
            named_bytes.as_slice(),
            "tuple and named-field carriers must emit identical bytes",
        );
        assert_eq!(
            size_of::<rkyv::Archived<$production_type>>(),
            size_of::<rkyv::Archived<$named_type>>(),
            "archived sizes must match",
        );
        assert_eq!(
            align_of::<rkyv::Archived<$production_type>>(),
            align_of::<rkyv::Archived<$named_type>>(),
            "archived alignments must match",
        );

        let _: &rkyv::Archived<$production_type> =
            rkyv::access::<rkyv::Archived<$production_type>, rkyv::rancor::Error>(&named_bytes)
                .expect("access named bytes through production archived layout");
        let _: &rkyv::Archived<$named_type> =
            rkyv::access::<rkyv::Archived<$named_type>, rkyv::rancor::Error>(&production_bytes)
                .expect("access production bytes through named archived layout");

        let production_from_named =
            rkyv::from_bytes::<$production_type, rkyv::rancor::Error>(&named_bytes)
                .expect("restore production carrier from named bytes");
        let named_from_production =
            rkyv::from_bytes::<$named_type, rkyv::rancor::Error>(&production_bytes)
                .expect("restore named carrier from production bytes");
        assert_eq!(production_from_named, production);
        assert_eq!(named_from_production, named);

        assert_eq!(
            rkyv::to_bytes::<rkyv::rancor::Error>(&production_from_named)
                .expect("reserialize production carrier restored from named bytes")
                .as_slice(),
            named_bytes.as_slice(),
        );
        assert_eq!(
            rkyv::to_bytes::<rkyv::rancor::Error>(&named_from_production)
                .expect("reserialize named carrier restored from production bytes")
                .as_slice(),
            production_bytes.as_slice(),
        );
    }};
}

// Trait exception — the proper trait cannot be determined: this function is an
// entry point whose contract is supplied by Rust's test harness.
#[test]
fn named_fields_preserve_every_whole_ethos_tuple_carrier_archive() {
    let encoded_id = |chain: &[u16]| {
        VocabularyEncodedId::new(
            VocabularyRoot::Universal,
            chain.iter().copied().map(LocalEncodedId::new).collect(),
        )
        .expect("nonempty fixture encoded ID")
    };

    let application_head = encoded_id(&[41, 3]);
    let application_payload = WholeEthosTypeReference::Identity(encoded_id(&[41, 5, 7]));
    assert_archive_compatible!(
        WholeEthosTypeApplication,
        NamedWholeEthosTypeApplication,
        WholeEthosTypeApplication::new(application_head.clone(), application_payload.clone()),
        NamedWholeEthosTypeApplication {
            head: application_head,
            payload: Box::new(application_payload),
        }
    );

    let wrapped_reference = WholeEthosTypeReference::Application(WholeEthosTypeApplication::new(
        encoded_id(&[43, 11]),
        WholeEthosTypeReference::Identity(encoded_id(&[43, 13, 17])),
    ));
    assert_archive_compatible!(
        WholeEthosWrappedField,
        NamedWholeEthosWrappedField,
        WholeEthosWrappedField::new(WholeEthosVisibility::Private, wrapped_reference.clone()),
        NamedWholeEthosWrappedField {
            visibility: WholeEthosVisibility::Private,
            reference: wrapped_reference.clone(),
        }
    );

    let newtype_name = encoded_id(&[47, 19]);
    let wrapped_field =
        WholeEthosWrappedField::new(WholeEthosVisibility::Private, wrapped_reference);
    assert_archive_compatible!(
        WholeEthosNewtype,
        NamedWholeEthosNewtype,
        WholeEthosNewtype::new(
            newtype_name.clone(),
            WholeEthosVisibility::Public,
            WholeEthosAttributes::empty(),
            wrapped_field.clone(),
        ),
        NamedWholeEthosNewtype {
            name: newtype_name,
            visibility: WholeEthosVisibility::Public,
            attributes: WholeEthosAttributes::empty(),
            wrapped_field,
        }
    );

    let variant_name = encoded_id(&[53, 23, 29]);
    let variant_payload = WholeEthosVariantPayload::Tuple(
        WholeEthosTupleFields::new(vec![
            WholeEthosTypeReference::Identity(encoded_id(&[53, 31])),
            WholeEthosTypeReference::Application(WholeEthosTypeApplication::new(
                encoded_id(&[53, 37]),
                WholeEthosTypeReference::Identity(encoded_id(&[53, 41, 43])),
            )),
        ])
        .expect("nonempty variant payload"),
    );
    assert_archive_compatible!(
        WholeEthosVariant,
        NamedWholeEthosVariant,
        WholeEthosVariant::new(
            variant_name.clone(),
            WholeEthosAttributes::empty(),
            variant_payload.clone(),
        ),
        NamedWholeEthosVariant {
            name: variant_name.clone(),
            attributes: WholeEthosAttributes::empty(),
            payload: variant_payload.clone(),
        }
    );

    let variants = vec![
        WholeEthosVariant::new(
            encoded_id(&[59, 47]),
            WholeEthosAttributes::empty(),
            WholeEthosVariantPayload::Unit,
        ),
        WholeEthosVariant::new(variant_name, WholeEthosAttributes::empty(), variant_payload),
    ];
    let enumeration_name = encoded_id(&[59, 61]);
    assert_archive_compatible!(
        WholeEthosEnumeration,
        NamedWholeEthosEnumeration,
        WholeEthosEnumeration::new(
            enumeration_name.clone(),
            WholeEthosVisibility::Private,
            WholeEthosAttributes::empty(),
            variants.clone(),
        ),
        NamedWholeEthosEnumeration {
            name: enumeration_name,
            visibility: WholeEthosVisibility::Private,
            attributes: WholeEthosAttributes::empty(),
            variants,
        }
    );
}
