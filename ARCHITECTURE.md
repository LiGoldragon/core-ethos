# Architecture — core-ethos

## One full-chain structural universe

`core-ethos` has one canonical `raw-discovery` dependency and one canonical
`structural-codec` dependency. The `whole` module defines the typed six-slot
document grammar with `OrderedProduct`, delegated item/reference rules, and
translator-issued `VocabularyEncodedId` chains.

The decoder accepts `DecodeNameBindings<VocabularyRoot>` as read-only authority
state. Declaration positions require `declaration_assignment`; references
require `reference_resolution`. The component never receives an allocation
capability.

The decoded value is `WholeEthos`, an ordered, positional carrier:

- newtypes retain visibility, declaration identity, and wrapped type;
- enumerations retain declaration and variant identities plus positional
  payloads;
- type references retain complete identity or application-head chains;
- fields have no authored names.

Archive restoration validates root and chain invariants before returning a
value. `WholeEthos` does not carry a composed NameTree pin and is not itself a
Capsule.

## Retained execution data boundary

`EncodedEthos` and its declaration/reference algebra predate the full-chain
identity model. They remain only because the current sealed execution engine
still consumes those values. They expose no textual decoder, structural table,
universe bridge, or `EncodedForm` implementation. This makes the phase boundary
honest: the flat values are execution data awaiting migration, not a second
authoring model.

The retired modules were `document`, `fixture`, `rules`, `textual`, and
`universe`. They depended on flat `ScopedEncodedTypeId`, local NameTable
allocation, and the structural-codec 0.6 descriptor model. Preserving them by
porting a duplicate full-chain grammar would have created a second authoring
surface beside `whole`.

## Capsule boundary

`capsule_from_issued_hash` fixes the outer kind to `protos::Ethos`. It passes
through a caller-issued content hash and opaque complete NameTree pin; it does
not derive their relationship or make the retained flat execution data a
full-chain carrier.
