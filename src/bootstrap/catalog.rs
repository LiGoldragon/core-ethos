//! Independent textual and semantic authority snapshots.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use name_table::{EncodedName, TextualName};

use super::error::BootstrapReadError;
use super::model::{EthosKind, EthosVersion, InterfaceRole, RuntimeStreamSchemaContract};

/// Bootstrap-local address of one closed prior definition.
///
/// This is construction metadata, not an identity. The Sema authority mints
/// opaque [`EncodedName`] values for these positions before it builds a
/// [`BootstrapCatalog`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BootstrapPriorSlot {
    InterfaceKind,
    NexusKind,
    SemaKind,
    InputRole,
    OutputRole,
    RefusalRole,
    StringType,
    IntegerType,
    BooleanType,
    UnitType,
    VectorShape,
    OptionShape,
    MapShape,
    ResultShape,
    Stream,
    StreamIdentityShape,
}

/// One identity-free role in the closed bootstrap seed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapPriorRole {
    FileKind(EthosKind),
    InterfaceRole(InterfaceRole),
    Nominal { persistent: bool },
    Shape { arity: u16 },
    Nomos(NomosSchema),
}

impl BootstrapPriorRole {
    /// Materialize this identity-free seed role for the catalog it builds.
    pub const fn schema_role(self) -> SchemaRole {
        match self {
            Self::FileKind(kind) => SchemaRole::FileKind(kind),
            Self::InterfaceRole(role) => SchemaRole::InterfaceRole(role),
            Self::Nominal { persistent } => SchemaRole::Nominal { persistent },
            Self::Shape { arity } => SchemaRole::Shape { arity },
            Self::Nomos(nomos) => SchemaRole::Nomos(nomos),
        }
    }
}

/// One closed bootstrap prior before the authority assigns its opaque name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapPriorDefinition {
    pub slot: BootstrapPriorSlot,
    pub textual_name: &'static str,
    pub roles: &'static [BootstrapPriorRole],
}

const INTERFACE_KIND_ROLES: &[BootstrapPriorRole] =
    &[BootstrapPriorRole::FileKind(EthosKind::Interface)];
const NEXUS_KIND_ROLES: &[BootstrapPriorRole] = &[BootstrapPriorRole::FileKind(EthosKind::Nexus)];
const SEMA_KIND_ROLES: &[BootstrapPriorRole] = &[BootstrapPriorRole::FileKind(EthosKind::Sema)];
const INPUT_ROLE_ROLES: &[BootstrapPriorRole] =
    &[BootstrapPriorRole::InterfaceRole(InterfaceRole::Input)];
const OUTPUT_ROLE_ROLES: &[BootstrapPriorRole] =
    &[BootstrapPriorRole::InterfaceRole(InterfaceRole::Output)];
const REFUSAL_ROLE_ROLES: &[BootstrapPriorRole] =
    &[BootstrapPriorRole::InterfaceRole(InterfaceRole::Refusal)];
const PERSISTENT_NOMINAL_ROLES: &[BootstrapPriorRole] =
    &[BootstrapPriorRole::Nominal { persistent: true }];
const UNARY_SHAPE_ROLES: &[BootstrapPriorRole] = &[BootstrapPriorRole::Shape { arity: 1 }];
const BINARY_SHAPE_ROLES: &[BootstrapPriorRole] = &[BootstrapPriorRole::Shape { arity: 2 }];
const STREAM_ROLES: &[BootstrapPriorRole] = &[
    BootstrapPriorRole::Shape { arity: 1 },
    BootstrapPriorRole::Nomos(NomosSchema::StreamInitiation { arity: 2 }),
];

const BOOTSTRAP_PRIOR_DEFINITIONS: &[BootstrapPriorDefinition] = &[
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::InterfaceKind,
        textual_name: "Interface",
        roles: INTERFACE_KIND_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::NexusKind,
        textual_name: "Nexus",
        roles: NEXUS_KIND_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::SemaKind,
        textual_name: "Sema",
        roles: SEMA_KIND_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::InputRole,
        textual_name: "Input",
        roles: INPUT_ROLE_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::OutputRole,
        textual_name: "Output",
        roles: OUTPUT_ROLE_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::RefusalRole,
        textual_name: "Refusal",
        roles: REFUSAL_ROLE_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::StringType,
        textual_name: "String",
        roles: PERSISTENT_NOMINAL_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::IntegerType,
        textual_name: "Integer",
        roles: PERSISTENT_NOMINAL_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::BooleanType,
        textual_name: "Boolean",
        roles: PERSISTENT_NOMINAL_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::UnitType,
        textual_name: "Unit",
        roles: PERSISTENT_NOMINAL_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::VectorShape,
        textual_name: "Vector",
        roles: UNARY_SHAPE_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::OptionShape,
        textual_name: "Option",
        roles: UNARY_SHAPE_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::MapShape,
        textual_name: "Map",
        roles: BINARY_SHAPE_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::ResultShape,
        textual_name: "Result",
        roles: BINARY_SHAPE_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::Stream,
        textual_name: "Stream",
        roles: STREAM_ROLES,
    },
    BootstrapPriorDefinition {
        slot: BootstrapPriorSlot::StreamIdentityShape,
        textual_name: "StreamIdentity",
        roles: UNARY_SHAPE_ROLES,
    },
];

