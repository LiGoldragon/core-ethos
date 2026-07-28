//! The six-slot document grammar as archived typed structural records.

use std::collections::BTreeMap;

use structural_codec::{
    AcceptedDecodeForm, AddressedStructuralTable, ApplicationDelimitedRule, ApplicationRule,
    AtomCase, AtomDescriptor, ConstructorCodec, DecodeFormId, EncodedConstructorId,
    EncodedLanguage, SharedDescriptor, StructuralEntry, StructuralRule,
    StructuralVocabularyIdentity, TableIdentityPayload, TargetLayoutIdentity, UnaryRule,
};

use crate::error::UniverseError;
use crate::fixture::{
    APPLICATION_OPERATOR, BRACE_BOUNDARY, SQUARE_BOUNDARY, standard_block_discovery,
    standard_textual_rendering, standard_token_profile,
};
use crate::rules::{DelimitedRule, EthosRule, core_rule, delimited_rule};

pub const TYPE_REFERENCE: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(100);
pub const FIELD: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(101);
pub const DECLARATION: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(102);
pub const TYPES_BLOCK: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(103);
pub const INTERFACE_VARIANT: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(104);
pub const INTERFACE: structural_codec::ScopedEncodedTypeId =
    structural_codec::ScopedEncodedTypeId::schema(105);

pub const DOCUMENT_SLOTS: usize = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceConstructor {
    Name,
    Application,
}

impl ReferenceConstructor {
    pub const ALL: [Self; 2] = [Self::Name, Self::Application];

    pub fn index(self) -> u16 {
        match self {
            Self::Name => 0,
            Self::Application => 1,
        }
    }

