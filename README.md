# core-ethos

`core-ethos` owns the strict bootstrap Ethos reader and its purpose-built
Interface, Nexus, and Sema semantic forms.

## Canonical bootstrap surface

The `bootstrap` module is the canonical authoring boundary. One shared
`BootstrapReader` handles every file kind in two phases:

1. `plan` uses `raw-discovery` and the existing `structural-codec` evaluator to
   select one complete structural tree and report every declaration identity
   request with its exact source bound. It allocates nothing.
2. `seal` requires exactly one caller-issued `NamingAssignment` per planned
   occurrence, rejects missing and extra assignments, loads typed priors and
   imported metadata, establishes scopes, resolves strict leaf classes, and
   constructs the kind-selected semantic model.

The reader accepts the ruled bootstrap projections:

```ethos
Interface.{1 0 0}
[component:protocol:interface.[Entry Referent]]
{
  [Submit.Request]
  [Accepted.Response]
  [Rejected.{Reason Explanation}]
  [Request.String Response.String Reason.String Explanation.String]
}
```

- Headers contain exactly three canonical nonnegative decimal components.
- Imports use a colon module path and a nonempty square selector vector,
  including singleton imports.
- Interface is `{Inputs Outputs Refusals Types}` and role relations belong to
  the Interface, not to the referenced types.
- Nexus is `{Traits Types}`; a Trait always has an explicit method product and
  each method's final position is its mandatory return.
- Sema is `{RecordTypes Tables}` and both table leaves must resolve to persistent
  nominal declarations.
- Type expressions are exactly nominal references, nonempty Shape applications,
  or guillemet Trait requirements. Inferred Trait vectors normalize and co-refer
  only within their containing declaration; named binders are scope-local.
- `Name.Stream.(Query Event)` is the sole audited Nomos arm. It atomically
  consumes three externally assigned identities and produces initiation Input,
  direct `Stream<Event>` Output, and termination Input declarations. There is no
  generic transformer carrier or authored termination arm.

`BootstrapReader::write` emits a canonical equivalent projection from encoded
meaning plus source-only import and local-binder metadata. Global visible names
come only from the caller's `EncodedNameResolver`.

The current `VocabularyEncodedId` representation is treated as an injected
opaque naming-authority value. Allocation, collision handling, module storage,
Capsule composition, and identity persistence are outside this crate.

## Transitional carriers

The historical `whole` module and older flat `EncodedEthos` algebra remain for
downstream migration. They are not extended by the bootstrap reader. The flat
algebra is sealed execution data for the current Nomos engine, not an authoring
model.

## Capsule carrier

`capsule_from_issued_hash` fixes the outer kind to `protos::Ethos` and carries a
caller-issued content hash plus an opaque complete NameTree pin. It does not
derive, inspect, or compose the pin.

## Build and test

```sh
cargo test --all-targets
nix flake check --print-build-logs
```