/// The exact existing closed bootstrap prior definitions, without identities.
pub fn bootstrap_prior_definitions() -> &'static [BootstrapPriorDefinition] {
    BOOTSTRAP_PRIOR_DEFINITIONS
}

/// The exact lexical address of one textual projection. Nested declarations
/// use their owning encoded identity, so equal sibling spellings never occupy
/// the same address.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TextualProjectionAddress {
    pub module_path: Vec<String>,
    pub lexical_owner: Option<EncodedName>,
    pub textual_name: TextualName,
}

/// One textual projection record. It carries no semantic class information.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextualMetadataRecord {
    pub address: TextualProjectionAddress,
    pub encoded_name: EncodedName,
}

/// An immutable bidirectional one-record-per-object textual snapshot.
#[derive(Clone, Debug)]
pub struct TextualMetadataSnapshot {
    records: Vec<TextualMetadataRecord>,
    by_identity: BTreeMap<EncodedName, usize>,
    by_projection: BTreeMap<TextualProjectionAddress, EncodedName>,
}

impl PartialEq for TextualMetadataSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.by_identity.len() == other.by_identity.len()
            && self.records.iter().all(|record| {
                other
                    .record(&record.encoded_name)
                    .is_some_and(|other| other == record)
            })
    }
}

impl Eq for TextualMetadataSnapshot {}

impl TextualMetadataSnapshot {
    pub fn new(records: Vec<TextualMetadataRecord>) -> Result<Self, BootstrapReadError> {
        let mut by_identity = BTreeMap::new();
        let mut by_projection = BTreeMap::new();
        for (index, record) in records.iter().enumerate() {
            validate_module_path(&record.address.module_path)?;
            validate_textual_name(record.address.textual_name.as_str())?;
            if by_identity
                .insert(record.encoded_name.clone(), index)
                .is_some()
            {
                return Err(BootstrapReadError::DuplicateMetadataIdentity(
                    record.encoded_name.clone(),
                ));
            }
            if by_projection
                .insert(record.address.clone(), record.encoded_name.clone())
                .is_some()
            {
                return Err(BootstrapReadError::DuplicateMetadataProjectionAddress {
                    module_path: record.address.module_path.clone(),
                    lexical_owner: record.address.lexical_owner.clone(),
                    name: record.address.textual_name.as_str().to_owned(),
                });
            }
        }
        for record in &records {
            if let Some(owner) = &record.address.lexical_owner {
                if owner == &record.encoded_name || !by_identity.contains_key(owner) {
                    return Err(BootstrapReadError::InvalidMetadataLexicalOwner {
                        identity: record.encoded_name.clone(),
                        owner: owner.clone(),
                    });
                }
            }
        }
        Ok(Self {
            records,
            by_identity,
            by_projection,
        })
    }

    pub fn records(&self) -> &[TextualMetadataRecord] {
        &self.records
    }

    pub fn record(&self, identity: &EncodedName) -> Option<&TextualMetadataRecord> {
        self.by_identity
            .get(identity)
            .map(|index| &self.records[*index])
    }

    pub fn spelling(&self, identity: &EncodedName) -> Option<&str> {
        self.record(identity)
            .map(|record| record.address.textual_name.as_str())
    }

    pub fn identity_at(
        &self,
        module_path: &[String],
        lexical_owner: Option<&EncodedName>,
        textual_name: &str,
    ) -> Option<&EncodedName> {
        self.by_projection.get(&TextualProjectionAddress {
            module_path: module_path.to_vec(),
            lexical_owner: lexical_owner.cloned(),
            textual_name: TextualName::new(textual_name),
        })
    }

