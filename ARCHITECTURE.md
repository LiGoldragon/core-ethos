# Architecture — core-ethos

This document distinguishes the crate's conforming first-slice carrier from the
older flat-identifier graph that remains for regression evidence. Only the
full-chain first-slice path is a production precedent.

## Position in the language family

The next-generation NOTA family is four foundation crates with strictly downward
dependencies, so stringless Encoded never depends on text:

```
content-identity  <-  name-table  <-  raw-discovery  <-  structural-codec
```

Slice one delivered those four with a **synthetic** fixture universe: type ids that
keyed no real Encoded layout, so the structural table's positional signatures were
hand-authored with nothing to check them against (structural-codec named this its
one deferred deviation: "signature-vs-Encoded validation deferred — no Encoded layout in
the PoC").

The original `EncodedEthos`/`TextualEthos` work connected those foundations to a
real encoded layout and exercised signature validation. That graph predates the
approved nested-table identity model and is not the production carrier. The
separate `slice_one` module is the conforming path described below.

## Capsule carrier boundary

`capsule_from_issued_hash` fixes only the outer kind to `protos::Ethos`. It
passes a caller-issued `ContentAddressedHash` and caller-supplied opaque complete
NameTree pin into `protos::Capsule`; it does not wrap `EncodedEthos`, derive a
whole-content hash, inspect the pin, compose module tables, or verify any
correspondence between those values. Complete-pin verification and the
module-table-to-Capsule relationship remain unwired.

The existing `EncodedEthos::content_identity` API and its archive layout are
unchanged. Its flat `Identifier` fields and legacy `NameTable` dependency remain
implementation facts awaiting the encodedID-chain migration, not evidence that
the final nested-table model has landed here. The new identity producer is
dependency-renamed `capsule-content-identity`; the original dependency remains
the established per-value/archive type in the legacy structural graph, preventing
the two revisions from crossing one typed error boundary.

## Typed six-slot production-chain path

The `slice_one` module is intentionally independent of the legacy flat-ID
universe bridge. It defines the authored document as one heterogeneous
`OrderedProduct` with six distinct typed roles: imports, input, output, types,
generics, and implementations. Each slot delegates to its own expected shape,
and the complete source is decoded in one call to the shared structural
evaluator. There is no positional root indexing and no literal
empty-document check. The bounded result exposes the exact absolute
`SourceBound` for every slot.

The supported fixture item set is reified into positional `WholeEthos` data:

- an attribute-free newtype carrying its translator-issued declaration
  `VocabularyEncodedId`, item visibility, typed empty attributes, and a private
  wrapped field;
- an attribute-free, non-generic brace- or square-delimited enumeration
  carrying its declaration chain and ordered variants, where each variant has
  its own full declaration chain and either no payload or one or more
  positional tuple fields; the compact `Variant.Type` source form reifies to
  the same single positional payload as `Variant.{Type}`;
- recursively typed unary applications such as `Vector.Integer`, represented
  as an application-head chain plus one typed payload reference.

The six-slot structural table distinguishes these forms by typed expected
positions and evaluates them through the shared evaluator. `Integer` and
`Vector` seed caller-supplied Universal prior sets; callers explicitly add
every other directly referenced identity and unary application head admitted
by their fixture. Fields carry no names.

Naming stays outside this component. The codec receives its structural type
identities, declaration assignments, and exact Universal reference priors from
callers using the naming-authority contract. Item and variant declaration
positions consume assignments; every type-reference position performs lookup
only. A missing, non-Universal, or unregistered fixture identity is a typed
failure. No
`NameInterner`, local ID mint, or flat-identifier adapter exists on this path.

`WholeEthos` is an archive-validated content carrier, not a Capsule: it carries
neither a composed NameTree pin nor a content-addressed Capsule identity. The
module-to-Capsule relation and pin composition remain separate design work.

## Legacy flat-ID encoded evidence

This section and the universe bridge below describe the older `EncodedEthos`
graph. It is preserved implementation and test evidence, not the current
production contract. In particular, it uses flat `name_table::Identifier`
values, owns a `NameTable`, and models named fields. The approved production
model instead carries complete root-fronted encodedID chains and permits no
field names.

`EncodedType { Newtype | Struct | Enumeration }` is modelled one-for-one on
`schema-language`'s proven `EncodedType` (`schema-language/src/core.rs`). The
legacy shapes carried over:

- Every name is an `Identifier` into a `NameTable`; the declarations carry no
  strings directly. Content identity (`EncodedEthosDomain`, blake3 over archived
  bytes via `content-identity`'s `ContentHash::of_core`) excludes the NameTable,
  so changing a legacy spelling leaves the identifier-bearing value unchanged.
  This does not prove operational rename or nested-table identity continuity.
- `EncodedReference` dispatches **by kind and projection, never a head string**: the
  scalar leaves, `Plain(Identifier)`, and the `SingleTypeReferenceProjection {
  Vector | Optional | ScopeOf }` / `MultiTypeReferenceProjection { Map }` /
  `ValueReferenceProjection { Bytes }` applications lifted verbatim from the ground
  truth. "Generics lower by kind" is thus real in the type, not a convention.

## Legacy universe-bridge evidence

`EncodedUniverse` turns a set of `EncodedEthos` declarations into a structural-codec Encoded
universe. This proves relationships between an encoded layout and an authored
structural table inside the legacy graph; it does not allocate or resolve
production encodedID chains:

- **Fixture type allocation.** One `ScopedEncodedTypeId` per Encoded type — the scalar-leaf
  primitives, the `Field` meta-type, and each user declaration — in an explicit
  fixture universe. One `EncodedConstructorId` per constructor: a product
  (newtype, struct) has one; a sum (enumeration) one per variant. These fixture
  IDs are structural-codec addresses, not translator-issued declaration
  identities.
- **Signature derivation.** `EncodedUniverse::encoded_signature` derives, from the Encoded
  layout alone, each constructor's `PositionalSignature`: the ordered universe-type
  ids of its fields' **referenced** types. A newtype yields `[inner]`; the
  `DatabaseMarker` struct yields `[CommitSequence, StateDigest, StateDigest]`; a
  variant with a payload yields `[payload]`, without yields `[]`.
- **Validation — the deferred deviation, closed.** `EncodedUniverse::validate_table`
  walks an authored `AddressedStructuralTable` and proves every `ConstructorCodec`
  signature equals the Encoded field signature (and that constructor counts match). A
  mismatch is the loud, typed `UniverseError::SignatureMismatch`. The authored table
  and the Encoded-layout derivation are **independent**: the table's signatures are
  hand-authored (as a table author writes them) and checked against the Encoded truth,
  so the agreement test is real, not a tautology — `tests/universe_bridge.rs` proves
  both the agreement and the loud rejection of a corrupted table.

The table's `core_layout_identity` is the Ethos value's own `EncodedEthos` content hash,
tying each structural table to the exact stringless Encoded it targets while the table
identity itself stays **excluded** from Encoded value identity (law 4).

### Two legacy construction modes

`EncodedUniverse` is built two ways. Neither is the production nested-table
authority:

- **Local / offline mode** — `EncodedUniverseBuilder` interns names in call order and
  the caller assigns type ids (the `fixture` family's hardcoded fixture ids). This is
  the self-contained path the legacy tests use. Because interning is parse-order,
  two traversals can allocate different locals. Production allocation is external
  to this crate and canonically ordered by the naming authority.
- **Legacy authority-assignment mode** — `EncodedUniverse::from_assignment(universe, members,
  names)` takes a central-authority-minted universe id, a set of `AssignedMember`s
  (each a declared name, its authority-assigned local, and its kind), and its complete
  composed Schema `NameTable`. It registers members in ascending assigned-local order
  while transferring every supplied identifier and the whole table unchanged. Neither
  `from_assignment` nor `build` resolves and re-interns, re-stamps, or converts an
  identifier: the seal instead validates Schema ownership, table resolution, assigned
  declaration identity, and registered reference targets (`tests/authority_assignment.rs`).
  This is preserved behavior over the old flat identifiers. It is not the approved
  nested module-owned encodedID-chain authority model; migrating the builder and parser
  assignment surfaces belongs to the coordinated encodedID-chain work.

### Legacy encoded/text granularity evidence

A struct's Encoded `signature` records its fields' **referenced types**
(`[CommitSequence, StateDigest, StateDigest]`) — the Encoded truth. Its structural
**form** is a product of `Delegate(Field)` slots — the text surface, where each
field is decoded through the `Field` meta-type's two disjoint constructors. Signature
(Encoded) and form (text) are decoupled at different granularities, which is why
the evaluator walks forms while `validate_table` checks signatures against
Encoded. The `Field` meta-type also carries legacy field-name identifiers; that
part is nonconforming and is not a precedent for the positional production
carrier.

## Legacy TextualEthos evidence

`TextualEthos` is one bidirectional codec over the universe. Decode: raw-discovery
recognizes text into a `Block`; structural-codec's trusted evaluator decodes it
(under the expected Encoded type) to a generic `StructuralValue`; `core-ethos`
**reifies** that mirror into a real `EncodedType` with a real `NameTable`. Encode
**reflects** an `EncodedType` back into a `StructuralValue`, the evaluator renders
it to a `Block`, and it is written as canonical text. Its `Field`
elided-vs-explicit alternatives and early `snake_case` field-name interning are
legacy behavior. They conflict with positional field storage and with evaluating
typed name projections only at the textual-form boundary.

The reify/reflect pair remains useful evaluator evidence. This document makes no
claim about a future generated replacement.

## Repository boundary and migration status

`core-ethos` does **not** edit `schema-language`, `schema`, `schema-rust`, `nota`,
`sema-engine`, or the foundation crates. The repository currently contains two
grades of implementation: the conforming, deliberately narrow `WholeEthos`
first-slice path and the broader legacy `EncodedEthos` graph. The latter's
coverage does not widen production support.

Cross-repository consumption is by pinned git revisions. `Cargo.toml` and
`Cargo.lock` are the authority for the exact dependency revisions.

## Historical choices in the legacy graph

These choices describe existing legacy code. They are not approved extensions
of the production carrier:

1. **Struct field slots delegate to the legacy `Field` meta-type** (form) while the struct
   **signature records referenced types** (Encoded). The alternative — inlining
   per-field forms and making the signature `[Field, Field, Field]` — loses the
   concrete referenced types from the signature. The chosen reading keeps the
   signature the most informative "Encoded field types, in order" and matches slice
   one's historical `Field` disjointness exercise.
2. **The legacy `Field` constructor signatures are empty.** A field's payload is name
   identifiers (a type *name*, an optional field *name*), not typed sub-structures,
   and names are not types — so the positional **type** signature is empty. The
   optional field name is prohibited on the production path.
3. **Legacy `Text` is a string-leaf primitive**, and the `Documentation -> Summary -> Text`
   chain is newtypes delegating to it; the terminal scalar leaf does the dotted-text
   rejoin.
4. **Generic applications (Vector/Optional/Map/ScopeOf/Bytes-value) are modelled in
   `EncodedReference` but have no allocated universe type** in this PoC universe: the
   fixture family uses none, and `resolve_reference` returns a loud
   `UnsupportedApplication` rather than guessing.
