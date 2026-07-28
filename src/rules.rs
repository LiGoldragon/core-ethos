//! The ethos vocabulary's archived typed rule records.
//!
//! `structural-codec` owns evaluation.  These records only name this vocabulary's
//! fixed positions and the one metadata sidecar needed to validate the fixture
//! struct's authored layout against its Encoded declaration.

use structural_codec::{
    ApplicationDelimitedBody, ApplicationDelimitedHead, ApplicationDelimitedItems,
    ApplicationDelimitedRoot, AtomDescriptor, FieldEnd, FieldLink, FieldRole, Position,
    RuleCoproduct, SharedDescriptor, StableRoleId, StructuralRule, StructureRecord,
};

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct DelimitedRoot;

impl FieldRole for DelimitedRoot {
    const STABLE_ID: u16 = 1001;
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct DelimitedItems;

impl FieldRole for DelimitedItems {
    const STABLE_ID: u16 = 1002;
}

/// A root-delimited repeated rule absent from the kernel's small convenience
/// coproduct.  It is only archived data; the shared evaluator interprets it.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct DelimitedRule {
    root: Position<DelimitedRoot>,
    items: Position<DelimitedItems>,
}

impl DelimitedRule {
    pub(crate) fn new(
        boundary: raw_discovery::TriggerIdentifier,
        element: SharedDescriptor,
        minimum: u64,
        maximum: Option<u64>,
    ) -> Result<Self, structural_codec::AuthoringError> {
        let items = Position::try_new(SharedDescriptor::Repeated {
            minimum,
            maximum,
            element: Box::new(element),
        })?;
        Ok(Self {
            root: Position::try_new(SharedDescriptor::Delimited {
                boundary,
                content: items.role(),
            })?,
            items,
        })
    }
}

impl StructureRecord for DelimitedRule {
    type View<'record>
        = FieldLink<'record, DelimitedRoot, FieldLink<'record, DelimitedItems, FieldEnd>>
    where
        Self: 'record;

    fn root_role(&self) -> StableRoleId {
        self.root.role()
    }

    fn fields(&self) -> Self::View<'_> {
        FieldLink::new(&self.root, FieldLink::new(&self.items, FieldEnd))
    }
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct SignatureFirst;

impl FieldRole for SignatureFirst {
    const STABLE_ID: u16 = 1003;
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct SignatureSecond;

impl FieldRole for SignatureSecond {
    const STABLE_ID: u16 = 1004;
}

#[derive(
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub struct SignatureThird;

impl FieldRole for SignatureThird {
    const STABLE_ID: u16 = 1005;
}

/// An executable application-delimited record with three typed, unreachable
/// signature fields.  The fields are part of the table archive and let
/// `EncodedUniverse::validate_table` compare the authored fixture layout with
/// the three field references of `DatabaseMarker` without restoring the removed
/// positional-signature vector.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct SignatureApplicationDelimitedRule {
    application: Position<ApplicationDelimitedRoot>,
    head: Position<ApplicationDelimitedHead>,
    body: Position<ApplicationDelimitedBody>,
    items: Position<ApplicationDelimitedItems>,
    first: Position<SignatureFirst>,
    second: Position<SignatureSecond>,
    third: Position<SignatureThird>,
}

impl SignatureApplicationDelimitedRule {
    pub(crate) fn new(
        operator: raw_discovery::TriggerIdentifier,
        boundary: raw_discovery::TriggerIdentifier,
        head: SharedDescriptor,
        element: SharedDescriptor,
        minimum: u64,
        maximum: Option<u64>,
        signature: [Option<structural_codec::ScopedEncodedTypeId>; 3],
    ) -> Result<Self, structural_codec::AuthoringError> {
        let head = Position::try_new(head)?;
        let items = Position::try_new(SharedDescriptor::Repeated {
            minimum,
            maximum,
            element: Box::new(element),
        })?;
        let body = Position::try_new(SharedDescriptor::Delimited {
            boundary,
            content: items.role(),
        })?;
        Ok(Self {
            application: Position::try_new(SharedDescriptor::Application {
                operator,
                head: head.role(),
                payload: body.role(),
            })?,
            head,
            body,
            items,
            first: Self::signature_position(signature[0])?,
            second: Self::signature_position(signature[1])?,
            third: Self::signature_position(signature[2])?,
        })
    }

    fn signature_position<Role: FieldRole>(
        target: Option<structural_codec::ScopedEncodedTypeId>,
    ) -> Result<Position<Role>, structural_codec::AuthoringError> {
        Position::try_new(match target {
            Some(target) => SharedDescriptor::Delegate {
                target,
                payload: None,
            },
            None => SharedDescriptor::Atom(AtomDescriptor::any_case()),
        })
    }
}

impl StructureRecord for SignatureApplicationDelimitedRule {
    type View<'record> = FieldLink<
        'record,
        ApplicationDelimitedRoot,
        FieldLink<
            'record,
            ApplicationDelimitedHead,
            FieldLink<
                'record,
                ApplicationDelimitedBody,
                FieldLink<
                    'record,
                    ApplicationDelimitedItems,
                    FieldLink<
                        'record,
                        SignatureFirst,
                        FieldLink<
                            'record,
                            SignatureSecond,
                            FieldLink<'record, SignatureThird, FieldEnd>,
                        >,
                    >,
                >,
            >,
        >,
    >;

    fn root_role(&self) -> StableRoleId {
        self.application.role()
    }

    fn fields(&self) -> Self::View<'_> {
        FieldLink::new(
            &self.application,
            FieldLink::new(
                &self.head,
                FieldLink::new(
                    &self.body,
                    FieldLink::new(
                        &self.items,
                        FieldLink::new(
                            &self.first,
                            FieldLink::new(&self.second, FieldLink::new(&self.third, FieldEnd)),
                        ),
                    ),
                ),
            ),
        )
    }
}

/// The whole core-ethos vocabulary, combined as data-only coproducts.  The
/// evaluator remains one shared implementation for every branch.
pub type EthosRule =
    RuleCoproduct<RuleCoproduct<SignatureApplicationDelimitedRule, DelimitedRule>, StructuralRule>;

pub(crate) fn core_rule(rule: StructuralRule) -> EthosRule {
    RuleCoproduct::Right(rule)
}

pub(crate) fn delimited_rule(rule: DelimitedRule) -> EthosRule {
    RuleCoproduct::Left(RuleCoproduct::Right(rule))
}

pub(crate) fn signature_rule(rule: SignatureApplicationDelimitedRule) -> EthosRule {
    RuleCoproduct::Left(RuleCoproduct::Left(rule))
}