    pub(crate) fn identities(&self) -> impl Iterator<Item = &EncodedName> {
        self.by_identity.keys()
    }
}

/// Public proposal for one explicit before-to-after projection change. Only an
/// injected [`crate::bootstrap::BootstrapNamingAuthority`] can authenticate it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextualMetadataTransition {
    before: TextualMetadataSnapshot,
    after: TextualMetadataSnapshot,
}

impl TextualMetadataTransition {
    pub const fn new(before: TextualMetadataSnapshot, after: TextualMetadataSnapshot) -> Self {
        Self { before, after }
    }

    pub const fn before(&self) -> &TextualMetadataSnapshot {
        &self.before
    }

    pub const fn after(&self) -> &TextualMetadataSnapshot {
        &self.after
    }
}

/// Canonical bytes supplied by identity authority. No EncodedName carrier
/// anatomy participates in semantic ordering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalIdentityOrder {
    by_identity: BTreeMap<EncodedName, Vec<u8>>,
}

impl CanonicalIdentityOrder {
    pub fn new(
        entries: impl IntoIterator<Item = (EncodedName, Vec<u8>)>,
    ) -> Result<Self, BootstrapReadError> {
        let mut by_identity = BTreeMap::new();
        let mut identities_by_bytes = BTreeMap::<Vec<u8>, EncodedName>::new();
        for (identity, bytes) in entries {
            if bytes.is_empty() {
                return Err(BootstrapReadError::EmptyCanonicalIdentityBytes(identity));
            }
            if by_identity
                .insert(identity.clone(), bytes.clone())
                .is_some()
            {
                return Err(BootstrapReadError::DuplicateCanonicalIdentity(identity));
            }
            if let Some(other) = identities_by_bytes.insert(bytes, identity.clone()) {
                return Err(BootstrapReadError::DuplicateCanonicalIdentityBytes {
                    first: other,
                    second: identity,
                });
            }
        }
        Ok(Self { by_identity })
    }

    pub fn bytes(&self, identity: &EncodedName) -> Option<&[u8]> {
        self.by_identity.get(identity).map(Vec::as_slice)
    }

    pub fn entries(&self) -> impl Iterator<Item = (&EncodedName, &[u8])> {
        self.by_identity
            .iter()
            .map(|(identity, bytes)| (identity, bytes.as_slice()))
    }

    pub fn compare(
        &self,
        left: &EncodedName,
        right: &EncodedName,
    ) -> Result<Ordering, BootstrapReadError> {
        let left = self
            .bytes(left)
            .ok_or_else(|| BootstrapReadError::MissingCanonicalIdentity(left.clone()))?;
        let right = self
            .bytes(right)
            .ok_or_else(|| BootstrapReadError::MissingCanonicalIdentity(right.clone()))?;
        Ok(left.cmp(right))
    }

    pub(crate) fn contains(&self, identity: &EncodedName) -> bool {
        self.by_identity.contains_key(identity)
    }

    pub(crate) fn extended(
        &self,
        additions: impl IntoIterator<Item = (EncodedName, Vec<u8>)>,
    ) -> Result<Self, BootstrapReadError> {
        let mut entries = self
            .by_identity
            .iter()
            .map(|(identity, bytes)| (identity.clone(), bytes.clone()))
            .collect::<Vec<_>>();
        entries.extend(additions);
        Self::new(entries)
    }
}

/// The concrete audited bootstrap Nomos schema selected by an identity role.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NomosSchema {
    StreamInitiation { arity: u16 },
}

/// One semantic role registered independently of textual projection.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SchemaRole {
    FileKind(EthosKind),
    InterfaceRole(InterfaceRole),
    Nominal { persistent: bool },
    Shape { arity: u16 },
    Trait,
    Nomos(NomosSchema),
    Variant,
    Method,
    Table,
}

/// All admitted roles for one encoded identity. Multiple distinct roles are
/// intentional; a Stream identity can be both Shape and audited Nomos head.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentitySchema {
    identity: EncodedName,
    roles: BTreeSet<SchemaRole>,
}

impl IdentitySchema {
    pub fn new(
        identity: EncodedName,
        roles: impl IntoIterator<Item = SchemaRole>,
    ) -> Result<Self, BootstrapReadError> {
        let roles = roles.into_iter().collect::<BTreeSet<_>>();
        if roles.is_empty() || !admitted_role_set(&roles) {
            return Err(BootstrapReadError::IncompatibleSchemaRoles {
                identity,
                roles: roles.iter().copied().collect(),
            });
        }
        Ok(Self { identity, roles })
    }

