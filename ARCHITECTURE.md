# Architecture — core-ethos

## Shared two-phase bootstrap reader

`bootstrap` separates structural discovery, naming authority, semantic sealing,
and textual projection.

The structural layer owns one Ethos-specific sealed token profile and one
addressed table. Its recursive syntax type distinguishes atom, dot application,
bare-angle application, braces, squares, parentheses, and guillemets. A single
ordered document record requires Header, Imports, and Body. Every file kind goes
through the same `StructuralEvaluator::plan_text` call; the selected header only
chooses a typed body-root definition. There is no Interface, Nexus, or Sema
parser.

This structural tree is intentionally pre-semantic. The expected typed position
then chooses `Declaration`, `TypeExpression`, `RoleEntry`, Trait method, or table
meaning. Consequently an atom is never classified by spelling alone, and the
semantic model contains no generic syntax tree or transformer application.

Planning walks the selected schema before any body resolution. It produces
ephemeral `DeclarationOccurrence` handles with exact full-source bounds and
enforces module, enum-variant, and Trait-method uniqueness. A Stream occurrence
requests its output, initiation, and termination identities in one plan. These
handles are decode-local coordination values; they are not archived or hashed.

Sealing accepts a `NamingAssignments` set and proves it equals the planned set.
The reader has no allocation capability. It constructs a complete resolution
environment from:

- all top-level assignments, so source order is irrelevant;
- the source-only import selectors resolved through `BootstrapCatalog`;
- the closed, role-typed `BootstrapPriorVocabulary`.

References are checked against their position's admitted class: nominal,
persistent nominal, Shape, Trait, or the audited Stream Nomos head. Local
parameter binders never receive a global encoded identity. Inferred Trait
vectors are sorted by encoded identity and used as declaration-local
co-reference keys; named binders additionally reject incompatible reuse.

The semantic output consists only of purpose-built carriers:

- Interface roles contain declarations or nominal references, while all
  `InterfaceRoleMembership` relations are owned by the Interface root;
- plain types contain newtype, struct, or enum bodies and strict recursive type
  expressions;
- Stream contains exactly its three generated nominal declarations, while the
  Interface holds their Input/Output/Input relations;
- Nexus holds Traits first, then supporting declarations, with marker Traits
  represented by an explicit empty method product;
- Sema holds persistent nominal declarations and tables whose record and key
  leaves are persistent nominal identities.

Review-sensitive root orders and the complete prior catalog are isolated in
typed definitions. Nothing in these semantic forms names Rust, LLVM, an ABI, a
storage engine, or the current operating system.

## Canonical writer

The writer traverses the strict model rather than source bytes. It retrieves
global spellings from an injected `EncodedNameResolver`, uses retained
source-only imports and local-binder projections, restores every explicit empty
vector/product, and emits the ruled delimiters and ordering. Stream writes back
only `OutputName.Stream.(Query Event)`; initiation and termination remain
generated meaning rather than additional authored declarations.

## Transitional boundary

`whole` is the previous composite authoring carrier and remains temporarily for
downstream compatibility. `EncodedEthos` and its declaration/reference algebra
predate the full-chain model and remain only as sealed execution data. The new
reader does not extend either representation. Removing them belongs to consumer
migration, not to widening the bootstrap schema.

## Identity and Capsule boundary

Grammar, prior, declaration, and lookup identities are caller-supplied. The
crate treats the pinned `VocabularyEncodedId` carrier as opaque and does not
allocate names, handle collisions, store naming tables, or derive a durable
identity scheme from its present chain anatomy.

`capsule_from_issued_hash` similarly passes through a caller-issued content hash
and opaque complete NameTree pin. It does not derive their relationship.
