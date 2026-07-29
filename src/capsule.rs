//! The kind-fixed Ethos Capsule carrier boundary.

use capsule_content_identity::ContentAddressedHash;
use protos::{Capsule, Ethos};

/// Carry a caller-issued hash and caller-supplied complete NameTree pin in an
/// Ethos-kind Capsule.
///
/// This is a pass-through constructor. It does not derive the hash from an
/// [`EncodedEthos`](crate::EncodedEthos), compare the hash with component
/// content, inspect or compose the pin, or verify that the pin is complete.
/// Module-table composition and the correspondence between this carrier and an
/// `EncodedEthos` value remain unwired.
pub fn capsule_from_issued_hash<CompleteNameTreePin>(
    hash: ContentAddressedHash,
    complete_nametree_pin: CompleteNameTreePin,
) -> Capsule<Ethos, CompleteNameTreePin> {
    Capsule::new(hash, complete_nametree_pin)
}