    pub fn identity(&self) -> &EncodedName {
        &self.identity
    }

    pub fn roles(&self) -> &BTreeSet<SchemaRole> {
        &self.roles
    }

    pub fn admits(&self, role: SchemaRole) -> bool {
        self.roles.contains(&role)
    }

    pub fn shape_arity(&self) -> Option<u16> {
        self.roles.iter().find_map(|role| match role {
            SchemaRole::Shape { arity } => Some(*arity),
            _ => None,
        })
    }

    pub fn nomos(&self) -> Option<NomosSchema> {
        self.roles.iter().find_map(|role| match role {
            SchemaRole::Nomos(schema) => Some(*schema),
            _ => None,
        })
    }
}

fn admitted_role_set(roles: &BTreeSet<SchemaRole>) -> bool {
    if roles.len() == 1 {
        return true;
    }
    roles
        == &BTreeSet::from([
            SchemaRole::Shape { arity: 1 },
            SchemaRole::Nomos(NomosSchema::StreamInitiation { arity: 2 }),
        ])
}

/// Identity-keyed semantic schema authority.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IdentitySchemaCatalog {
    entries: BTreeMap<EncodedName, IdentitySchema>,
}

impl IdentitySchemaCatalog {
    pub fn new(entries: Vec<IdentitySchema>) -> Result<Self, BootstrapReadError> {
        let mut by_identity = BTreeMap::new();
        for entry in entries {
            let identity = entry.identity.clone();
            if by_identity.insert(identity.clone(), entry).is_some() {
                return Err(BootstrapReadError::DuplicateSchemaIdentity(identity));
            }
        }
        Ok(Self {
            entries: by_identity,
        })
    }

    pub fn get(&self, identity: &EncodedName) -> Option<&IdentitySchema> {
        self.entries.get(identity)
    }

    pub fn entries(&self) -> impl Iterator<Item = &IdentitySchema> {
        self.entries.values()
    }

    pub(crate) fn contains(&self, identity: &EncodedName) -> bool {
        self.entries.contains_key(identity)
    }
}

/// Typed identities for the closed bootstrap prior vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapPriorIdentities {
    pub interface_kind: EncodedName,
    pub nexus_kind: EncodedName,
    pub sema_kind: EncodedName,
    pub input_role: EncodedName,
    pub output_role: EncodedName,
    pub refusal_role: EncodedName,
    pub string_type: EncodedName,
    pub integer_type: EncodedName,
    pub boolean_type: EncodedName,
    pub unit_type: EncodedName,
    pub vector_shape: EncodedName,
    pub option_shape: EncodedName,
    pub map_shape: EncodedName,
    pub result_shape: EncodedName,
    pub stream_nomos: EncodedName,
    pub stream_shape: EncodedName,
    pub stream_identity_shape: EncodedName,
}

/// Validated typed seating of all bootstrap priors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapPriorVocabulary {
    identities: BootstrapPriorIdentities,
}

