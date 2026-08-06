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
5. `TextualMetadataTransition` is a public before→after proposal, never proof of
   its own authority. `BootstrapReader::seal` presents the complete
   `PreparedBootstrapDraft` and an authority-specific proof to the injected
   `BootstrapNamingAuthority`. Only successful authorization issues the private,
   configuration-specific receipt retained by the prepared transaction. A
   proposal may preserve, rename, move, add, or remove projections; the reader
   still validates its structural consistency and never commits it.

The resulting `PreparedBootstrapTransaction<Authority>` is branded by its
authority type and has private invariant-bearing fields. Its receipt is exposed
read-only, with a public verification method for stores configured with that
authority. Untrusted parts remain a `PreparedBootstrapDraft` until
`validate_draft` obtains a receipt and succeeds. Reader validation and every
write re-verify that receipt against the exact reconstructed draft before using
the transition. No magic spelling exists for
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
query initiation value, a `RuntimeStream<Event>` containing only its registered
typed `RuntimeStreamIdentity<Event>`, and termination containing the exact same
`RuntimeStream<Event>` value.
Live registries, routing behavior, and storage commitment remain outside Ethos.

## Identity and archive boundary

Grammar document and syntax identities are both injected explicitly. The reader
derives no hierarchical encoded identity and reconstructs no ordering from an
identity carrier. `CanonicalIdentityOrder` is supplied by identity authority for
every catalog object; each New disposition supplies the bytes for its object.
Every unordered named semantic collection is normalized by those bytes while
positional struct fields, method parameters, and Shape arguments retain authored
order.

Authored and generated assignments must be admitted by the injected naming
authority and remain mutually collision-free. Bootstrap code never inspects an
encoded identity's internal anatomy. `New` identities must be absent from
metadata, schemas, and canonical authority; `Existing` identities must already
exist with the exact reusable schema role. This makes unchanged rereads and
stable rename/move/delete edits survive a persisted restart without reminting
identities.

Each validated Interface, Nexus, or Sema body is its own portable rkyv value.
`PreparedBootstrapTransaction::body_true_name` derives its `TrueName` directly
from that strict body; source imports, textual projections, receipts, and
authority state remain outside the content identity.

## Build and test

```sh
cargo test --all-targets
nix flake check --print-build-logs --no-update-lock-file --option substitute false
```
