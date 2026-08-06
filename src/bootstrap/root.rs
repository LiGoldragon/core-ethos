//! The one registry seating provisional root/order choices.

use signal_sema_translator::VocabularyEncodedId;

use super::catalog::{BootstrapPriorVocabulary, TextualMetadataSnapshot};
use super::error::BootstrapReadError;
use super::model::{EthosKind, InterfaceRole};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SectionSchema {
    Role(InterfaceRole),
    Declarations { admit_nomos: bool },
    Traits,
    PersistentDeclarations,
    Tables,
}

#[derive(Clone, Debug)]
pub(crate) struct RootSchema {
    pub(crate) kind: EthosKind,
    pub(crate) kind_identity: VocabularyEncodedId,
    pub(crate) sections: Vec<SectionSchema>,
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
                        SectionSchema::Role(InterfaceRole::Input),
                        SectionSchema::Role(InterfaceRole::Output),
                        SectionSchema::Role(InterfaceRole::Refusal),
                        SectionSchema::Declarations { admit_nomos: true },
                    ],
                },
                RootSchema {
                    kind: EthosKind::Nexus,
                    kind_identity: ids.nexus_kind.clone(),
                    sections: vec![
                        SectionSchema::Traits,
                        SectionSchema::Declarations { admit_nomos: false },
                    ],
                },
                RootSchema {
                    kind: EthosKind::Sema,
                    kind_identity: ids.sema_kind.clone(),
                    sections: vec![SectionSchema::PersistentDeclarations, SectionSchema::Tables],
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
}