impl BootstrapPriorVocabulary {
    pub fn new(
        identities: BootstrapPriorIdentities,
        schemas: &IdentitySchemaCatalog,
        metadata: &TextualMetadataSnapshot,
    ) -> Result<Self, BootstrapReadError> {
        if identities.stream_nomos != identities.stream_shape {
            return Err(BootstrapReadError::InvalidPriorIdentityRelationship(
                "stream_nomos and stream_shape must be the same identity",
            ));
        }
        let requirements = [
            (
                "interface_kind",
                &identities.interface_kind,
                SchemaRole::FileKind(EthosKind::Interface),
            ),
            (
                "nexus_kind",
                &identities.nexus_kind,
                SchemaRole::FileKind(EthosKind::Nexus),
            ),
            (
                "sema_kind",
                &identities.sema_kind,
                SchemaRole::FileKind(EthosKind::Sema),
            ),
            (
                "input_role",
                &identities.input_role,
                SchemaRole::InterfaceRole(InterfaceRole::Input),
            ),
            (
                "output_role",
                &identities.output_role,
                SchemaRole::InterfaceRole(InterfaceRole::Output),
            ),
            (
                "refusal_role",
                &identities.refusal_role,
                SchemaRole::InterfaceRole(InterfaceRole::Refusal),
            ),
            (
                "string_type",
                &identities.string_type,
                SchemaRole::Nominal { persistent: true },
            ),
            (
                "integer_type",
                &identities.integer_type,
                SchemaRole::Nominal { persistent: true },
            ),
            (
                "boolean_type",
                &identities.boolean_type,
                SchemaRole::Nominal { persistent: true },
            ),
            (
                "unit_type",
                &identities.unit_type,
                SchemaRole::Nominal { persistent: true },
            ),
            (
                "vector_shape",
                &identities.vector_shape,
                SchemaRole::Shape { arity: 1 },
            ),
            (
                "option_shape",
                &identities.option_shape,
                SchemaRole::Shape { arity: 1 },
            ),
            (
                "map_shape",
                &identities.map_shape,
                SchemaRole::Shape { arity: 2 },
            ),
            (
                "result_shape",
                &identities.result_shape,
                SchemaRole::Shape { arity: 2 },
            ),
            (
                "stream_nomos",
                &identities.stream_nomos,
                SchemaRole::Nomos(NomosSchema::StreamInitiation { arity: 2 }),
            ),
            (
                "stream_shape",
                &identities.stream_shape,
                SchemaRole::Shape { arity: 1 },
            ),
            (
                "stream_identity_shape",
                &identities.stream_identity_shape,
                SchemaRole::Shape { arity: 1 },
            ),
        ];
        for (position, identity, required) in requirements {
            let schema =
                schemas
                    .get(identity)
                    .ok_or_else(|| BootstrapReadError::InvalidPriorRole {
                        position,
                        identity: identity.clone(),
                        required,
                    })?;
            if !schema.admits(required) {
                return Err(BootstrapReadError::InvalidPriorRole {
                    position,
                    identity: identity.clone(),
                    required,
                });
            }
            if metadata.record(identity).is_none() {
                return Err(BootstrapReadError::MissingMetadataIdentity(
                    identity.clone(),
                ));
            }
        }
        let positions = [
            ("interface_kind", &identities.interface_kind),
            ("nexus_kind", &identities.nexus_kind),
            ("sema_kind", &identities.sema_kind),
            ("input_role", &identities.input_role),
            ("output_role", &identities.output_role),
            ("refusal_role", &identities.refusal_role),
            ("string_type", &identities.string_type),
            ("integer_type", &identities.integer_type),
            ("boolean_type", &identities.boolean_type),
            ("unit_type", &identities.unit_type),
            ("vector_shape", &identities.vector_shape),
            ("option_shape", &identities.option_shape),
            ("map_shape", &identities.map_shape),
            ("result_shape", &identities.result_shape),
            ("stream_shape", &identities.stream_shape),
            ("stream_identity_shape", &identities.stream_identity_shape),
        ];
        let mut seated = BTreeMap::new();
        for (position, identity) in positions {
            if let Some(previous) = seated.insert(identity.clone(), position) {
                return Err(BootstrapReadError::DuplicatePriorIdentity {
                    first: previous,
                    second: position,
                    identity: identity.clone(),
                });
            }
        }
        let kind_names = [
            metadata.spelling(&identities.interface_kind),
            metadata.spelling(&identities.nexus_kind),
            metadata.spelling(&identities.sema_kind),
        ];
        if kind_names[0] == kind_names[1]
            || kind_names[0] == kind_names[2]
            || kind_names[1] == kind_names[2]
        {
            return Err(BootstrapReadError::DuplicateFileKindProjection);
        }
        Ok(Self { identities })
    }

    pub fn identities(&self) -> &BootstrapPriorIdentities {
        &self.identities
    }

    pub fn runtime_stream_contract(&self) -> RuntimeStreamSchemaContract {
        RuntimeStreamSchemaContract {
            stream_shape: self.identities.stream_shape.clone(),
            stream_identity_shape: self.identities.stream_identity_shape.clone(),
            stream_shape_arity: 1,
            stream_identity_shape_arity: 1,
        }
    }

    pub(crate) fn body_nominal_identities(&self) -> [&EncodedName; 4] {
        let ids = &self.identities;
        [
            &ids.string_type,
            &ids.integer_type,
            &ids.boolean_type,
            &ids.unit_type,
        ]
    }

    pub(crate) fn shape_identities(&self) -> [&EncodedName; 6] {
        let ids = &self.identities;
        [
            &ids.vector_shape,
            &ids.option_shape,
            &ids.map_shape,
            &ids.result_shape,
            &ids.stream_shape,
            &ids.stream_identity_shape,
        ]
    }

    pub(crate) fn role_identity(&self, role: InterfaceRole) -> &EncodedName {
        match role {
            InterfaceRole::Input => &self.identities.input_role,
            InterfaceRole::Output => &self.identities.output_role,
            InterfaceRole::Refusal => &self.identities.refusal_role,
        }
    }

