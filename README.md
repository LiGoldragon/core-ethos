# core-ethos

`core-ethos` owns the strict bootstrap Ethos reader and its purpose-built
Interface, Nexus, and Sema semantic forms.

## Canonical bootstrap surface

The `bootstrap` module separates five authorities that must never collapse:

1. `TextualMetadataSnapshot` is a bidirectional, one-record-per-object snapshot.
   Each projection address is module path, optional lexical-owner identity, and
   visible name. Exact addresses are unique; variants or methods under different
   owners may lawfully reuse one spelling. Metadata says nothing about semantic
   class.
2. `IdentitySchemaCatalog` is keyed only by encoded identity. One identity may
   carry several compatible roles, each with exact data such as Shape arity,
   persistence, file kind, Interface role, or audited Nomos schema.
3. `BootstrapReader::plan` uses the shared structural evaluator, validates every
   assignment-independent cardinality and Shape/Nomos arity, and reports only
   authored declaration occurrences with exact source bounds. It allocates no
   identity.
4. `NamingAssignments` supplies exactly one externally authorized identity and
   `Existing`/`New` disposition for every authored occurrence. New identities
   carry authority-supplied canonical ordering bytes. Generated Stream identities
   use the same dispositions in their separate assignment channel.
5. `BootstrapReader::seal` consumes an explicit authority-issued
   `TextualMetadataTransition`. That before→after transition may preserve,
   rename, move, add, or remove projections. The reader validates it against the
   plan and catalog; it never invents or commits a transition.

The resulting `PreparedBootstrapTransaction` has private invariant-bearing fields
and read-only accessors, so stores can accept only a validated value. Untrusted
parts remain a `PreparedBootstrapDraft` until `validate_draft` succeeds. The writer
consumes its exact transition and uses the after snapshot in both directions. No
magic spelling exists for
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

The root orders live once in a typed root-schema registry. Planning, assembly,
semantic projection, and writing all consult that registry. The order is exposed
read-only for tools that need to render the provisional bootstrap surface.

Body lookup is namespace-first. File-kind and Interface-role priors never enter
nominal, Trait, Shape, or Nomos reference candidates. Ambiguity is refused within
the syntactically applicable namespace before exact role data is checked. Shape
and Nomos heads are closed over typed prior-vocabulary identities; an imported
object cannot become language syntax merely by registering a Shape or Nomos role.

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
one-argument Stream and StreamIdentity Shapes. Strict runtime carriers model the
query initiation value, a `RuntimeStream<Event>` containing its typed
`RuntimeStreamIdentity<Event>`, and termination containing the same handle.
Live registries, routing behavior, and storage commitment remain outside Ethos.

## Identity and archive boundary

Grammar document and syntax identities are both injected explicitly. The reader
derives no hierarchical encoded identity and reconstructs no ordering from an
identity carrier. `CanonicalIdentityOrder` is supplied by identity authority for
every catalog object; each New disposition supplies the bytes for its object.
Every unordered named semantic collection is normalized by those bytes while
positional struct fields, method parameters, and Shape arguments retain authored
order.

Authored and generated assignments must be Universal and mutually collision-free.
`New` identities must be absent from metadata, schemas, and canonical authority;
`Existing` identities must already exist with the exact reusable schema role.
This makes unchanged rereads and stable rename/move/delete edits survive a
persisted restart without reminting identities.

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
