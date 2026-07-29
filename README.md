# core-ethos

`core-ethos` owns the stringless **Encoded** Ethos layer and its bidirectional
`TextualEthos` view. Its public model surface uses `Encoded*` names with no legacy
aliases or compatibility bridges.

## S1 slicing

- `Identifier` is a closed namespace variant with a namespace-local `u16`
  allocation. On the pre-identity compatibility pin, `core-ethos` owns the
  upstream `IdentifierNamespace::Schema` slice; it never
  reconstructs flat identifiers or converts between namespaces.
- A generic `NameTable` has the home namespace chosen by its owner.
  `core-ethos`-owned tables have that upstream home slice; a consumer composes completed
  foreign slices with `NameTable::compose`, which borrows them without copying,
  flattening, renumbering, or legacy fallback behavior.
- The existing positional field-name ban remains: fields are bare type references
  and equal field types are distinguished only by their position.
- `EncodedEthos` keeps ordered interface alternatives and carries closed
  `StreamingRelation` data without source-spelling or alias surfaces.

The universe bridge continues to derive positional constructor signatures from
Encoded layouts and validates authored structural tables against those signatures.
Names remain outside Encoded content identity, so a name-table change cannot alter
an Encoded value's content hash.

## Capsule carrier

`capsule_from_issued_hash` is the kind-fixed Ethos pass-through into
`protos::Capsule<protos::Ethos, Pin>`. The caller supplies both the
`ContentAddressedHash` and opaque complete NameTree pin. `core-ethos` does not
derive or verify their correspondence to an `EncodedEthos`, inspect or compose
the pin, or claim that its current flat `Identifier`/`NameTable` state is the
future nested-table chain.

Existing `EncodedEthos::content_identity` behavior remains the established
per-value API; the Capsule pass-through does not reinterpret or replace it.

## Dependency pins

The Capsule surface consumes immutable published revisions
`content-identity@f1f9c6efc828acaefd0f751550cd40389d312bf5` and
`protos@1435c9aeb7f24e811aca670101e355ff26818ae2`. The legacy flat name-table and
structural parsing graph, including its established per-value identity revision,
stays pinned to its existing revisions until the encodedID-chain migration.
Cargo names the new revision `capsule-content-identity` so its types cannot be
confused with that legacy graph; Cargo.lock records both exactly.

## Build and test

```sh
nix flake check --no-link --print-build-logs
cargo test
```
