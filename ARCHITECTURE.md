# Architecture — core-ethos

## Authority-separated bootstrap pipeline

The bootstrap implementation is a staged transaction boundary:

```text
existing TextualMetadataSnapshot ─┐
existing IdentitySchemaCatalog ───┼─> structural plan
typed priors + version policy ─────┘       │
                                           ├─ authored occurrences
external NamingAssignments ────────────────┤
external Stream generated assignments ─────┤
metadata before→after proposal ────────────┤
naming-authority proof ────────────────────┤
authority canonical identity bytes ────────┘
                                           │
                                           v
                         authority-issued exact-draft receipt
                                           │
                                           v
                         PreparedBootstrapTransaction<Authority>
                              (not committed storage)
```

### Textual authority

`TextualMetadataRecord` contains exactly a projection address and encoded identity.
The address is module path, optional lexical-owner identity, and visible name.
`TextualMetadataSnapshot` indexes the same immutable records in both directions,
refuses multiple records for an identity, and refuses multiple identities at an
exact address. Lexical owners let sibling enum variants and Trait methods reuse a
spelling without colliding.

`TextualMetadataTransition` is a caller-constructible proposal containing exact
before and after snapshots. It can preserve, add, rename, move, or remove
identities, but cannot authenticate itself. Seal first assembles the complete
`PreparedBootstrapDraft`, then presents that exact draft and an authority-specific
proof to the injected `BootstrapNamingAuthority`. Successful authorization
returns a configuration-specific receipt. Independently, seal requires the
before snapshot to equal the reader catalog and validates the after snapshot
against assignments, schemas, canonical identity bytes, and semantic visibility.
The reader never authors or commits the transition.

Candidate collection is first restricted to the syntactically applicable
namespace. Fixed file-kind/role vocabulary never enters body-reference lookup.
Within the applicable nominal, Trait, Shape, or Nomos namespace, ambiguity is
refused before exact schema data is validated. Two imports can therefore be
ambiguous only through distinct valid projection addresses, such as two module
paths selecting the same local spelling.

### Schema authority

`IdentitySchemaCatalog` is identity-keyed. `IdentitySchema` accepts only an
explicit role-set allowlist: a single exact role, or the designed Stream pairing
of one-argument Shape with two-argument Stream-initiation Nomos. Negative
same-family inference cannot accidentally admit a new cross-role combination.

`BootstrapPriorVocabulary` validates every prior identity against both the schema
catalog and textual snapshot. The prior type seats:

- three file kinds and three Interface roles;
- persistent primitive nominals;
- exact Vector/Option/Map/Result arities;
- exact Stream Nomos and Stream/StreamIdentity Shape contracts.

The Stream Nomos and Stream Shape positions must be the same identity.
StreamIdentity is distinct, and every other typed prior position is pairwise
distinct. Shape and Nomos syntax resolves only to these prior seats; imported
schema roles cannot extend bootstrap syntax.

Visible names are never embedded in the prior type or parser.

## One structural evaluator and one root registry

Both grammar type identities—document and recursive syntax—are caller supplied.
Only constructor-local structural addresses are derived. The custom sealed token
profile adds guillemets while retaining the established discovery machinery.

One `StructuralEvaluator::plan_text` call selects a recursive tree of atom, dot
application, bare-angle application, braces, squares, parentheses, and
guillemets. The semantic planner then walks the expected schema; this private tree
never appears in encoded meaning.

`RootSchemaRegistry` is the sole seating of provisional root choices:

- Interface: Role(Input), Role(Output), Role(Refusal), Declarations(Nomos admitted)
- Nexus: Traits, Declarations(Nomos refused)
- Sema: PersistentDeclarations, Tables

Planning, assembly, semantic projection, and writing iterate those descriptors.
There is no second per-kind section-order table.

## Planning and scopes

Planning validates before requesting identities:

- exact envelope/header/import/root arities and explicit empty vectors;
- enum nonemptiness and product-variant nonemptiness;
- method mandatory return and table two-leaf anatomy;
- exact catalog-defined Shape arity at every recursive occurrence;
- exact audited Nomos schema and arity;
- Interface/Nexus/Sema admission boundaries;
- module, enum-variant, and Trait-method duplicate scopes;
- safe declaration, import, local-binder, and reference projections;
- nonempty Trait requirements and duplicate visible Trait occurrences.

The result contains only authored `DeclarationOccurrence`s. Handles include a
process-local plan generation and ordinal, so assignments from another plan are
extras even if their ordinals coincide.