    pub fn from_index(index: u16) -> Option<Self> {
        Self::ALL.get(index as usize).copied()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationConstructor {
    Newtype,
    Struct,
    Enumeration,
}

impl DeclarationConstructor {
    pub const ALL: [Self; 3] = [Self::Newtype, Self::Struct, Self::Enumeration];

    pub fn index(self) -> u16 {
        match self {
            Self::Newtype => 0,
            Self::Struct => 1,
            Self::Enumeration => 2,
        }
    }

    pub fn from_index(index: u16) -> Option<Self> {
        Self::ALL.get(index as usize).copied()
    }
}

#[derive(Clone, Debug)]
pub struct EthosDocumentGrammar {
    table: AddressedStructuralTable<EthosRule>,
}

impl EthosDocumentGrammar {
    pub fn build() -> Result<Self, UniverseError> {
        let profile = standard_token_profile();
        let entries = DocumentTableAuthor
            .entries()?
            .into_iter()
            .map(|entry| (entry.encoded_type(), entry))
            .collect::<BTreeMap<_, _>>();
        let table = AddressedStructuralTable::seal(
            TableIdentityPayload::new(
                EncodedLanguage::Schema,
                TargetLayoutIdentity::derive(b"core-schema six-slot document grammar"),
                profile.identity(),
                StructuralVocabularyIdentity::language(b"core-schema document typed vocabulary"),
                standard_block_discovery(),
                standard_textual_rendering(),
                entries,
            ),
            &profile,
        )?;
        Ok(Self { table })
    }

    pub fn table(&self) -> &AddressedStructuralTable<EthosRule> {
        &self.table
    }
}

struct DocumentTableAuthor;

impl DocumentTableAuthor {
    fn entries(&self) -> Result<Vec<StructuralEntry<EthosRule>>, UniverseError> {
        Ok(vec![
            self.type_reference_entry(),
            Self::field_entry(),
            self.declaration_entry(),
            Self::types_block_entry(),
            Self::interface_variant_entry(),
            Self::interface_entry(),
        ])
    }

    fn type_reference_entry(&self) -> StructuralEntry<EthosRule> {
        let name = core_rule(StructuralRule::Unary(
            UnaryRule::new(SharedDescriptor::Atom(AtomDescriptor::with_case(
                AtomCase::PascalCase,
            )))
            .expect("kernel role"),
        ));
        let application = core_rule(StructuralRule::Application(
            ApplicationRule::new(
                APPLICATION_OPERATOR,
                SharedDescriptor::Atom(AtomDescriptor::with_case(AtomCase::PascalCase)),
                SharedDescriptor::Delegate {
                    target: TYPE_REFERENCE,
                    payload: None,
                },
            )
            .expect("kernel roles"),
        ));
        Self::entry(
            TYPE_REFERENCE,
            vec![
                Self::codec(TYPE_REFERENCE, ReferenceConstructor::Name.index(), name),
                Self::codec(
                    TYPE_REFERENCE,
                    ReferenceConstructor::Application.index(),
                    application,
                ),
            ],
        )
    }

    fn field_entry() -> StructuralEntry<EthosRule> {
        let rule = core_rule(StructuralRule::Unary(
            UnaryRule::new(SharedDescriptor::Atom(AtomDescriptor::with_case(
                AtomCase::PascalCase,
            )))
            .expect("kernel role"),
        ));
        Self::entry(FIELD, vec![Self::codec(FIELD, 0, rule)])
    }

    fn declaration_entry(&self) -> StructuralEntry<EthosRule> {
        let newtype = core_rule(StructuralRule::Application(
            ApplicationRule::new(
                APPLICATION_OPERATOR,
                SharedDescriptor::Atom(AtomDescriptor::with_case(AtomCase::PascalCase)),
                SharedDescriptor::Delegate {
                    target: TYPE_REFERENCE,
                    payload: None,
                },
            )
            .expect("kernel roles"),
        ));
        let structure = core_rule(StructuralRule::ApplicationDelimited(
            ApplicationDelimitedRule::new(
                APPLICATION_OPERATOR,
                BRACE_BOUNDARY,
                SharedDescriptor::Atom(AtomDescriptor::with_case(AtomCase::PascalCase)),
                SharedDescriptor::Delegate {
                    target: FIELD,
                    payload: None,
                },
                0,
                None,
            )
            .expect("kernel roles"),
        ));
        let enumeration = core_rule(StructuralRule::ApplicationDelimited(
            ApplicationDelimitedRule::new(
                APPLICATION_OPERATOR,
                SQUARE_BOUNDARY,
                SharedDescriptor::Atom(AtomDescriptor::with_case(AtomCase::PascalCase)),
                SharedDescriptor::Atom(AtomDescriptor::with_case(AtomCase::PascalCase)),
                0,
                None,
            )
            .expect("kernel roles"),
        ));
        Self::entry(
            DECLARATION,
            vec![
                Self::codec(
                    DECLARATION,
                    DeclarationConstructor::Newtype.index(),
                    newtype,
                ),
                Self::codec(
                    DECLARATION,
                    DeclarationConstructor::Struct.index(),
                    structure,
                ),
                Self::codec(
                    DECLARATION,
                    DeclarationConstructor::Enumeration.index(),
                    enumeration,
                ),
            ],
        )
    }

    fn types_block_entry() -> StructuralEntry<EthosRule> {
        let rule = delimited_rule(
            DelimitedRule::new(
                BRACE_BOUNDARY,
                SharedDescriptor::Delegate {
                    target: DECLARATION,
                    payload: None,
                },
                0,
                None,
            )
            .expect("ethos roles"),
        );
        Self::entry(TYPES_BLOCK, vec![Self::codec(TYPES_BLOCK, 0, rule)])
    }

    fn interface_variant_entry() -> StructuralEntry<EthosRule> {
        let rule = core_rule(StructuralRule::Application(
            ApplicationRule::new(
                APPLICATION_OPERATOR,
                SharedDescriptor::Atom(AtomDescriptor::with_case(AtomCase::PascalCase)),
                SharedDescriptor::Delegate {
                    target: TYPE_REFERENCE,
                    payload: None,
                },
            )
            .expect("kernel roles"),
        ));
        Self::entry(
            INTERFACE_VARIANT,
            vec![Self::codec(INTERFACE_VARIANT, 0, rule)],
        )
    }

    fn interface_entry() -> StructuralEntry<EthosRule> {
        let rule = delimited_rule(
            DelimitedRule::new(
                SQUARE_BOUNDARY,
                SharedDescriptor::Delegate {
                    target: INTERFACE_VARIANT,
                    payload: None,
                },
                0,
                None,
            )
            .expect("ethos roles"),
        );
        Self::entry(INTERFACE, vec![Self::codec(INTERFACE, 0, rule)])
    }

    fn codec(
        type_id: structural_codec::ScopedEncodedTypeId,
        constructor: u16,
        rule: EthosRule,
    ) -> ConstructorCodec<EthosRule> {
        ConstructorCodec::new(
            EncodedConstructorId::under(type_id, constructor),
            vec![AcceptedDecodeForm::new(DecodeFormId::new(0), rule.clone())],
            rule,
        )
    }

    fn entry(
        type_id: structural_codec::ScopedEncodedTypeId,
        constructors: Vec<ConstructorCodec<EthosRule>>,
    ) -> StructuralEntry<EthosRule> {
        StructuralEntry::new(type_id, constructors)
    }
}

#[cfg(test)]
mod tests {
    use super::EthosDocumentGrammar;

    #[test]
    fn full_grammar_seals() {
        EthosDocumentGrammar::build().expect("the complete document grammar seals");
    }
}
