use capsule_content_identity::ContentAddressedHash;
use protos::{Capsule, CapsuleIdentityVariant, Ethos};

#[derive(Debug, Eq, PartialEq)]
struct OpaqueCompleteNameTreePin([u8; 37]);

#[test]
fn caller_values_are_carried_in_an_ethos_kind_capsule() {
    let pin_bytes = [0xa7; 37];
    let capsule: Capsule<Ethos, OpaqueCompleteNameTreePin> = core_ethos::capsule_from_issued_hash(
        ContentAddressedHash::from_bytes([0x31; 32]),
        OpaqueCompleteNameTreePin(pin_bytes),
    );

    assert_eq!(
        capsule.content_identity().variant(),
        CapsuleIdentityVariant::Ethos
    );
    assert_eq!(capsule.complete_nametree_pin().0, pin_bytes);
}