## Sealing and generated work

Sealing requires an exact authored assignment set and an exact generated Stream
assignment set. Every assignment has an authority disposition. `New` means the
authority-admitted identity is absent from metadata, schema, and canonical-order
authority; `Existing` means the object already exists, is not fixed prior
vocabulary, and admits exactly the declaration role being reused. All authored
and generated identities remain mutually unique. Grammar, catalog, reader, and
runtime admission never inspect an EncodedName root, tag, or chain; grammar
constructor identifiers are only local structural addresses.

The prepared transaction records the private receipt with the dispositions and
exact metadata transition. `PreparedBootstrapTransaction<Authority>` is branded
by the authority type, so even two authority implementations sharing one receipt
carrier cannot consume each other's transactions. Public untrusted parts remain
a `PreparedBootstrapDraft` until `validate_draft` obtains a receipt and constructs
the wrapper. Validation also proves that every after-snapshot identity has both a
schema and canonical bytes, so a next catalog built from the after snapshot,
merged schemas, and canonical order succeeds by construction. A reread then uses
`Existing` dispositions and produces no schema additions. Rename, move, and
deletion similarly reuse identities instead of reminting them.

All New declarations are installed in a transient schema overlay before
body resolution, making source order irrelevant. Visible resolution collects
local, import, and namespace-applicable typed-prior candidates, refuses zero or
several candidates, then checks the selected identity's exact schema data.

`CanonicalIdentityOrder` supplies opaque authoritative bytes for every existing
identity, and every New disposition supplies its bytes. No chain/tag anatomy is
reconstructed. Traits, top-level declarations, Interface role entries and
memberships, methods, enum variants, tables, generated Stream sets, and import
selectors are normalized by these bytes. Positional struct fields, method
parameters, Shape arguments, and the fixed Stream relation anatomy preserve their
meaningful order. Equal inferred Trait vectors co-refer within one owner. Named binders
carry their validated local projection inside the semantic binder form; the field
is not publicly mutable, and exhaustive transaction validation refuses owner
escape, incompatible reuse, named/inferred collapse, or noncanonical vectors.

The authored Stream remains
`Declaration::Nomos(StreamInitiationDeclaration)`. Its prepared generation is a
separate transaction result with three purpose-built declarations and exactly
three role relations. Nothing says those additions were stored.

## Invariant and writer boundary

Every seal and write performs exhaustive validation:

- supported header version and header/body agreement;
- root-section meaning and exact Interface memberships;
- declaration/schema-addition equality;
- reference class, persistence, Shape arity, and canonical Trait vectors;
- local binder ownership/co-reference laws;
- Stream identity distinctness, output Shape/arity/event equality, termination
  reference, and exact role relations;
- safe and complete textual projection records;
- exact authorized metadata transition and schema additions;
- visibility and same-identity textual round-trip for every nominal, Trait, Shape,
  and relation reference through the local/import/prior environment;
- canonical ordering of every unordered named collection.

The receipt has a public read-only accessor and a transaction verification method
for external stores. Verification reconstructs the exact complete draft and asks
the configured authority to verify its receipt; a different configuration of the
same authority type is therefore refused. Reader transaction validation and the
writer perform the same receipt check before exhaustive model validation. The
writer then uses the prepared transition's after snapshot for every emitted
spelling and reverse visibility check. Authored references see authored
declarations, imports, and priors; generated initiation and termination
declarations are seated only in the separate internal environment used to
validate generated Stream anatomy. A write, persisted restart, new plan, and
reseal with Existing assignments reproduces equal semantic meaning without new
schema objects.

Runtime values have strict typed carriers: initiation contains only Query, the
`RuntimeStream<Event>` contains only a registry-admitted
`RuntimeStreamIdentity<Event>`, and termination contains the exact same
`RuntimeStream<Event>` handle value rather than a bare identity. Unregistered and
mismatched handles are typed refusals.

## Archive and transitional boundary

`BootstrapArchiveStatus::NotYetArchived` is explicit. A durable archive today
would freeze the chain-shaped identity carrier the bootstrap otherwise treats as
opaque. Once random EncodedName is stable, the semantic document and prepared
schema additions can gain a validated archive/content-identity boundary while
continuing to exclude imports and textual metadata.

`whole` and flat `EncodedEthos` remain untouched transitional consumers. They are
not alternate schemas for new bootstrap work.
