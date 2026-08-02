# core-ethos

`core-ethos` owns the stringless encoded Ethos carriers.

## Canonical textual and structural surface

The `whole` module is the one production textual/structural path. Its
`EthosCodec` decodes Interface, Nexus, and Sema files through `raw-discovery`
and `structural-codec` using complete `VocabularyEncodedId` chains:

- declaration positions consume assignments already issued by the translator;
- reference positions perform lookup only;
- fields are positional;
- every document is a header, imports object, and kind-selected body;
- the header is decoded first and its kind selects one addressed document root;
- Interface bodies carry inputs, outputs, refusals, and shared types;
- Nexus bodies carry operand types and trait method signatures;
- Sema bodies carry record types and `table.{Record Key}` declarations;
- imports are parsed but retained only with the textual projection;
- the structural mirror is verified through the same evaluator before exact
  source formatting is re-emitted;
- no local name or identity allocation exists;
- `WholeEthos` is archivable and validates every complete chain on restore.

The grammar identities and builtin reference/operator priors are caller-supplied
typed data. Unknown file kinds and unsupported versions refuse through typed
errors. The codec uses the Standard discovery profile and the canonical
`structural-codec` dependency only.

## Retained sealed declaration data

The older `EncodedEthos` declaration algebra remains because the current sealed
Nomos execution engine consumes it. It carries flat `Identifier` values and is
not an authoring, textual, or structural-codec surface. Its former flat
`TextualEthos`, structural fixture, and universe bridge were retired when the
crate converged on the full-chain structural contract.

## Capsule carrier

`capsule_from_issued_hash` fixes the outer kind to `protos::Ethos` and carries a
caller-issued content hash plus an opaque complete NameTree pin. It does not
derive, inspect, or compose the pin.

## Build and test

```sh
cargo test --all-targets
nix flake check --print-build-logs
```