    pub(crate) fn is_fixed_identity(&self, identity: &EncodedName) -> bool {
        self.fixed_identities().contains(&identity)
    }

    pub(crate) fn fixed_identities(&self) -> [&EncodedName; 16] {
        let ids = &self.identities;
        [
            &ids.interface_kind,
            &ids.nexus_kind,
            &ids.sema_kind,
            &ids.input_role,
            &ids.output_role,
            &ids.refusal_role,
            &ids.string_type,
            &ids.integer_type,
            &ids.boolean_type,
            &ids.unit_type,
            &ids.vector_shape,
            &ids.option_shape,
            &ids.map_shape,
            &ids.result_shape,
            &ids.stream_shape,
            &ids.stream_identity_shape,
        ]
    }
}

/// Explicit supported-version policy applied during planning and writing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapVersionPolicy {
    supported: BTreeSet<EthosVersion>,
}

impl BootstrapVersionPolicy {
    pub fn exact(version: EthosVersion) -> Self {
        Self {
            supported: BTreeSet::from([version]),
        }
    }

    pub fn new(
        versions: impl IntoIterator<Item = EthosVersion>,
    ) -> Result<Self, BootstrapReadError> {
        let supported = versions.into_iter().collect::<BTreeSet<_>>();
        if supported.is_empty() {
            return Err(BootstrapReadError::InvalidPreparedModel(
                "supported-version policy is empty",
            ));
        }
        Ok(Self { supported })
    }

    pub fn supports(&self, version: EthosVersion) -> bool {
        self.supported.contains(&version)
    }

    pub fn supported(&self) -> Vec<EthosVersion> {
        self.supported.iter().copied().collect()
    }
}

/// Existing authority state injected into one reader.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapCatalog {
    current_module_path: Vec<String>,
    metadata: TextualMetadataSnapshot,
    schemas: IdentitySchemaCatalog,
    priors: BootstrapPriorVocabulary,
    versions: BootstrapVersionPolicy,
    canonical_order: CanonicalIdentityOrder,
}

impl BootstrapCatalog {
    pub fn new(
        current_module_path: Vec<String>,
        metadata: TextualMetadataSnapshot,
        schemas: IdentitySchemaCatalog,
        priors: BootstrapPriorVocabulary,
        versions: BootstrapVersionPolicy,
        canonical_order: CanonicalIdentityOrder,
    ) -> Result<Self, BootstrapReadError> {
        validate_module_path(&current_module_path)?;
        for identity in metadata.identities() {
            if !schemas.contains(identity) {
                return Err(BootstrapReadError::MissingSchema(identity.clone()));
            }
        }
        for schema in schemas.entries() {
            if !canonical_order.contains(schema.identity()) {
                return Err(BootstrapReadError::MissingCanonicalIdentity(
                    schema.identity().clone(),
                ));
            }
        }
        let priors = BootstrapPriorVocabulary::new(priors.identities.clone(), &schemas, &metadata)?;
        Ok(Self {
            current_module_path,
            metadata,
            schemas,
            priors,
            versions,
            canonical_order,
        })
    }

    pub fn current_module_path(&self) -> &[String] {
        &self.current_module_path
    }

    pub fn metadata(&self) -> &TextualMetadataSnapshot {
        &self.metadata
    }

    pub fn schemas(&self) -> &IdentitySchemaCatalog {
        &self.schemas
    }

    pub fn priors(&self) -> &BootstrapPriorVocabulary {
        &self.priors
    }

    pub fn versions(&self) -> &BootstrapVersionPolicy {
        &self.versions
    }

    pub fn canonical_order(&self) -> &CanonicalIdentityOrder {
        &self.canonical_order
    }
}

pub(crate) fn validate_module_path(path: &[String]) -> Result<(), BootstrapReadError> {
    if path.is_empty() || path.iter().any(|part| !is_safe_bare_name(part)) {
        return Err(BootstrapReadError::InvalidModulePath(path.to_vec()));
    }
    Ok(())
}

pub(crate) fn validate_textual_name(name: &str) -> Result<(), BootstrapReadError> {
    if is_safe_bare_name(name) {
        Ok(())
    } else {
        Err(BootstrapReadError::InvalidTextualName(name.to_owned()))
    }
}

pub(crate) fn is_safe_bare_name(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_alphabetic())
        && characters.all(|character| character == '_' || character.is_alphanumeric())
}
