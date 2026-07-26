//! The schema universe and its validation against archived typed table records.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use name_table::{Identifier, IdentifierNamespace, Name, NameResolver, NameTable};
use structural_codec::{
    AddressedStructuralTable, BorrowedFieldView, EncodedLanguage, FieldRole, FieldVisitor,
    Position, SharedDescriptor, StructureRecord,
};

use crate::declaration::{EncodedDeclaration, EncodedSchema, EncodedType};
use crate::error::UniverseError;
use crate::reference::{BuiltinReference, EncodedReference};

#[derive(Clone, Debug)]
pub enum MemberKind {
    Primitive,
    FieldMeta,
    Declaration(EncodedDeclaration),
}

impl MemberKind {
    fn constructor_count(&self) -> usize {
        match self {
            Self::Primitive | Self::FieldMeta => 1,
            Self::Declaration(declaration) => declaration.value().constructor_count(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UniverseType {
    id: structural_codec::ScopedEncodedTypeId,
    name: Identifier,
    kind: MemberKind,
}

impl UniverseType {
    pub fn id(&self) -> structural_codec::ScopedEncodedTypeId {
        self.id
    }

    pub fn name(&self) -> Identifier {
        self.name
    }

    pub fn kind(&self) -> &MemberKind {
        &self.kind
    }
}

/// The layout-derived constructor field signature.  Structural-codec no longer
/// stores a positional signature vector in each codec, so this remains a
/// core-schema value used to compare the Encoded layout to typed record metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedFieldSignature(Vec<structural_codec::ScopedEncodedTypeId>);

impl EncodedFieldSignature {
    pub fn fields(&self) -> &[structural_codec::ScopedEncodedTypeId] {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct EncodedUniverse {
    language: EncodedLanguage,
    names: NameTable,
    members: Vec<UniverseType>,
    by_id: BTreeMap<structural_codec::ScopedEncodedTypeId, usize>,
    by_name: HashMap<Identifier, structural_codec::ScopedEncodedTypeId>,
    /// Slice 0 retains this existing prior-definition index.  Slice 1 owns its
    /// replacement; no new builtin resolution is added here.
    builtins: HashMap<String, BuiltinReference>,
    integer: Option<structural_codec::ScopedEncodedTypeId>,
    text: Option<structural_codec::ScopedEncodedTypeId>,
    boolean: Option<structural_codec::ScopedEncodedTypeId>,
    bytes: Option<structural_codec::ScopedEncodedTypeId>,
}

impl EncodedUniverse {
    pub fn language(&self) -> EncodedLanguage {
        self.language
    }

    pub fn names(&self) -> &NameTable {
        &self.names
    }

    pub fn names_mut(&mut self) -> &mut NameTable {
        &mut self.names
    }

    pub fn members(&self) -> &[UniverseType] {
        &self.members
    }

    pub fn declared_schema(&self) -> EncodedSchema {
        let mut ordered: Vec<&UniverseType> = self.members.iter().collect();
        ordered.sort_by_key(|member| member.id);
        EncodedSchema::new(
            ordered
                .into_iter()
                .filter_map(|member| match member.kind() {
                    MemberKind::Declaration(declaration) => Some(declaration.clone()),
                    MemberKind::Primitive | MemberKind::FieldMeta => None,
                })
                .collect(),
        )
    }

    pub fn from_assignment(
        language: EncodedLanguage,
        mut members: Vec<AssignedMember>,
        names: NameTable,
    ) -> Result<Self, UniverseError> {
        members.sort_by_key(AssignedMember::local);
        let mut builder = EncodedUniverseBuilder::from_name_table(names);
        for member in members {
            let id = scoped(language, member.local);
            match member.kind {
                AssignedKind::ScalarPrimitive(slot) => {
                    builder.primitive_at(id, member.identifier, slot)
                }
                AssignedKind::LeafPrimitive => builder.leaf_at(id, member.identifier),
                AssignedKind::FieldMeta => builder.field_meta_at(id, member.identifier),
                AssignedKind::Declaration(declaration) => {
                    builder.assigned_declaration(id, member.identifier, declaration)
                }
            }
        }
        builder.build(language)
    }

    fn validate_schema_identifier(identifier: Identifier) -> Result<(), UniverseError> {
        if identifier.namespace() == IdentifierNamespace::Schema {
            Ok(())
        } else {
            Err(UniverseError::WrongSchemaIdentifier(identifier))
        }
    }

    fn validate_scoped_type_id(
        expected: EncodedLanguage,
        member: structural_codec::ScopedEncodedTypeId,
    ) -> Result<(), UniverseError> {
        if member.language() == expected {
            Ok(())
        } else {
            Err(UniverseError::UniverseScopeMismatch {
                expected,
                actual: member.language(),
                member,
            })
        }
    }

    fn validate_reference_identifiers(
        reference: &EncodedReference,
        names: &NameTable,
        members: &[UniverseType],
        scalar_registrations: &[(ScalarSlot, structural_codec::ScopedEncodedTypeId)],
        expected_language: EncodedLanguage,
    ) -> Result<(), UniverseError> {
        let validate_scalar = |slot| {
            let id = scalar_registrations
                .iter()
                .find_map(|(registered, id)| (*registered == slot).then_some(*id))
                .ok_or_else(|| UniverseError::MissingScalarSlot {
                    slot,
                    reference: reference.clone(),
                })?;
            Self::validate_scoped_type_id(expected_language, id)?;
            if members.iter().any(|member| member.id == id) {
                Ok(())
            } else {
                Err(UniverseError::MissingScalarSlot {
                    slot,
                    reference: reference.clone(),
                })
            }
        };
        match reference {
            EncodedReference::String => validate_scalar(ScalarSlot::Text),
            EncodedReference::Integer => validate_scalar(ScalarSlot::Integer),
            EncodedReference::Boolean => validate_scalar(ScalarSlot::Boolean),
            EncodedReference::Bytes => validate_scalar(ScalarSlot::Bytes),
            EncodedReference::Plain(identifier) => {
                Self::validate_schema_identifier(*identifier)?;
                names
                    .resolve(*identifier)
                    .map_err(|_| UniverseError::ReferenceNameAbsent {
                        identifier: *identifier,
                        reference: reference.clone(),
                    })?;
                let member = members
                    .iter()
                    .find(|member| member.name == *identifier)
                    .ok_or_else(|| UniverseError::ReferenceTargetUnregistered {
                        identifier: *identifier,
                        reference: reference.clone(),
                    })?;
                Self::validate_scoped_type_id(expected_language, member.id)
            }
            EncodedReference::SingleTypeApplication { argument, .. } => {
                Self::validate_reference_identifiers(
                    argument,
                    names,
                    members,
                    scalar_registrations,
                    expected_language,
                )
            }
            EncodedReference::MultiTypeApplication { arguments, .. } => {
                arguments.iter().try_for_each(|argument| {
                    Self::validate_reference_identifiers(
                        argument,
                        names,
                        members,
                        scalar_registrations,
                        expected_language,
                    )
                })
            }
            EncodedReference::ValueApplication { .. } => Ok(()),
        }
    }

    pub fn reference_from_name<Resolver: NameResolver + ?Sized>(
        &self,
        identifier: Identifier,
        names: &Resolver,
    ) -> Result<EncodedReference, UniverseError> {
        Self::validate_schema_identifier(identifier)?;
        let name = names.resolve(identifier)?;
        Ok(self
            .builtins
            .get(name.as_str())
            .and_then(|builtin| builtin.scalar_reference())
            .unwrap_or(EncodedReference::Plain(identifier)))
    }

    pub fn builtin_from_name<Resolver: NameResolver + ?Sized>(
        &self,
        identifier: Identifier,
        names: &Resolver,
    ) -> Result<Option<BuiltinReference>, UniverseError> {
        Self::validate_schema_identifier(identifier)?;
        Ok(self
            .builtins
            .get(names.resolve(identifier)?.as_str())
            .copied())
    }

    pub fn validate_declaration_name<Resolver: NameResolver + ?Sized>(
        &self,
        identifier: Identifier,
        names: &Resolver,
    ) -> Result<(), UniverseError> {
        if let Some(builtin) = self.builtin_from_name(identifier, names)? {
            return Err(crate::error::StructuralRedefinition::new(identifier, builtin).into());
        }
        Ok(())
    }

    fn validate_declaration_identifiers(
        declaration: &EncodedDeclaration,
        names: &NameTable,
        members: &[UniverseType],
        scalar_registrations: &[(ScalarSlot, structural_codec::ScopedEncodedTypeId)],
        expected_language: EncodedLanguage,
    ) -> Result<(), UniverseError> {
        let validate_identifier = |identifier| {
            Self::validate_schema_identifier(identifier)?;
            names.resolve(identifier)?;
            Ok::<_, UniverseError>(())
        };
        match declaration.value() {
            EncodedType::Newtype(newtype) => Self::validate_reference_identifiers(
                newtype.reference(),
                names,
                members,
                scalar_registrations,
                expected_language,
            ),
            EncodedType::Struct(structure) => {
                for field in structure.fields() {
                    validate_identifier(field.identifier())?;
                    Self::validate_reference_identifiers(
                        field.reference(),
                        names,
                        members,
                        scalar_registrations,
                        expected_language,
                    )?;
                }
                Ok(())
            }
            EncodedType::Enumeration(enumeration) => {
                for variant in enumeration.variants() {
                    validate_identifier(variant.identifier())?;
                    if let Some(payload) = variant.payload() {
                        Self::validate_reference_identifiers(
                            payload,
                            names,
                            members,
                            scalar_registrations,
                            expected_language,
                        )?;
                    }
                }
                Ok(())
            }
        }
    }

    fn member(
        &self,
        id: structural_codec::ScopedEncodedTypeId,
    ) -> Result<&UniverseType, UniverseError> {
        self.by_id
            .get(&id)
            .and_then(|index| self.members.get(*index))
            .ok_or(UniverseError::UnknownType(id))
    }

    pub fn encoded_type(&self, id: structural_codec::ScopedEncodedTypeId) -> Option<&EncodedType> {
        match self.member(id).ok()?.kind() {
            MemberKind::Declaration(declaration) => Some(declaration.value()),
            MemberKind::Primitive | MemberKind::FieldMeta => None,
        }
    }

    pub fn type_of_name(&self, name: Identifier) -> Option<structural_codec::ScopedEncodedTypeId> {
        self.by_name.get(&name).copied()
    }

    pub fn resolve_reference(
        &self,
        reference: &EncodedReference,
    ) -> Result<structural_codec::ScopedEncodedTypeId, UniverseError> {
        let scalar = |slot, id: Option<structural_codec::ScopedEncodedTypeId>| {
            id.ok_or_else(|| UniverseError::MissingScalarSlot {
                slot,
                reference: reference.clone(),
            })
        };
        match reference {
            EncodedReference::Integer => scalar(ScalarSlot::Integer, self.integer),
            EncodedReference::String => scalar(ScalarSlot::Text, self.text),
            EncodedReference::Boolean => scalar(ScalarSlot::Boolean, self.boolean),
            EncodedReference::Bytes => scalar(ScalarSlot::Bytes, self.bytes),
            EncodedReference::Plain(identifier) => {
                self.names.resolve(*identifier).map_err(|_| {
                    UniverseError::ReferenceNameAbsent {
                        identifier: *identifier,
                        reference: reference.clone(),
                    }
                })?;
                self.by_name.get(identifier).copied().ok_or_else(|| {
                    UniverseError::ReferenceTargetUnregistered {
                        identifier: *identifier,
                        reference: reference.clone(),
                    }
                })
            }
            EncodedReference::SingleTypeApplication { .. } => Err(
                UniverseError::UnsupportedApplication("single-type generic application"),
            ),
            EncodedReference::MultiTypeApplication { .. } => Err(
                UniverseError::UnsupportedApplication("multi-type generic application"),
            ),
            EncodedReference::ValueApplication { .. } => {
                Err(UniverseError::UnsupportedApplication("value application"))
            }
        }
    }

    pub fn constructor_count(
        &self,
        id: structural_codec::ScopedEncodedTypeId,
    ) -> Result<usize, UniverseError> {
        Ok(self.member(id)?.kind.constructor_count())
    }

    pub fn encoded_signature(
        &self,
        id: structural_codec::ScopedEncodedTypeId,
        constructor: u16,
    ) -> Result<EncodedFieldSignature, UniverseError> {
        let member = self.member(id)?;
        let fields = match &member.kind {
            MemberKind::Primitive | MemberKind::FieldMeta => Vec::new(),
            MemberKind::Declaration(declaration) => match declaration.value() {
                EncodedType::Newtype(newtype) => vec![self.resolve_reference(newtype.reference())?],
                EncodedType::Struct(structure) => structure
                    .fields()
                    .iter()
                    .map(|field| self.resolve_reference(field.reference()))
                    .collect::<Result<_, _>>()?,
                EncodedType::Enumeration(enumeration) => {
                    let variant = enumeration.variants().get(constructor as usize).ok_or(
                        UniverseError::ConstructorCountMismatch {
                            encoded_type: id,
                            members: enumeration.variants().len(),
                            codecs: constructor as usize + 1,
                        },
                    )?;
                    variant
                        .payload()
                        .map(|payload| self.resolve_reference(payload).map(|id| vec![id]))
                        .transpose()?
                        .unwrap_or_default()
                }
            },
        };
        Ok(EncodedFieldSignature(fields))
    }

    /// Validate the current table representation without restoring the removed
    /// positional-signature vector.  A constructor's unreachable typed metadata
    /// positions (stable roles 1003–1005) carry the exact authored layout when a
    /// repeated executable form cannot itself carry a fixed product.  Other
    /// constructors use their actual typed `Delegate` descriptor metadata.
    pub fn validate_table<Record: StructureRecord>(
        &self,
        table: &AddressedStructuralTable<Record>,
    ) -> Result<(), UniverseError> {
        struct DelegateMetadata {
            all: Vec<structural_codec::ScopedEncodedTypeId>,
            signature: Vec<structural_codec::ScopedEncodedTypeId>,
        }
        impl FieldVisitor for DelegateMetadata {
            fn field<Role: FieldRole>(&mut self, position: &Position<Role>) {
                if let SharedDescriptor::Delegate { target, .. } = position.descriptor() {
                    self.all.push(*target);
                    if (1003..=1005).contains(&position.role().value()) {
                        self.signature.push(*target);
                    }
                }
            }
        }

        for member in &self.members {
            let entry = table
                .entry(member.id)
                .ok_or(UniverseError::TableEntryAbsent(member.id))?;
            let expected_count = member.kind.constructor_count();
            if entry.constructors().len() != expected_count {
                return Err(UniverseError::ConstructorCountMismatch {
                    encoded_type: member.id,
                    members: expected_count,
                    codecs: entry.constructors().len(),
                });
            }
            for codec in entry.constructors() {
                let expected = self.encoded_signature(member.id, codec.constructor().local())?;
                let mut metadata = DelegateMetadata {
                    all: Vec::new(),
                    signature: Vec::new(),
                };
                codec.encode_form().fields().expose(&mut metadata);
                let authored = if metadata.signature.is_empty() {
                    metadata.all
                } else {
                    metadata.signature
                };
                if authored != expected.fields() {
                    return Err(UniverseError::SignatureMismatch {
                        encoded_type: member.id,
                        constructor: codec.constructor().local(),
                        authored,
                        encoded: expected.fields().to_vec(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum AssignedKind {
    ScalarPrimitive(ScalarSlot),
    LeafPrimitive,
    FieldMeta,
    Declaration(EncodedDeclaration),
}

#[derive(Clone, Debug)]
pub struct AssignedMember {
    local: u16,
    identifier: Identifier,
    kind: AssignedKind,
}

impl AssignedMember {
    pub fn new(local: u16, identifier: Identifier, kind: AssignedKind) -> Self {
        Self {
            local,
            identifier,
            kind,
        }
    }

    pub fn local(&self) -> u16 {
        self.local
    }

    pub fn identifier(&self) -> Identifier {
        self.identifier
    }

    pub fn kind(&self) -> &AssignedKind {
        &self.kind
    }
}

#[derive(Debug)]
pub struct EncodedUniverseBuilder {
    names: NameTable,
    members: Vec<UniverseType>,
    scalar_registrations: Vec<(ScalarSlot, structural_codec::ScopedEncodedTypeId)>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ScalarSlot {
    Integer,
    Text,
    Boolean,
    Bytes,
}

impl Default for EncodedUniverseBuilder {
    fn default() -> Self {
        Self {
            names: NameTable::new(IdentifierNamespace::Schema),
            members: Vec::new(),
            scalar_registrations: Vec::new(),
        }
    }
}

impl EncodedUniverseBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_name_table(names: NameTable) -> Self {
        Self {
            names,
            members: Vec::new(),
            scalar_registrations: Vec::new(),
        }
    }

    pub fn names(&self) -> &NameTable {
        &self.names
    }

    pub fn intern(&mut self, name: &str) -> Result<Identifier, name_table::NameTableError> {
        self.names.intern(Name::new(name))
    }

    pub fn primitive_at(
        &mut self,
        id: structural_codec::ScopedEncodedTypeId,
        name: Identifier,
        slot: ScalarSlot,
    ) {
        self.scalar_registrations.push((slot, id));
        self.register(id, name, MemberKind::Primitive);
    }

    pub fn leaf_at(&mut self, id: structural_codec::ScopedEncodedTypeId, name: Identifier) {
        self.register(id, name, MemberKind::Primitive);
    }

    pub fn field_meta_at(&mut self, id: structural_codec::ScopedEncodedTypeId, name: Identifier) {
        self.register(id, name, MemberKind::FieldMeta);
    }

    fn register(
        &mut self,
        id: structural_codec::ScopedEncodedTypeId,
        name: Identifier,
        kind: MemberKind,
    ) {
        self.members.push(UniverseType { id, name, kind });
    }

    fn is_sanctioned_builtin_scalar(
        &self,
        member: &UniverseType,
        builtin: BuiltinReference,
    ) -> bool {
        let required_slot = match builtin {
            BuiltinReference::Integer => Some(ScalarSlot::Integer),
            BuiltinReference::String => Some(ScalarSlot::Text),
            BuiltinReference::Boolean => Some(ScalarSlot::Boolean),
            BuiltinReference::Bytes => Some(ScalarSlot::Bytes),
            BuiltinReference::Vector | BuiltinReference::Optional | BuiltinReference::ScopeOf => {
                None
            }
        };
        matches!(member.kind, MemberKind::Primitive)
            && required_slot.is_some_and(|slot| {
                self.scalar_registrations
                    .iter()
                    .any(|(registered, id)| *registered == slot && *id == member.id)
            })
    }

    pub fn primitive(
        &mut self,
        id: structural_codec::ScopedEncodedTypeId,
        name: &str,
        slot: ScalarSlot,
    ) -> Result<Identifier, name_table::NameTableError> {
        let identifier = self.intern(name)?;
        self.primitive_at(id, identifier, slot);
        Ok(identifier)
    }

    pub fn primitive_leaf(
        &mut self,
        id: structural_codec::ScopedEncodedTypeId,
        name: &str,
    ) -> Result<Identifier, name_table::NameTableError> {
        let identifier = self.intern(name)?;
        self.leaf_at(id, identifier);
        Ok(identifier)
    }

    pub fn field_meta(
        &mut self,
        id: structural_codec::ScopedEncodedTypeId,
        name: &str,
    ) -> Result<Identifier, name_table::NameTableError> {
        let identifier = self.intern(name)?;
        self.field_meta_at(id, identifier);
        Ok(identifier)
    }

    pub fn declaration(
        &mut self,
        id: structural_codec::ScopedEncodedTypeId,
        declaration: EncodedDeclaration,
    ) {
        self.assigned_declaration(id, declaration.identifier(), declaration);
    }

    fn assigned_declaration(
        &mut self,
        id: structural_codec::ScopedEncodedTypeId,
        assigned: Identifier,
        declaration: EncodedDeclaration,
    ) {
        self.register(id, assigned, MemberKind::Declaration(declaration));
    }

    pub fn build(self, language: EncodedLanguage) -> Result<EncodedUniverse, UniverseError> {
        if self.names.namespace() != IdentifierNamespace::Schema {
            return Err(UniverseError::WrongNameTableHome {
                actual: self.names.namespace(),
            });
        }
        let builtins: HashMap<String, BuiltinReference> = BuiltinReference::ALL
            .into_iter()
            .map(|builtin| (builtin.spelling().to_owned(), builtin))
            .collect();
        let mut member_ids = BTreeSet::new();
        let mut member_names = HashSet::new();
        for member in &self.members {
            EncodedUniverse::validate_schema_identifier(member.name)?;
            let resolved_name = self.names.resolve(member.name)?;
            EncodedUniverse::validate_scoped_type_id(language, member.id)?;
            if let Some(builtin) = builtins.get(resolved_name.as_str()) {
                if !self.is_sanctioned_builtin_scalar(member, *builtin) {
                    return Err(
                        crate::error::StructuralRedefinition::new(member.name, *builtin).into(),
                    );
                }
            }
            if let MemberKind::Declaration(declaration) = &member.kind {
                if declaration.identifier() != member.name {
                    return Err(UniverseError::AssignedDeclarationIdentifierMismatch {
                        assigned: member.name,
                        declared: declaration.identifier(),
                    });
                }
                EncodedUniverse::validate_declaration_identifiers(
                    declaration,
                    &self.names,
                    &self.members,
                    &self.scalar_registrations,
                    language,
                )?;
            }
            if !member_ids.insert(member.id) {
                return Err(UniverseError::DuplicateMemberIdentity(member.id));
            }
            if !member_names.insert(member.name) {
                return Err(UniverseError::DuplicateMemberName(member.name));
            }
        }
        let mut scalar_slots = HashSet::new();
        for (slot, _) in &self.scalar_registrations {
            if !scalar_slots.insert(*slot) {
                return Err(UniverseError::DuplicateScalarSlot(*slot));
            }
        }
        let by_id = self
            .members
            .iter()
            .enumerate()
            .map(|(index, member)| (member.id, index))
            .collect();
        let by_name = self
            .members
            .iter()
            .map(|member| (member.name, member.id))
            .collect();
        let scalars: HashMap<_, _> = self.scalar_registrations.into_iter().collect();
        let scalar = |slot| scalars.get(&slot).copied();
        Ok(EncodedUniverse {
            language,
            integer: scalar(ScalarSlot::Integer),
            text: scalar(ScalarSlot::Text),
            boolean: scalar(ScalarSlot::Boolean),
            bytes: scalar(ScalarSlot::Bytes),
            names: self.names,
            members: self.members,
            by_id,
            by_name,
            builtins,
        })
    }
}

fn scoped(language: EncodedLanguage, local: u16) -> structural_codec::ScopedEncodedTypeId {
    match language {
        EncodedLanguage::Schema => structural_codec::ScopedEncodedTypeId::schema(local),
        EncodedLanguage::Logos => structural_codec::ScopedEncodedTypeId::logos(local),
        EncodedLanguage::Nomos => structural_codec::ScopedEncodedTypeId::nomos(local),
    }
}

pub const ENCODED_UNIVERSE: EncodedLanguage = EncodedLanguage::Schema;
