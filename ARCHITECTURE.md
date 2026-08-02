# Architecture — core-ethos

## One full-chain structural universe

`core-ethos` has one canonical `raw-discovery` dependency and one canonical
`structural-codec` dependency. The `whole` module defines one composite document
shape: header, imports, body. Its `OrderedSequence` admits the bare header and
the two following delimited objects without a positional source splitter.

The codec accepts `DecodeNameBindings<VocabularyRoot>` as read-only authority
state. Declaration positions require `declaration_assignment`; references
require `reference_resolution`. The component never receives an allocation
capability.

Decode is two-phase. The shared evaluator decodes `Kind.Version`; a registry of
`EthosFileRoot` implementations selects the addressed document root; then the
same evaluator decodes the complete document. Each kind contributes a body root
record and a small trait implementation rather than a parser. Interface, Nexus,
and Sema body records all delegate through the same declaration, reference,
application, list, and product rules.

The decode result retains the evaluator-produced structural mirror and exact
textual projection at runtime. Encoding first proves that the mirror remains
renderable through the same sealed table and selected root, then returns the
retained projection so layout-sensitive reviewed fixtures remain byte-identical.
Imports live only in this textual projection and are absent from `WholeEthos`
archives.

The encoded value is `WholeEthos`, a header plus one selected body:

- newtypes retain visibility, declaration identity, and wrapped type;
- enumerations retain declaration and variant identities plus positional
  payloads;
- structs retain only ordered type references, never authored field labels;
- object-first operator applications retain operator, authored name, and
  positional payload without assigning Stream semantics;
- traits retain methods whose last positional type is the explicit return;
- tables retain record and key type positions while their section supplies the
  table operator;
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
