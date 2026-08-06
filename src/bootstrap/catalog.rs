//! Independent textual and semantic authority snapshots.

use std::collections::{BTreeMap, BTreeSet};

use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};

use super::error::BootstrapReadError;
use super::model::{EthosKind, EthosVersion, InterfaceRole, RuntimeStreamSchemaContract};

/// One textual projection record. It carries no semantic class information.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextualMetadataRecord {
    pub module_path: Vec<String>,
    pub visible_name: String,
    pub encoded_name: VocabularyEncodedId,
}

/// An immutable bidirectional one-record-per-object textual snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextualMetadataSnapshot {
    records: Vec<TextualMetadataRecord>,
    by_identity: BTreeMap<VocabularyEncodedId, usize>,
    by_projection: BTreeMap<(Vec<String>, String), Vec<VocabularyEncodedId>>,
}

impl TextualMetadataSnapshot {
    pub fn new(records: Vec<TextualMetadataRecord>) -> Result<Self, BootstrapReadError> {
        let mut by_identity = BTreeMap::new();
        let mut by_projection = BTreeMap::<(Vec<String>, String), Vec<VocabularyEncodedId>>::new();
        for (index, record) in records.iter().enumerate() {
            validate_module_path(&record.module_path)?;
            validate_visible_name(&record.visible_name)?;
            if by_identity
                .insert(record.encoded_name.clone(), index)
                .is_some()
            {
                return Err(BootstrapReadError::DuplicateMetadataIdentity(
                    record.encoded_name.clone(),
                ));
            }
            by_projection
                .entry((record.module_path.clone(), record.visible_name.clone()))
                .or_default()
                .push(record.encoded_name.clone());
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

    pub fn record(&self, identity: &VocabularyEncodedId) -> Option<&TextualMetadataRecord> {
        self.by_identity
            .get(identity)
            .map(|index| &self.records[*index])
    }

    pub fn spelling(&self, identity: &VocabularyEncodedId) -> Option<&str> {
        self.record(identity)
            .map(|record| record.visible_name.as_str())
    }

    pub fn identities_at(
        &self,
        module_path: &[String],
        visible_name: &str,
    ) -> &[VocabularyEncodedId] {
        self.by_projection
            .get(&(module_path.to_vec(), visible_name.to_owned()))
            .map_or(&[], Vec::as_slice)
    }

    pub(crate) fn identities(&self) -> impl Iterator<Item = &VocabularyEncodedId> {
        self.by_identity.keys()
    }

    pub(crate) fn extends(&self, earlier: &Self) -> Result<(), BootstrapReadError> {
        for record in &earlier.records {
            if self.record(&record.encoded_name) != Some(record) {
                return Err(BootstrapReadError::MetadataSnapshotDoesNotExtend(
                    record.encoded_name.clone(),
                ));
            }
        }
        Ok(())
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
    identity: VocabularyEncodedId,
    roles: BTreeSet<SchemaRole>,
}

impl IdentitySchema {
    pub fn new(
        identity: VocabularyEncodedId,
        roles: impl IntoIterator<Item = SchemaRole>,
    ) -> Result<Self, BootstrapReadError> {
        if identity.root_variant() != &VocabularyRoot::Universal {
            return Err(BootstrapReadError::NonUniversalSchemaIdentity(identity));
        }
        let roles = roles.into_iter().collect::<BTreeSet<_>>();
        for role in &roles {
            for other in &roles {
                if role < other && conflicting_role_family(*role, *other) {
                    return Err(BootstrapReadError::ConflictingSchemaRoles {
                        identity: identity.clone(),
                        first: *role,
                        second: *other,
                    });
                }
            }
        }
        Ok(Self { identity, roles })
    }

    pub fn identity(&self) -> &VocabularyEncodedId {
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

fn conflicting_role_family(left: SchemaRole, right: SchemaRole) -> bool {
    matches!(
        (left, right),
        (SchemaRole::FileKind(_), SchemaRole::FileKind(_))
            | (SchemaRole::InterfaceRole(_), SchemaRole::InterfaceRole(_))
            | (SchemaRole::Nominal { .. }, SchemaRole::Nominal { .. })
            | (SchemaRole::Shape { .. }, SchemaRole::Shape { .. })
            | (SchemaRole::Nomos(_), SchemaRole::Nomos(_))
    )
}

/// Identity-keyed semantic schema authority.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IdentitySchemaCatalog {
    entries: BTreeMap<VocabularyEncodedId, IdentitySchema>,
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

    pub fn get(&self, identity: &VocabularyEncodedId) -> Option<&IdentitySchema> {
        self.entries.get(identity)
    }

    pub fn entries(&self) -> impl Iterator<Item = &IdentitySchema> {
        self.entries.values()
    }

    pub(crate) fn contains(&self, identity: &VocabularyEncodedId) -> bool {
        self.entries.contains_key(identity)
    }
}

/// Typed identities for the closed bootstrap prior vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapPriorIdentities {
    pub interface_kind: VocabularyEncodedId,
    pub nexus_kind: VocabularyEncodedId,
    pub sema_kind: VocabularyEncodedId,
    pub input_role: VocabularyEncodedId,
    pub output_role: VocabularyEncodedId,
    pub refusal_role: VocabularyEncodedId,
    pub string_type: VocabularyEncodedId,
    pub integer_type: VocabularyEncodedId,
    pub boolean_type: VocabularyEncodedId,
    pub unit_type: VocabularyEncodedId,
    pub vector_shape: VocabularyEncodedId,
    pub option_shape: VocabularyEncodedId,
    pub map_shape: VocabularyEncodedId,
    pub result_shape: VocabularyEncodedId,
    pub stream_nomos: VocabularyEncodedId,
    pub stream_shape: VocabularyEncodedId,
    pub stream_identity_shape: VocabularyEncodedId,
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
            if identity.root_variant() != &VocabularyRoot::Universal {
                return Err(BootstrapReadError::NonUniversalPrior { position });
            }
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

    pub(crate) fn all_identities(&self) -> Vec<VocabularyEncodedId> {
        let ids = &self.identities;
        vec![
            ids.interface_kind.clone(),
            ids.nexus_kind.clone(),
            ids.sema_kind.clone(),
            ids.input_role.clone(),
            ids.output_role.clone(),
            ids.refusal_role.clone(),
            ids.string_type.clone(),
            ids.integer_type.clone(),
            ids.boolean_type.clone(),
            ids.unit_type.clone(),
            ids.vector_shape.clone(),
            ids.option_shape.clone(),
            ids.map_shape.clone(),
            ids.result_shape.clone(),
            ids.stream_nomos.clone(),
            ids.stream_shape.clone(),
            ids.stream_identity_shape.clone(),
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
}

impl BootstrapCatalog {
    pub fn new(
        current_module_path: Vec<String>,
        metadata: TextualMetadataSnapshot,
        schemas: IdentitySchemaCatalog,
        priors: BootstrapPriorVocabulary,
        versions: BootstrapVersionPolicy,
    ) -> Result<Self, BootstrapReadError> {
        validate_module_path(&current_module_path)?;
        for schema in schemas.entries() {
            if metadata.record(schema.identity()).is_none() {
                return Err(BootstrapReadError::MissingMetadataIdentity(
                    schema.identity().clone(),
                ));
            }
        }
        for identity in metadata.identities() {
            if !schemas.contains(identity) {
                return Err(BootstrapReadError::MissingSchema(identity.clone()));
            }
        }
        let priors = BootstrapPriorVocabulary::new(priors.identities.clone(), &schemas, &metadata)?;
        Ok(Self {
            current_module_path,
            metadata,
            schemas,
            priors,
            versions,
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
}

pub(crate) fn validate_module_path(path: &[String]) -> Result<(), BootstrapReadError> {
    if path.is_empty() || path.iter().any(|part| !is_safe_bare_name(part)) {
        return Err(BootstrapReadError::InvalidModulePath(path.to_vec()));
    }
    Ok(())
}

pub(crate) fn validate_visible_name(name: &str) -> Result<(), BootstrapReadError> {
    if is_safe_bare_name(name) {
        Ok(())
    } else {
        Err(BootstrapReadError::InvalidVisibleName(name.to_owned()))
    }
}

pub(crate) fn is_safe_bare_name(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_alphabetic())
        && characters.all(|character| character == '_' || character.is_alphanumeric())
}
