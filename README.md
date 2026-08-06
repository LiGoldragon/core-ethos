# core-ethos

`core-ethos` owns the strict bootstrap Ethos reader and its purpose-built
Interface, Nexus, and Sema semantic forms.

## Canonical bootstrap surface

The `bootstrap` module separates five authorities that must never collapse:

1. `TextualMetadataSnapshot` is a bidirectional, one-record-per-object snapshot.
   Each record is only module path, visible name, and encoded identity. It says
   nothing about semantic class.
2. `IdentitySchemaCatalog` is keyed only by encoded identity. One identity may
   carry several compatible roles, each with exact data such as Shape arity,
   persistence, file kind, Interface role, or audited Nomos schema.
3. `BootstrapReader::plan` uses the shared structural evaluator, validates every
   assignment-independent cardinality and Shape/Nomos arity, and reports only
   authored declaration occurrences with exact source bounds. It allocates no
   identity.
4. `NamingAssignments` supplies exactly one externally allocated identity for
   every authored occurrence. `GeneratedStreamAssignments` separately supplies
   initiation and termination identities keyed to an authored Stream occurrence.
5. `BootstrapReader::seal` resolves textual ambiguity to one identity before
   consulting its schema roles, validates the complete post-operation name
   snapshot, and returns a `PreparedBootstrapTransaction`. It commits no naming,
   schema, or component storage.

The writer consumes that same prepared transaction and therefore the same
bidirectional textual snapshot used by sealing. No magic spelling exists for
file kinds, primitives, Shapes, roles, Stream, or StreamIdentity: every visible
projection comes from metadata attached to a typed prior identity.

## Strict bootstrap projections

The active file laws remain:

- a metadata-projected kind followed by exactly `{Major Minor Patch}`;
- an explicit square import vector with colon module paths and nonempty square
  selector vectors, including singleton imports;
- Interface `{Inputs Outputs Refusals Types}`;
- Nexus `{Traits Types}` with explicit marker Trait products and one mandatory
  final method return;
- Sema `{RecordTypes Tables}` with persistent nominal record/key leaves;
- recursive type expressions consisting only of a nominal reference, an
  exact-arity Shape application, or a nonempty guillemet Trait requirement.

The root orders live once in a typed root-schema registry. Planning, sealing,
and writing iterate that registry; their only kind-specific operation is the
final construction/projection of the purpose-built body carrier.

## Stream transaction

`Name.Stream.(Query Event)` decodes as the authored algebra
`Declaration::Nomos(NomosDeclaration::StreamInitiation(...))`. The authored name
is assigned like every other source occurrence and designates the direct Stream
Output identity.

The separate generated assignment supplies exactly two more identities. Sealing
prepares, without storing:

- initiation Input containing the Query type;
- direct `Stream<Event>` Output;
- termination Input referencing that direct Output;
- exactly Input, Output, Input role relations in that order.

The `BootstrapPriorVocabulary::runtime_stream_contract` explicitly seats the
one-argument Stream and StreamIdentity runtime Shape contracts. Live registries,
routing, termination behavior, and storage commitment remain outside Ethos.

## Identity and archive boundary

Grammar document and syntax identities are both injected explicitly. The reader
derives no hierarchical encoded identity. Authored and generated assignments must
be Universal, fresh against all existing metadata/schema identities, and mutually
collision-free.

Semantic archiving intentionally reports `BootstrapArchiveStatus::NotYetArchived`.
Archiving now would freeze the current chain-shaped `VocabularyEncodedId` carrier
before the random EncodedName substrate is settled. Source-only imports and
textual projections are already excluded from semantic meaning; a validated
archive/content-identity boundary belongs immediately after the identity substrate
is stable.

## Transitional carriers

The historical `whole` module and flat `EncodedEthos` execution algebra remain
unchanged for downstream migration. The bootstrap reader does not extend them.

## Build and test

```sh
cargo test --all-targets
nix flake check --print-build-logs --no-update-lock-file --option substitute false
```
