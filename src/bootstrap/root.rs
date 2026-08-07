//! The one registry seating provisional root/order choices.

use name_table::EncodedName;

use super::catalog::{BootstrapPriorVocabulary, TextualMetadataSnapshot};
use super::error::BootstrapReadError;
use super::model::{
    BootstrapBody, Declaration, EthosKind, InterfaceRole, RoleEntry, TableDeclaration,
    TraitDeclaration, TypeDeclaration,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapSectionSchema {
    Role(InterfaceRole),
    Declarations,
    Traits,
    PersistentDeclarations,
    Tables,
}

#[derive(Clone, Debug)]
pub(crate) struct RootSchema {
    pub(crate) kind: EthosKind,
    pub(crate) kind_identity: EncodedName,
    pub(crate) sections: Vec<BootstrapSectionSchema>,
}

pub(crate) enum RootSemanticSectionRef<'a> {
    Role(&'a [RoleEntry]),
    Declarations(&'a [Declaration]),
    Traits(&'a [TraitDeclaration]),
    PersistentDeclarations(&'a [TypeDeclaration]),
    Tables(&'a [TableDeclaration]),
}

impl RootSchema {
    pub(crate) fn semantic_sections<'a>(
        &self,
        body: &'a BootstrapBody,
    ) -> Result<Vec<RootSemanticSectionRef<'a>>, BootstrapReadError> {
        if body.kind() != self.kind {
            return Err(BootstrapReadError::HeaderBodyMismatch {
                header: self.kind,
                body: body.kind(),
            });
        }
        self.sections
            .iter()
            .map(|schema| match (schema, body) {
                (
                    BootstrapSectionSchema::Role(InterfaceRole::Input),
                    BootstrapBody::Interface(body),
                ) => Ok(RootSemanticSectionRef::Role(&body.inputs)),
                (
                    BootstrapSectionSchema::Role(InterfaceRole::Output),
                    BootstrapBody::Interface(body),
                ) => Ok(RootSemanticSectionRef::Role(&body.outputs)),
                (
                    BootstrapSectionSchema::Role(InterfaceRole::Refusal),
                    BootstrapBody::Interface(body),
                ) => Ok(RootSemanticSectionRef::Role(&body.refusals)),
                (BootstrapSectionSchema::Declarations { .. }, BootstrapBody::Interface(body)) => {
                    Ok(RootSemanticSectionRef::Declarations(&body.types))
                }
                (BootstrapSectionSchema::Traits, BootstrapBody::Nexus(body)) => {
                    Ok(RootSemanticSectionRef::Traits(&body.traits))
                }
                (BootstrapSectionSchema::Declarations { .. }, BootstrapBody::Nexus(body)) => {
                    Ok(RootSemanticSectionRef::Declarations(&body.types))
                }
                (BootstrapSectionSchema::PersistentDeclarations, BootstrapBody::Sema(body)) => Ok(
                    RootSemanticSectionRef::PersistentDeclarations(&body.record_types),
                ),
                (BootstrapSectionSchema::Tables, BootstrapBody::Sema(body)) => {
                    Ok(RootSemanticSectionRef::Tables(&body.tables))
                }
                _ => Err(BootstrapReadError::InvalidPreparedModel(
                    "root registry section cannot project this body",
                )),
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RootSchemaRegistry {
    roots: Vec<RootSchema>,
}

impl RootSchemaRegistry {
    pub(crate) fn new(priors: &BootstrapPriorVocabulary) -> Self {
        let ids = priors.identities();
        Self {
            roots: vec![
                RootSchema {
                    kind: EthosKind::Interface,
                    kind_identity: ids.interface_kind.clone(),
                    sections: vec![
                        BootstrapSectionSchema::Role(InterfaceRole::Input),
                        BootstrapSectionSchema::Role(InterfaceRole::Output),
                        BootstrapSectionSchema::Role(InterfaceRole::Refusal),
                        BootstrapSectionSchema::Declarations,
                    ],
                },
                RootSchema {
                    kind: EthosKind::Nexus,
                    kind_identity: ids.nexus_kind.clone(),
                    sections: vec![
                        BootstrapSectionSchema::Traits,
                        BootstrapSectionSchema::Declarations,
                    ],
                },
                RootSchema {
                    kind: EthosKind::Sema,
                    kind_identity: ids.sema_kind.clone(),
                    sections: vec![
                        BootstrapSectionSchema::PersistentDeclarations,
                        BootstrapSectionSchema::Tables,
                    ],
                },
            ],
        }
    }

    pub(crate) fn resolve_header<'a>(
        &'a self,
        spelling: &str,
        metadata: &TextualMetadataSnapshot,
    ) -> Result<&'a RootSchema, BootstrapReadError> {
        self.roots
            .iter()
            .find(|root| metadata.spelling(&root.kind_identity) == Some(spelling))
            .ok_or_else(|| BootstrapReadError::UnknownFileKind(spelling.to_owned()))
    }

    pub(crate) fn for_kind(&self, kind: EthosKind) -> &RootSchema {
        self.roots
            .iter()
            .find(|root| root.kind == kind)
            .expect("the closed registry seats every EthosKind")
    }

    pub(crate) fn section_order(&self, kind: EthosKind) -> &[BootstrapSectionSchema] {
        &self.for_kind(kind).sections
    }
}
