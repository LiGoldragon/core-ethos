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
complete post-operation name snapshot ─────┘
                                           │
                                           v
                              PreparedBootstrapTransaction
                              (not committed storage)
```

### Textual authority

`TextualMetadataRecord` contains exactly module path, visible name, and encoded
identity. `TextualMetadataSnapshot` indexes the same immutable records in both
directions and refuses more than one record for an identity. Several objects may
share a visible projection; resolving a path/name or visible body reference must
first produce exactly one identity or refuse ambiguity.

Semantic class is never used to pick among ambiguous textual candidates. Only
after one identity is known does the reader query `IdentitySchemaCatalog`.

### Schema authority

`IdentitySchemaCatalog` is identity-keyed. `IdentitySchema` carries a set of
compatible typed `SchemaRole`s. Role families with data—file kind, Interface role,
nominality/persistence, Shape arity, and concrete Nomos schema—admit exactly one
value per family. Distinct families may coexist, so the same Stream identity can
be both an exact one-argument Shape and the exact two-argument audited Nomos head.

`BootstrapPriorVocabulary` validates every prior identity against both the schema
catalog and textual snapshot. The prior type seats:

- three file kinds and three Interface roles;
- persistent primitive nominals;
- exact Vector/Option/Map/Result arities;
- exact Stream Nomos and Stream/StreamIdentity Shape contracts.

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

Planning, sealing, and writing iterate those section descriptors. Limited final
assembly/projection into purpose-built Rust body types is intentionally isolated.

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
assignment set. Every identity must be Universal, fresh against every existing
metadata/schema identity, and unique across authored plus generated identities.
The supplied complete name snapshot must preserve every old record and add exactly
one current-module record for each new object.

All top-level declarations are installed in a transient schema overlay before
body resolution, making source order irrelevant. Visible resolution collects
local, import, and typed-prior candidates, deduplicates by identity, refuses zero
or several candidates, then checks the selected identity's schema role.

Trait requirements sort resolved identities using an explicit canonical byte
projection—root tag followed by big-endian table-local components—not the carrier's
incidental `Ord`. Equal inferred vectors co-refer within one owner. Named binders
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
- no unrelated metadata or schema addition.

The writer uses the prepared transaction's own snapshot. A write, new plan, and
reseal with the same assignments/snapshot reproduces equal semantic meaning and
prepared generation.

## Archive and transitional boundary

`BootstrapArchiveStatus::NotYetArchived` is explicit. A durable archive today
would freeze the chain-shaped identity carrier the bootstrap otherwise treats as
opaque. Once random EncodedName is stable, the semantic document and prepared
schema additions can gain a validated archive/content-identity boundary while
continuing to exclude imports and textual metadata.

`whole` and flat `EncodedEthos` remain untouched transitional consumers. They are
not alternate schemas for new bootstrap work.
