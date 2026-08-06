//! Allocation-free occurrence planning followed by exact-assignment sealing.

use std::cmp::Ordering as CompareOrdering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

use raw_discovery::SourceBound;
use signal_sema_translator::VocabularyEncodedId;

use super::catalog::*;
use super::error::{BootstrapBuildError, BootstrapReadError};
use super::grammar::{
    BootstrapGrammar, BootstrapGrammarIdentities, Delimiter, StructuralDocumentPlan, SyntaxNode,
};
use super::model::*;
use super::root::{
    BootstrapSectionSchema as SectionSchema, RootSchema, RootSchemaRegistry, RootSemanticSectionRef,
};

/// Ephemeral, plan-local handle for one authored declaration occurrence.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeclarationOccurrence {
    plan: u64,
    ordinal: u32,
}

impl DeclarationOccurrence {
    pub const fn ordinal(self) -> u32 {
        self.ordinal
    }
}

/// The exact scope in which an authored visible name must be unique.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlannedScope {
    Module,
    Enum(DeclarationOccurrence),
    Trait(DeclarationOccurrence),
}

/// Semantic purpose of one authored naming request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationPurpose {
    Type,
    PersistentType,
    Variant,
    Trait,
    Method,
    Table,
    StreamInitiation,
}

/// One exact authored occurrence discovered before identity authority is used.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedDeclaration {
    occurrence: DeclarationOccurrence,
    spelling: String,
    bound: SourceBound,
    scope: PlannedScope,
    purpose: DeclarationPurpose,
}

impl PlannedDeclaration {
    pub const fn occurrence(&self) -> DeclarationOccurrence {
        self.occurrence
    }

    pub fn spelling(&self) -> &str {
        &self.spelling
    }

    pub const fn bound(&self) -> SourceBound {
        self.bound
    }

    pub const fn scope(&self) -> PlannedScope {
        self.scope
    }

    pub const fn purpose(&self) -> DeclarationPurpose {
        self.purpose
    }
}

/// Allocation-free result of structural discovery and all assignment-independent
/// schema/cardinality checks.
#[derive(Clone, Debug)]
pub struct BootstrapReadPlan {
    structural: StructuralDocumentPlan,
    header: EthosHeader,
    imports: Vec<ImportEntry>,
    declarations: Vec<PlannedDeclaration>,
}

impl BootstrapReadPlan {
    pub const fn header(&self) -> EthosHeader {
        self.header
    }

    pub fn imports(&self) -> &[ImportEntry] {
        &self.imports
    }

    pub fn declarations(&self) -> &[PlannedDeclaration] {
        &self.declarations
    }
}

/// One caller-issued identity for one authored occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamingAssignment {
    pub occurrence: DeclarationOccurrence,
    pub encoded_name: VocabularyEncodedId,
    pub disposition: IdentityDisposition,
}

/// Naming authority's statement about whether an identity is being reused or
/// minted. New identities carry their authority-supplied canonical bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdentityDisposition {
    Existing,
    New { canonical_bytes: Vec<u8> },
}

/// One authority disposition for a generated identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssignedIdentity {
    pub encoded_name: VocabularyEncodedId,
    pub disposition: IdentityDisposition,
}

/// Publicly constructible, explicitly unvalidated transaction parts. External
/// stores and writers accept [`PreparedBootstrapTransaction`] instead; callers
/// obtain that wrapper through [`BootstrapReader::validate_draft`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedBootstrapDraft {
    pub decoded: DecodedBootstrap,
    pub generated_streams: Vec<PreparedStreamGeneration>,
    pub schema_additions: IdentitySchemaCatalog,
    pub naming_transition: TextualMetadataTransition,
    pub identity_dispositions: BTreeMap<VocabularyEncodedId, IdentityDisposition>,
    pub canonical_order: CanonicalIdentityOrder,
}

/// The exact prepared proposal presented to the authority injected into one
/// reader. Callers can inspect this request but cannot construct one.
pub struct BootstrapNamingAuthorityRequest<'a> {
    transaction: &'a PreparedBootstrapDraft,
}

impl BootstrapNamingAuthorityRequest<'_> {
    pub const fn transaction(&self) -> &PreparedBootstrapDraft {
        self.transaction
    }

    pub const fn transition(&self) -> &TextualMetadataTransition {
        &self.transaction.naming_transition
    }

    pub const fn identity_dispositions(
        &self,
    ) -> &BTreeMap<VocabularyEncodedId, IdentityDisposition> {
        &self.transaction.identity_dispositions
    }
}

/// Authenticity boundary for naming proposals. The reader validates structural
/// consistency separately; only this injected authority can issue and verify its
/// configuration-specific receipt.
pub trait BootstrapNamingAuthority {
    type Proof;
    type Receipt: Clone + std::fmt::Debug + Eq;

    fn authorize(
        &self,
        request: BootstrapNamingAuthorityRequest<'_>,
        proof: &Self::Proof,
    ) -> Option<Self::Receipt>;

    fn verify_receipt(
        &self,
        request: BootstrapNamingAuthorityRequest<'_>,
        receipt: &Self::Receipt,
    ) -> bool;
}

/// Checked syntactic assignment set; plan-relative exactness is enforced at seal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamingAssignments {
    by_occurrence: BTreeMap<DeclarationOccurrence, NamingAssignment>,
}

impl NamingAssignments {
    pub fn new(assignments: Vec<NamingAssignment>) -> Result<Self, BootstrapReadError> {
        let mut by_occurrence = BTreeMap::new();
        for assignment in assignments {
            if by_occurrence
                .insert(assignment.occurrence, assignment.clone())
                .is_some()
            {
                return Err(BootstrapReadError::DuplicateAssignment(
                    assignment.occurrence.ordinal(),
                ));
            }
        }
        Ok(Self { by_occurrence })
    }

    fn get(&self, occurrence: DeclarationOccurrence) -> Option<&NamingAssignment> {
        self.by_occurrence.get(&occurrence)
    }
}

/// The two additional identities required by one authored Stream occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedStreamAssignment {
    pub source: DeclarationOccurrence,
    pub initiation: AssignedIdentity,
    pub termination: AssignedIdentity,
}

/// Exact generated Stream assignments, separate from authored naming requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedStreamAssignments {
    by_source: BTreeMap<DeclarationOccurrence, GeneratedStreamAssignment>,
}

impl GeneratedStreamAssignments {
    pub fn new(assignments: Vec<GeneratedStreamAssignment>) -> Result<Self, BootstrapReadError> {
        let mut by_source = BTreeMap::new();
        for assignment in assignments {
            let source = assignment.source;
            if by_source.insert(source, assignment).is_some() {
                return Err(BootstrapReadError::DuplicateGeneratedStreamAssignment(
                    source.ordinal(),
                ));
            }
        }
        Ok(Self { by_source })
    }

    fn get(&self, source: DeclarationOccurrence) -> Option<&GeneratedStreamAssignment> {
        self.by_source.get(&source)
    }
}

fn identity_dispositions(
    assignments: &NamingAssignments,
    generated: &GeneratedStreamAssignments,
) -> BTreeMap<VocabularyEncodedId, IdentityDisposition> {
    assignments
        .by_occurrence
        .values()
        .map(|assignment| {
            (
                assignment.encoded_name.clone(),
                assignment.disposition.clone(),
            )
        })
        .chain(generated.by_source.values().flat_map(|assignment| {
            [&assignment.initiation, &assignment.termination]
                .map(|assigned| (assigned.encoded_name.clone(), assigned.disposition.clone()))
        }))
        .collect()
}

/// Complete, validated meaning and schema/name updates prepared for an external
/// authority to commit atomically. The reader has not committed storage.
#[derive(Clone, Debug, Eq, PartialEq)]
struct VerifiedNamingAuthorization<Receipt> {
    transition: TextualMetadataTransition,
    identity_dispositions: BTreeMap<VocabularyEncodedId, IdentityDisposition>,
    receipt: Receipt,
}

pub struct PreparedBootstrapTransaction<Authority: BootstrapNamingAuthority> {
    decoded: DecodedBootstrap,
    generated_streams: Vec<PreparedStreamGeneration>,
    schema_additions: IdentitySchemaCatalog,
    naming_authorization: VerifiedNamingAuthorization<Authority::Receipt>,
    canonical_order: CanonicalIdentityOrder,
}

impl<Authority: BootstrapNamingAuthority> Clone for PreparedBootstrapTransaction<Authority> {
    fn clone(&self) -> Self {
        Self {
            decoded: self.decoded.clone(),
            generated_streams: self.generated_streams.clone(),
            schema_additions: self.schema_additions.clone(),
            naming_authorization: self.naming_authorization.clone(),
            canonical_order: self.canonical_order.clone(),
        }
    }
}

impl<Authority: BootstrapNamingAuthority> std::fmt::Debug
    for PreparedBootstrapTransaction<Authority>
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedBootstrapTransaction")
            .field("decoded", &self.decoded)
            .field("generated_streams", &self.generated_streams)
            .field("schema_additions", &self.schema_additions)
            .field("naming_authorization", &self.naming_authorization)
            .field("canonical_order", &self.canonical_order)
            .finish()
    }
}

impl<Authority: BootstrapNamingAuthority> PartialEq for PreparedBootstrapTransaction<Authority> {
    fn eq(&self, other: &Self) -> bool {
        self.decoded == other.decoded
            && self.generated_streams == other.generated_streams
            && self.schema_additions == other.schema_additions
            && self.naming_authorization == other.naming_authorization
            && self.canonical_order == other.canonical_order
    }
}

impl<Authority: BootstrapNamingAuthority> Eq for PreparedBootstrapTransaction<Authority> {}

impl<Authority: BootstrapNamingAuthority> PreparedBootstrapTransaction<Authority> {
    pub const fn archive_status(&self) -> BootstrapArchiveStatus {
        BootstrapArchiveStatus::NotYetArchived
    }

    pub const fn decoded(&self) -> &DecodedBootstrap {
        &self.decoded
    }

    pub fn generated_streams(&self) -> &[PreparedStreamGeneration] {
        &self.generated_streams
    }

    pub const fn schema_additions(&self) -> &IdentitySchemaCatalog {
        &self.schema_additions
    }

    pub const fn naming_transition(&self) -> &TextualMetadataTransition {
        &self.naming_authorization.transition
    }

    pub const fn identity_dispositions(
        &self,
    ) -> &BTreeMap<VocabularyEncodedId, IdentityDisposition> {
        &self.naming_authorization.identity_dispositions
    }

    pub const fn canonical_order(&self) -> &CanonicalIdentityOrder {
        &self.canonical_order
    }

    /// The authority-issued receipt is exposed read-only for storage boundaries.
    pub const fn naming_authority_receipt(&self) -> &Authority::Receipt {
        &self.naming_authorization.receipt
    }

    /// Re-verify this exact transaction with the authority configuration that
    /// brands its type.
    pub fn verify_naming_authority(&self, authority: &Authority) -> Result<(), BootstrapReadError> {
        let draft = self.to_draft();
        if authority.verify_receipt(
            BootstrapNamingAuthorityRequest {
                transaction: &draft,
            },
            self.naming_authority_receipt(),
        ) {
            Ok(())
        } else {
            Err(BootstrapReadError::NamingAuthorityReceiptRejected)
        }
    }

    pub fn to_draft(&self) -> PreparedBootstrapDraft {
        PreparedBootstrapDraft {
            decoded: self.decoded.clone(),
            generated_streams: self.generated_streams.clone(),
            schema_additions: self.schema_additions.clone(),
            naming_transition: self.naming_authorization.transition.clone(),
            identity_dispositions: self.naming_authorization.identity_dispositions.clone(),
            canonical_order: self.canonical_order.clone(),
        }
    }
}

/// Shared two-phase reader. Allocation, persistence, and commit authority stay
/// outside this type.
#[derive(Clone, Debug)]
pub struct BootstrapReader<Authority> {
    grammar: BootstrapGrammar,
    catalog: BootstrapCatalog,
    roots: RootSchemaRegistry,
    naming_authority: Authority,
}

impl<Authority: BootstrapNamingAuthority> BootstrapReader<Authority> {
    pub fn build(
        grammar_identities: BootstrapGrammarIdentities,
        catalog: BootstrapCatalog,
        naming_authority: Authority,
    ) -> Result<Self, BootstrapBuildError> {
        let roots = RootSchemaRegistry::new(catalog.priors());
        Ok(Self {
            grammar: BootstrapGrammar::build(grammar_identities)?,
            catalog,
            roots,
            naming_authority,
        })
    }

    /// Discover structure, select the registered root, validate all independent
    /// cardinality/arity laws, and enumerate authored occurrences only.
    pub fn plan(&self, source: &str) -> Result<BootstrapReadPlan, BootstrapReadError> {
        static NEXT_PLAN: AtomicU64 = AtomicU64::new(1);
        let structural = self.grammar.plan(source)?;
        let (header, imports, body, root) = self.envelope(&structural)?;
        let external = ExistingVisibleEnvironment::new(&imports, &self.catalog)?;
        let mut planner = OccurrencePlanner::new(NEXT_PLAN.fetch_add(1, Ordering::Relaxed));
        let fields = expect_delimited(body, Delimiter::Brace, "kind-selected body")?;
        if fields.len() != root.sections.len() {
            return Err(unexpected_node(body, "the registered body section arity"));
        }
        for (section, field) in root.sections.iter().zip(fields) {
            planner.discover_section(*section, field, &external, &self.catalog)?;
        }
        Ok(BootstrapReadPlan {
            structural,
            header,
            imports,
            declarations: planner.declarations,
        })
    }

    /// Seal using exact authored dispositions, exact Stream-generated
    /// dispositions, a before-to-after textual proposal, and its authority proof.
    /// The result remains a prepared transaction; no authority state is mutated.
    pub fn seal(
        &self,
        plan: &BootstrapReadPlan,
        assignments: &NamingAssignments,
        generated: &GeneratedStreamAssignments,
        naming_transition: &TextualMetadataTransition,
        authority_proof: &Authority::Proof,
    ) -> Result<PreparedBootstrapTransaction<Authority>, BootstrapReadError> {
        let identity_dispositions = identity_dispositions(assignments, generated);
        let canonical_order =
            self.validate_assignment_inputs(plan, assignments, generated, naming_transition)?;
        let schema_additions = self.schema_additions(plan, assignments, generated)?;
        let schemas = SchemaView {
            existing: self.catalog.schemas(),
            additions: &schema_additions,
        };
        let environment = ResolutionEnvironment::new(
            plan,
            assignments,
            naming_transition.after(),
            &self.catalog,
            schemas,
            &canonical_order,
        )?;
        let (_, _, body, root) = self.envelope(&plan.structural)?;
        let fields = expect_delimited(body, Delimiter::Brace, "registered body")?;
        let mut cursor = AssignmentCursor::new(&plan.declarations, assignments);
        let mut prepared_streams = Vec::new();
        let mut sections = Vec::new();
        for (section, field) in root.sections.iter().zip(fields) {
            sections.push(reify_section(
                *section,
                field,
                &environment,
                &mut cursor,
                generated,
                &mut prepared_streams,
                self.catalog.priors(),
            )?);
        }
        cursor.finish()?;
        sort_by_identity(
            &mut prepared_streams,
            |stream| &stream.output.name,
            &canonical_order,
        )?;
        let decoded = DecodedBootstrap {
            document: BootstrapDocument {
                header: plan.header,
                body: assemble_body(root, sections, self.catalog.priors(), &canonical_order)?,
            },
            source: BootstrapSourceProjection {
                imports: canonicalize_imports(
                    &plan.imports,
                    naming_transition.after(),
                    &canonical_order,
                )?,
            },
        };
        self.authorize_draft(
            PreparedBootstrapDraft {
                decoded,
                generated_streams: prepared_streams,
                schema_additions,
                naming_transition: naming_transition.clone(),
                identity_dispositions,
                canonical_order,
            },
            authority_proof,
        )
    }

    /// Validate untrusted transaction parts and return the invariant-bearing
    /// wrapper accepted by writers and external stores.
    pub fn validate_draft(
        &self,
        draft: PreparedBootstrapDraft,
        authority_proof: &Authority::Proof,
    ) -> Result<PreparedBootstrapTransaction<Authority>, BootstrapReadError> {
        self.authorize_draft(draft, authority_proof)
    }

    fn authorize_draft(
        &self,
        draft: PreparedBootstrapDraft,
        authority_proof: &Authority::Proof,
    ) -> Result<PreparedBootstrapTransaction<Authority>, BootstrapReadError> {
        let receipt = self
            .naming_authority
            .authorize(
                BootstrapNamingAuthorityRequest {
                    transaction: &draft,
                },
                authority_proof,
            )
            .ok_or(BootstrapReadError::NamingAuthorityRejected)?;
        let transaction = PreparedBootstrapTransaction {
            decoded: draft.decoded,
            generated_streams: draft.generated_streams,
            schema_additions: draft.schema_additions,
            naming_authorization: VerifiedNamingAuthorization {
                transition: draft.naming_transition,
                identity_dispositions: draft.identity_dispositions,
                receipt,
            },
            canonical_order: draft.canonical_order,
        };
        self.validate_transaction(&transaction)?;
        Ok(transaction)
    }

    /// Re-verify authority authenticity and every prepared-model invariant.
    pub fn validate_transaction(
        &self,
        transaction: &PreparedBootstrapTransaction<Authority>,
    ) -> Result<(), BootstrapReadError> {
        transaction.verify_naming_authority(&self.naming_authority)?;
        self.validate_prepared(transaction)
    }

    pub fn archive_status(&self) -> BootstrapArchiveStatus {
        BootstrapArchiveStatus::NotYetArchived
    }

    pub fn section_order(&self, kind: EthosKind) -> &[super::root::BootstrapSectionSchema] {
        self.roots.section_order(kind)
    }

    pub(crate) fn catalog(&self) -> &BootstrapCatalog {
        &self.catalog
    }

    pub(crate) fn roots(&self) -> &RootSchemaRegistry {
        &self.roots
    }

    pub(crate) fn validate_prepared(
        &self,
        transaction: &PreparedBootstrapTransaction<Authority>,
    ) -> Result<(), BootstrapReadError> {
        let decoded = &transaction.decoded;
        if decoded.document.header.kind != decoded.document.body.kind() {
            return Err(BootstrapReadError::HeaderBodyMismatch {
                header: decoded.document.header.kind,
                body: decoded.document.body.kind(),
            });
        }
        if !self
            .catalog
            .versions()
            .supports(decoded.document.header.version)
        {
            return Err(BootstrapReadError::UnsupportedVersion {
                found: decoded.document.header.version,
                supported: self.catalog.versions().supported(),
            });
        }
        if transaction.naming_transition().before() != self.catalog.metadata() {
            return Err(BootstrapReadError::MetadataTransitionBeforeMismatch);
        }
        let snapshot = transaction.naming_transition().after();
        for identity in self.catalog.priors().fixed_identities() {
            snapshot
                .record(identity)
                .ok_or_else(|| BootstrapReadError::MissingMetadataIdentity(identity.clone()))?;
        }
        for import in &decoded.source.imports {
            validate_module_path(&import.module_path)?;
            if import.imported_names.is_empty() {
                return Err(BootstrapReadError::EmptyImportSelectors);
            }
            for name in &import.imported_names {
                validate_visible_name(name)?;
                snapshot
                    .identity_at(&import.module_path, None, name)
                    .ok_or_else(|| BootstrapReadError::MissingTextualLookup {
                        module_path: import.module_path.clone(),
                        name: name.clone(),
                    })?;
            }
        }
        let schemas = SchemaView {
            existing: self.catalog.schemas(),
            additions: &transaction.schema_additions,
        };
        let mut validator = PreparedModelValidator {
            schemas,
            snapshot,
            current_module: self.catalog.current_module_path(),
            expected_additions: BTreeMap::new(),
            seen_existing: BTreeSet::new(),
            streams: BTreeMap::new(),
            canonical_order: &transaction.canonical_order,
            priors: self.catalog.priors(),
            new_identities: transaction
                .identity_dispositions()
                .iter()
                .filter_map(|(identity, disposition)| match disposition {
                    IdentityDisposition::New { .. } => Some(identity.clone()),
                    IdentityDisposition::Existing => None,
                })
                .collect(),
        };
        for identity in snapshot.identities() {
            if schemas.get(identity).is_none() {
                return Err(BootstrapReadError::ExtraMetadataIdentity(identity.clone()));
            }
            if transaction.canonical_order.bytes(identity).is_none() {
                return Err(BootstrapReadError::MissingCanonicalIdentity(
                    identity.clone(),
                ));
            }
        }
        validator.validate_body(
            &decoded.document.body,
            self.roots.for_kind(decoded.document.header.kind),
        )?;
        validator.validate_streams(&transaction.generated_streams, self.catalog.priors())?;
        validator.validate_additions(&transaction.schema_additions)?;
        let declared_identities = validator.seen_existing.clone();
        self.validate_prepared_dispositions(transaction, &declared_identities)?;
        self.validate_writer_visibility(transaction, schemas)
    }

    fn validate_prepared_dispositions(
        &self,
        transaction: &PreparedBootstrapTransaction<Authority>,
        declared_identities: &BTreeSet<VocabularyEncodedId>,
    ) -> Result<(), BootstrapReadError> {
        let disposition_identities = transaction
            .identity_dispositions()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        if &disposition_identities != declared_identities {
            return Err(BootstrapReadError::InvalidPreparedModel(
                "authority dispositions do not exactly equal prepared declarations",
            ));
        }
        let mut canonical_additions = Vec::new();
        for (identity, disposition) in transaction.identity_dispositions() {
            match disposition {
                IdentityDisposition::Existing => {
                    if self.catalog.priors().is_fixed_identity(identity)
                        || self.catalog.schemas().get(identity).is_none()
                        || transaction.schema_additions.get(identity).is_some()
                    {
                        return Err(BootstrapReadError::ExistingAssignmentMissing {
                            identity: identity.clone(),
                        });
                    }
                    if transaction.canonical_order.bytes(identity)
                        != self.catalog.canonical_order().bytes(identity)
                    {
                        return Err(BootstrapReadError::MissingCanonicalIdentity(
                            identity.clone(),
                        ));
                    }
                }
                IdentityDisposition::New { canonical_bytes } => {
                    if self.catalog.schemas().contains(identity)
                        || self.catalog.metadata().record(identity).is_some()
                        || self.catalog.canonical_order().contains(identity)
                    {
                        return Err(BootstrapReadError::NewAssignmentAlreadyExists {
                            identity: identity.clone(),
                        });
                    }
                    if transaction.schema_additions.get(identity).is_none()
                        || transaction.canonical_order.bytes(identity)
                            != Some(canonical_bytes.as_slice())
                    {
                        return Err(BootstrapReadError::MissingCanonicalIdentity(
                            identity.clone(),
                        ));
                    }
                    canonical_additions.push((identity.clone(), canonical_bytes.clone()));
                }
            }
        }
        let exact_order = self
            .catalog
            .canonical_order()
            .extended(canonical_additions)?;
        if transaction.canonical_order != exact_order {
            return Err(BootstrapReadError::InvalidPreparedModel(
                "prepared canonical order is not the exact authority extension",
            ));
        }
        Ok(())
    }

    fn validate_writer_visibility(
        &self,
        transaction: &PreparedBootstrapTransaction<Authority>,
        schemas: SchemaView<'_>,
    ) -> Result<(), BootstrapReadError> {
        let snapshot = transaction.naming_transition().after();
        let mut authored_local = BTreeMap::<String, Vec<VocabularyEncodedId>>::new();
        match &transaction.decoded.document.body {
            BootstrapBody::Interface(body) => {
                for entry in body
                    .inputs
                    .iter()
                    .chain(&body.outputs)
                    .chain(&body.refusals)
                {
                    if let RoleEntry::Declaration(declaration) = entry {
                        seat_local_identity(
                            &mut authored_local,
                            &declaration.name,
                            snapshot,
                            self.catalog.current_module_path(),
                        )?;
                    }
                }
                for declaration in &body.types {
                    seat_local_identity(
                        &mut authored_local,
                        declaration_identity(declaration),
                        snapshot,
                        self.catalog.current_module_path(),
                    )?;
                }
            }
            BootstrapBody::Nexus(body) => {
                for declaration in &body.traits {
                    seat_local_identity(
                        &mut authored_local,
                        &declaration.name,
                        snapshot,
                        self.catalog.current_module_path(),
                    )?;
                }
                for declaration in &body.types {
                    seat_local_identity(
                        &mut authored_local,
                        declaration_identity(declaration),
                        snapshot,
                        self.catalog.current_module_path(),
                    )?;
                }
            }
            BootstrapBody::Sema(body) => {
                for declaration in &body.record_types {
                    seat_local_identity(
                        &mut authored_local,
                        &declaration.name,
                        snapshot,
                        self.catalog.current_module_path(),
                    )?;
                }
            }
        }
        let mut generated_local = authored_local.clone();
        for generated in &transaction.generated_streams {
            seat_local_identity(
                &mut generated_local,
                &generated.initiation.name,
                snapshot,
                self.catalog.current_module_path(),
            )?;
            seat_local_identity(
                &mut generated_local,
                &generated.termination.name,
                snapshot,
                self.catalog.current_module_path(),
            )?;
        }

        let mut imported = BTreeMap::<String, Vec<VocabularyEncodedId>>::new();
        for import in &transaction.decoded.source.imports {
            for name in &import.imported_names {
                let identity = snapshot
                    .identity_at(&import.module_path, None, name)
                    .ok_or_else(|| BootstrapReadError::MissingTextualLookup {
                        module_path: import.module_path.clone(),
                        name: name.clone(),
                    })?;
                imported
                    .entry(name.clone())
                    .or_default()
                    .push(identity.clone());
            }
        }

        let mut authored_references = Vec::new();
        collect_document_references(&transaction.decoded.document.body, &mut authored_references);
        self.validate_visible_references(
            authored_references,
            &authored_local,
            &imported,
            snapshot,
            schemas,
            &transaction.canonical_order,
        )?;

        let mut generated_references = Vec::new();
        for generated in &transaction.generated_streams {
            collect_expression_references(
                &TypeExpression::ShapeApplication(generated.output.stream_of_event.clone()),
                &mut generated_references,
            );
            generated_references.push((
                generated.termination.stream_handle.clone(),
                ReferenceNamespace::Nominal,
            ));
            for relation in &generated.role_relations {
                generated_references.push((relation.target.clone(), ReferenceNamespace::Nominal));
            }
        }
        self.validate_visible_references(
            generated_references,
            &generated_local,
            &imported,
            snapshot,
            schemas,
            &transaction.canonical_order,
        )
    }

    fn validate_visible_references(
        &self,
        references: Vec<(VocabularyEncodedId, ReferenceNamespace)>,
        local: &BTreeMap<String, Vec<VocabularyEncodedId>>,
        imported: &BTreeMap<String, Vec<VocabularyEncodedId>>,
        snapshot: &TextualMetadataSnapshot,
        schemas: SchemaView<'_>,
        canonical_order: &CanonicalIdentityOrder,
    ) -> Result<(), BootstrapReadError> {
        for (identity, namespace) in references {
            let Some(name) = snapshot.spelling(&identity) else {
                return Err(BootstrapReadError::MissingMetadataIdentity(identity));
            };
            let resolved = resolve_visible_identity(
                name,
                local
                    .get(name)
                    .into_iter()
                    .flat_map(|identities| identities.iter().cloned()),
                imported,
                namespace,
                ResolutionAuthority {
                    priors: self.catalog.priors(),
                    snapshot,
                    schemas,
                    canonical_order,
                },
            );
            if !matches!(resolved, Ok(ref resolved) if resolved == &identity) {
                return Err(BootstrapReadError::InvisibleOrNonRoundTrippingReference {
                    identity,
                    name: name.to_owned(),
                });
            }
        }
        Ok(())
    }

    fn envelope<'a>(
        &'a self,
        structural: &'a StructuralDocumentPlan,
    ) -> Result<
        (
            EthosHeader,
            Vec<ImportEntry>,
            &'a SyntaxNode,
            &'a RootSchema,
        ),
        BootstrapReadError,
    > {
        let [header, imports, body] = structural.roots.as_slice() else {
            return Err(BootstrapReadError::UnexpectedStructure {
                expected: "exactly Header, Imports, and Body",
                found: "different top-level arity",
                start: 0,
            });
        };
        let (header, root) = parse_header(header, &self.roots, &self.catalog)?;
        let imports = parse_imports(imports, self.catalog.metadata())?;
        expect_delimited(body, Delimiter::Brace, "kind-selected body")?;
        Ok((header, imports, body, root))
    }

    fn validate_assignment_inputs(
        &self,
        plan: &BootstrapReadPlan,
        assignments: &NamingAssignments,
        generated: &GeneratedStreamAssignments,
        transition: &TextualMetadataTransition,
    ) -> Result<CanonicalIdentityOrder, BootstrapReadError> {
        if transition.before() != self.catalog.metadata() {
            return Err(BootstrapReadError::MetadataTransitionBeforeMismatch);
        }
        let snapshot = transition.after();
        let expected = plan
            .declarations
            .iter()
            .map(PlannedDeclaration::occurrence)
            .collect::<BTreeSet<_>>();
        for declaration in &plan.declarations {
            if assignments.get(declaration.occurrence).is_none() {
                return Err(BootstrapReadError::MissingAssignment(
                    declaration.occurrence.ordinal(),
                ));
            }
        }
        if assignments
            .by_occurrence
            .keys()
            .any(|occurrence| !expected.contains(occurrence))
        {
            return Err(BootstrapReadError::ExtraAssignment);
        }

        let streams = plan
            .declarations
            .iter()
            .filter(|declaration| declaration.purpose == DeclarationPurpose::StreamInitiation)
            .map(PlannedDeclaration::occurrence)
            .collect::<BTreeSet<_>>();
        for stream in &streams {
            if generated.get(*stream).is_none() {
                return Err(BootstrapReadError::MissingGeneratedStreamAssignment(
                    stream.ordinal(),
                ));
            }
        }
        if generated
            .by_source
            .keys()
            .any(|occurrence| !streams.contains(occurrence))
        {
            return Err(BootstrapReadError::ExtraGeneratedStreamAssignment);
        }

        let mut used_ids = BTreeSet::new();
        let mut new_ids = BTreeSet::new();
        let mut canonical_additions = Vec::new();
        for declaration in &plan.declarations {
            let assignment = assignments
                .get(declaration.occurrence)
                .expect("completeness checked above");
            let identity = &assignment.encoded_name;
            let required = schema_role_for_purpose(declaration.purpose);
            self.validate_authority_assignment(
                identity,
                &assignment.disposition,
                required,
                &mut used_ids,
                &mut new_ids,
                &mut canonical_additions,
            )?;
            let record = snapshot
                .record(identity)
                .ok_or_else(|| BootstrapReadError::MissingMetadataIdentity(identity.clone()))?;
            let lexical_owner = match declaration.scope {
                PlannedScope::Module => None,
                PlannedScope::Enum(owner) | PlannedScope::Trait(owner) => Some(
                    assignments
                        .get(owner)
                        .expect("owner occurrence precedes its nested declaration")
                        .encoded_name
                        .clone(),
                ),
            };
            if record.address.module_path != self.catalog.current_module_path()
                || record.address.lexical_owner != lexical_owner
                || record.address.visible_name != declaration.spelling
            {
                return Err(BootstrapReadError::MetadataProjectionMismatch {
                    identity: identity.clone(),
                });
            }
        }
        for assignment in generated.by_source.values() {
            for assigned in [&assignment.initiation, &assignment.termination] {
                let identity = &assigned.encoded_name;
                self.validate_authority_assignment(
                    identity,
                    &assigned.disposition,
                    SchemaRole::Nominal { persistent: false },
                    &mut used_ids,
                    &mut new_ids,
                    &mut canonical_additions,
                )?;
                let record = snapshot
                    .record(identity)
                    .ok_or_else(|| BootstrapReadError::MissingMetadataIdentity(identity.clone()))?;
                if record.address.module_path != self.catalog.current_module_path()
                    || record.address.lexical_owner.is_some()
                {
                    return Err(BootstrapReadError::MetadataProjectionMismatch {
                        identity: identity.clone(),
                    });
                }
            }
        }
        for identity in snapshot.identities() {
            if !self.catalog.schemas().contains(identity) && !new_ids.contains(identity) {
                return Err(BootstrapReadError::ExtraMetadataIdentity(identity.clone()));
            }
        }
        self.catalog.canonical_order().extended(canonical_additions)
    }

    fn validate_authority_assignment(
        &self,
        identity: &VocabularyEncodedId,
        disposition: &IdentityDisposition,
        required: SchemaRole,
        used_ids: &mut BTreeSet<VocabularyEncodedId>,
        new_ids: &mut BTreeSet<VocabularyEncodedId>,
        canonical_additions: &mut Vec<(VocabularyEncodedId, Vec<u8>)>,
    ) -> Result<(), BootstrapReadError> {
        if !used_ids.insert(identity.clone()) {
            return Err(BootstrapReadError::AssignedIdentityCollision {
                identity: identity.clone(),
            });
        }
        match disposition {
            IdentityDisposition::Existing => {
                let Some(schema) = self.catalog.schemas().get(identity) else {
                    return Err(BootstrapReadError::ExistingAssignmentMissing {
                        identity: identity.clone(),
                    });
                };
                if self.catalog.priors().is_fixed_identity(identity) || !schema.admits(required) {
                    return Err(BootstrapReadError::ExistingAssignmentNotReusable {
                        identity: identity.clone(),
                        required,
                    });
                }
                if !self.catalog.canonical_order().contains(identity) {
                    return Err(BootstrapReadError::MissingCanonicalIdentity(
                        identity.clone(),
                    ));
                }
                Ok(())
            }
            IdentityDisposition::New { canonical_bytes } => {
                if self.catalog.schemas().contains(identity)
                    || self.catalog.metadata().record(identity).is_some()
                    || self.catalog.canonical_order().contains(identity)
                {
                    return Err(BootstrapReadError::NewAssignmentAlreadyExists {
                        identity: identity.clone(),
                    });
                }
                new_ids.insert(identity.clone());
                canonical_additions.push((identity.clone(), canonical_bytes.clone()));
                Ok(())
            }
        }
    }

    fn schema_additions(
        &self,
        plan: &BootstrapReadPlan,
        assignments: &NamingAssignments,
        generated: &GeneratedStreamAssignments,
    ) -> Result<IdentitySchemaCatalog, BootstrapReadError> {
        let mut additions = Vec::new();
        for declaration in &plan.declarations {
            let assignment = assignments
                .get(declaration.occurrence)
                .expect("assignment inputs validated before schema preparation");
            if matches!(assignment.disposition, IdentityDisposition::New { .. }) {
                additions.push(IdentitySchema::new(
                    assignment.encoded_name.clone(),
                    [schema_role_for_purpose(declaration.purpose)],
                )?);
            }
        }
        for assignment in generated.by_source.values() {
            for assigned in [&assignment.initiation, &assignment.termination] {
                if matches!(assigned.disposition, IdentityDisposition::New { .. }) {
                    additions.push(IdentitySchema::new(
                        assigned.encoded_name.clone(),
                        [SchemaRole::Nominal { persistent: false }],
                    )?);
                }
            }
        }
        IdentitySchemaCatalog::new(additions)
    }
}

struct PreparedModelValidator<'a> {
    schemas: SchemaView<'a>,
    snapshot: &'a TextualMetadataSnapshot,
    current_module: &'a [String],
    expected_additions: BTreeMap<VocabularyEncodedId, SchemaRole>,
    seen_existing: BTreeSet<VocabularyEncodedId>,
    streams: BTreeMap<VocabularyEncodedId, StreamInitiationDeclaration>,
    canonical_order: &'a CanonicalIdentityOrder,
    priors: &'a BootstrapPriorVocabulary,
    new_identities: BTreeSet<VocabularyEncodedId>,
}

impl PreparedModelValidator<'_> {
    fn validate_body(
        &mut self,
        body: &BootstrapBody,
        root: &RootSchema,
    ) -> Result<(), BootstrapReadError> {
        let mut expected_memberships = Vec::new();
        for (schema, section) in root.sections.iter().zip(root.semantic_sections(body)?) {
            match (schema, section) {
                (SectionSchema::Role(role), RootSemanticSectionRef::Role(entries)) => {
                    for entry in entries {
                        match entry {
                            RoleEntry::Declaration(declaration) => {
                                self.validate_type_declaration(declaration, false)?;
                            }
                            RoleEntry::Reference(identity) => {
                                require_schema(self.schemas, identity, ExpectedSchema::Nominal)?;
                            }
                        }
                        expected_memberships.push(InterfaceRoleMembership {
                            role: *role,
                            target: entry.target().clone(),
                        });
                    }
                    self.require_sorted_by_identity(entries, RoleEntry::target)?;
                }
                (
                    SectionSchema::Declarations { admit_nomos },
                    RootSemanticSectionRef::Declarations(declarations),
                ) => {
                    for declaration in declarations {
                        self.validate_declaration(declaration, *admit_nomos)?;
                    }
                    self.require_sorted_by_identity(declarations, declaration_identity)?;
                }
                (SectionSchema::Traits, RootSemanticSectionRef::Traits(declarations)) => {
                    for declaration in declarations {
                        self.validate_trait(declaration)?;
                    }
                    self.require_sorted_by_identity(declarations, |item| &item.name)?;
                }
                (
                    SectionSchema::PersistentDeclarations,
                    RootSemanticSectionRef::PersistentDeclarations(declarations),
                ) => {
                    for declaration in declarations {
                        self.validate_type_declaration(declaration, true)?;
                    }
                    self.require_sorted_by_identity(declarations, |item| &item.name)?;
                }
                (SectionSchema::Tables, RootSemanticSectionRef::Tables(tables)) => {
                    for table in tables {
                        self.expect_declaration(&table.name, SchemaRole::Table, None)?;
                        require_schema(
                            self.schemas,
                            &table.record_type,
                            ExpectedSchema::PersistentNominal,
                        )?;
                        require_schema(
                            self.schemas,
                            &table.key_type,
                            ExpectedSchema::PersistentNominal,
                        )?;
                    }
                    self.require_sorted_by_identity(tables, |item| &item.name)?;
                }
                _ => {
                    return Err(BootstrapReadError::InvalidPreparedModel(
                        "root registry section does not match semantic projection",
                    ));
                }
            }
        }
        if let BootstrapBody::Interface(body) = body {
            self.sort_memberships(&mut expected_memberships)?;
            if body.memberships != expected_memberships {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "Interface memberships do not exactly equal role entries",
                ));
            }
        } else if !expected_memberships.is_empty() {
            return Err(BootstrapReadError::InvalidPreparedModel(
                "non-Interface root produced role memberships",
            ));
        }
        Ok(())
    }

    fn validate_declaration(
        &mut self,
        declaration: &Declaration,
        admit_nomos: bool,
    ) -> Result<(), BootstrapReadError> {
        match declaration {
            Declaration::Type(declaration) => self.validate_type_declaration(declaration, false),
            Declaration::Nomos(NomosDeclaration::StreamInitiation(declaration)) => {
                if !admit_nomos {
                    return Err(BootstrapReadError::StreamOutsideInterfaceTypes);
                }
                self.expect_declaration(
                    &declaration.name,
                    SchemaRole::Nominal { persistent: false },
                    None,
                )?;
                let mut binders = BinderValidation::new(&declaration.name, self.canonical_order);
                self.validate_expression(&declaration.query, &mut binders)?;
                self.validate_expression(&declaration.event, &mut binders)?;
                if self
                    .streams
                    .insert(declaration.name.clone(), declaration.clone())
                    .is_some()
                {
                    return Err(BootstrapReadError::InvalidPreparedModel(
                        "duplicate authored Stream output identity",
                    ));
                }
                Ok(())
            }
        }
    }

    fn validate_type_declaration(
        &mut self,
        declaration: &TypeDeclaration,
        persistent: bool,
    ) -> Result<(), BootstrapReadError> {
        self.expect_declaration(&declaration.name, SchemaRole::Nominal { persistent }, None)?;
        let mut binders = BinderValidation::new(&declaration.name, self.canonical_order);
        match &declaration.body {
            TypeBody::Newtype(expression) => self.validate_expression(expression, &mut binders)?,
            TypeBody::Struct(fields) => {
                for field in fields {
                    self.validate_expression(field, &mut binders)?;
                }
            }
            TypeBody::Enum(variants) => {
                if variants.is_empty() {
                    return Err(BootstrapReadError::InvalidPreparedModel(
                        "enum has no variants",
                    ));
                }
                for variant in variants {
                    self.expect_declaration(
                        &variant.name,
                        SchemaRole::Variant,
                        Some(&declaration.name),
                    )?;
                    match &variant.body {
                        VariantBody::Unit => {}
                        VariantBody::Unary(expression) => {
                            self.validate_expression(expression, &mut binders)?;
                        }
                        VariantBody::Product(fields) => {
                            if fields.is_empty() {
                                return Err(BootstrapReadError::InvalidPreparedModel(
                                    "product variant has no positions",
                                ));
                            }
                            for field in fields {
                                self.validate_expression(field, &mut binders)?;
                            }
                        }
                    }
                }
                self.require_sorted_by_identity(variants, |item| &item.name)?;
            }
        }
        Ok(())
    }

    fn validate_trait(&mut self, declaration: &TraitDeclaration) -> Result<(), BootstrapReadError> {
        self.expect_declaration(&declaration.name, SchemaRole::Trait, None)?;
        for method in &declaration.methods {
            self.expect_declaration(&method.name, SchemaRole::Method, Some(&declaration.name))?;
            let mut binders = BinderValidation::new(&method.name, self.canonical_order);
            for parameter in &method.parameters {
                self.validate_expression(parameter, &mut binders)?;
            }
            self.validate_expression(&method.return_type, &mut binders)?;
        }
        self.require_sorted_by_identity(&declaration.methods, |item| &item.name)?;
        Ok(())
    }

    fn validate_expression(
        &self,
        expression: &TypeExpression,
        binders: &mut BinderValidation,
    ) -> Result<(), BootstrapReadError> {
        match expression {
            TypeExpression::Reference(identity) => {
                require_schema(self.schemas, identity, ExpectedSchema::Nominal)?;
            }
            TypeExpression::ShapeApplication(application) => {
                let schema =
                    require_schema(self.schemas, &application.shape, ExpectedSchema::Shape)?;
                let arity = schema
                    .shape_arity()
                    .expect("Shape requirement proved exact Shape role");
                if application.arguments.len() != usize::from(arity) {
                    return Err(BootstrapReadError::ShapeArity {
                        identity: application.shape.clone(),
                        expected: arity,
                        found: application.arguments.len(),
                    });
                }
                for argument in &application.arguments {
                    self.validate_expression(argument, binders)?;
                }
            }
            TypeExpression::TraitRequirement(requirement) => {
                if requirement.required_traits.is_empty() {
                    return Err(BootstrapReadError::InvalidPreparedModel(
                        "Trait requirement is empty",
                    ));
                }
                for identity in &requirement.required_traits {
                    require_schema(self.schemas, identity, ExpectedSchema::Trait)?;
                }
                if !self.strictly_sorted(&requirement.required_traits)? {
                    return Err(BootstrapReadError::InvalidPreparedModel(
                        "Trait requirement is not in unique canonical byte order",
                    ));
                }
                binders.observe(requirement)?;
            }
        }
        Ok(())
    }

    fn validate_streams(
        &mut self,
        generated: &[PreparedStreamGeneration],
        priors: &BootstrapPriorVocabulary,
    ) -> Result<(), BootstrapReadError> {
        if generated.len() != self.streams.len() {
            return Err(BootstrapReadError::InvalidPreparedModel(
                "generated Stream transaction count",
            ));
        }
        let mut seen = BTreeSet::new();
        for stream in generated {
            let Some(authored) = self.streams.get(&stream.output.name) else {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "generated Stream has no authored source declaration",
                ));
            };
            if !seen.insert(stream.output.name.clone()) {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "duplicate generated Stream output",
                ));
            }
            if stream.initiation.query != authored.query
                || stream.output.stream_of_event.shape != priors.identities().stream_shape
                || stream.output.stream_of_event.arguments.as_slice() != [authored.event.clone()]
                || stream.termination.stream_handle != authored.name
                || stream.initiation.name == stream.output.name
                || stream.initiation.name == stream.termination.name
                || stream.output.name == stream.termination.name
            {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "generated Stream declaration anatomy",
                ));
            }
            self.expect_declaration(
                &stream.initiation.name,
                SchemaRole::Nominal { persistent: false },
                None,
            )?;
            self.expect_declaration(
                &stream.termination.name,
                SchemaRole::Nominal { persistent: false },
                None,
            )?;
            let expected_relations = [
                InterfaceRoleMembership {
                    role: InterfaceRole::Input,
                    target: stream.initiation.name.clone(),
                },
                InterfaceRoleMembership {
                    role: InterfaceRole::Output,
                    target: stream.output.name.clone(),
                },
                InterfaceRoleMembership {
                    role: InterfaceRole::Input,
                    target: stream.termination.name.clone(),
                },
            ];
            if stream.role_relations != expected_relations {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "generated Stream role relations",
                ));
            }
        }
        self.require_sorted_by_identity(generated, |item| &item.output.name)?;
        Ok(())
    }

    fn expect_declaration(
        &mut self,
        identity: &VocabularyEncodedId,
        role: SchemaRole,
        lexical_owner: Option<&VocabularyEncodedId>,
    ) -> Result<(), BootstrapReadError> {
        let schema = self
            .schemas
            .get(identity)
            .ok_or_else(|| BootstrapReadError::MissingSchema(identity.clone()))?;
        if !schema.admits(role) {
            return Err(BootstrapReadError::WrongSchemaRole {
                identity: identity.clone(),
                required: role,
            });
        }
        let record = self
            .snapshot
            .record(identity)
            .ok_or_else(|| BootstrapReadError::MissingMetadataIdentity(identity.clone()))?;
        if record.address.module_path != self.current_module
            || record.address.lexical_owner.as_ref() != lexical_owner
        {
            return Err(BootstrapReadError::MetadataProjectionMismatch {
                identity: identity.clone(),
            });
        }
        if !self.seen_existing.insert(identity.clone()) {
            return Err(BootstrapReadError::AssignedIdentityCollision {
                identity: identity.clone(),
            });
        }
        if self.new_identities.contains(identity) {
            let addition = self
                .schemas
                .additions
                .get(identity)
                .ok_or_else(|| BootstrapReadError::MissingSchema(identity.clone()))?;
            if addition.roles() != &BTreeSet::from([role]) {
                return Err(BootstrapReadError::WrongSchemaRole {
                    identity: identity.clone(),
                    required: role,
                });
            }
            self.expected_additions.insert(identity.clone(), role);
        } else if self.schemas.existing.get(identity).is_none() {
            return Err(BootstrapReadError::ExistingAssignmentMissing {
                identity: identity.clone(),
            });
        }
        Ok(())
    }

    fn validate_additions(
        &self,
        additions: &IdentitySchemaCatalog,
    ) -> Result<(), BootstrapReadError> {
        let actual = additions
            .entries()
            .map(|schema| schema.identity().clone())
            .collect::<BTreeSet<_>>();
        let expected = self
            .expected_additions
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        if actual != expected {
            return Err(BootstrapReadError::InvalidPreparedModel(
                "schema additions are not exactly the prepared declarations",
            ));
        }
        Ok(())
    }

    fn strictly_sorted(
        &self,
        identities: &[VocabularyEncodedId],
    ) -> Result<bool, BootstrapReadError> {
        for pair in identities.windows(2) {
            if self.canonical_order.compare(&pair[0], &pair[1])? != CompareOrdering::Less {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn require_sorted_by_identity<T>(
        &self,
        values: &[T],
        identity: impl Fn(&T) -> &VocabularyEncodedId,
    ) -> Result<(), BootstrapReadError> {
        for pair in values.windows(2) {
            if self
                .canonical_order
                .compare(identity(&pair[0]), identity(&pair[1]))?
                != CompareOrdering::Less
            {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "unordered named collection is not in canonical identity order",
                ));
            }
        }
        Ok(())
    }

    fn sort_memberships(
        &self,
        memberships: &mut [InterfaceRoleMembership],
    ) -> Result<(), BootstrapReadError> {
        memberships.sort_by(|left, right| {
            let left_role = self.canonical_order.bytes(match left.role {
                InterfaceRole::Input => self.priors.role_identity(InterfaceRole::Input),
                InterfaceRole::Output => self.priors.role_identity(InterfaceRole::Output),
                InterfaceRole::Refusal => self.priors.role_identity(InterfaceRole::Refusal),
            });
            let right_role = self.canonical_order.bytes(match right.role {
                InterfaceRole::Input => self.priors.role_identity(InterfaceRole::Input),
                InterfaceRole::Output => self.priors.role_identity(InterfaceRole::Output),
                InterfaceRole::Refusal => self.priors.role_identity(InterfaceRole::Refusal),
            });
            left_role.cmp(&right_role).then_with(|| {
                self.canonical_order
                    .bytes(&left.target)
                    .cmp(&self.canonical_order.bytes(&right.target))
            })
        });
        Ok(())
    }
}

struct BinderValidation<'a> {
    owner: VocabularyEncodedId,
    by_parameter: BTreeMap<LocalParameter, (Option<String>, Vec<u8>)>,
    inferred: BTreeMap<Vec<u8>, LocalParameter>,
    named: BTreeMap<String, (Vec<u8>, LocalParameter)>,
    canonical_order: &'a CanonicalIdentityOrder,
}

impl<'a> BinderValidation<'a> {
    fn new(owner: &VocabularyEncodedId, canonical_order: &'a CanonicalIdentityOrder) -> Self {
        Self {
            owner: owner.clone(),
            by_parameter: BTreeMap::new(),
            inferred: BTreeMap::new(),
            named: BTreeMap::new(),
            canonical_order,
        }
    }

    fn observe(&mut self, requirement: &TraitRequirement) -> Result<(), BootstrapReadError> {
        let parameter = requirement.binder.parameter();
        if parameter.owner != self.owner {
            return Err(BootstrapReadError::InvalidPreparedModel(
                "local parameter escapes its containing declaration",
            ));
        }
        let key = normalized_trait_key(&requirement.required_traits, self.canonical_order)?;
        let name = match &requirement.binder {
            ParameterBinder::Inferred(_) => None,
            ParameterBinder::Named { local_name, .. } => {
                validate_visible_name(local_name)?;
                Some(local_name.clone())
            }
        };
        if let Some((prior_name, prior_key)) = self.by_parameter.get(parameter) {
            if prior_name != &name || prior_key != &key {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "one local parameter has conflicting binder projections",
                ));
            }
        } else {
            self.by_parameter
                .insert(parameter.clone(), (name.clone(), key.clone()));
        }
        if let Some(name) = name {
            if let Some((prior_key, prior_parameter)) = self.named.get(&name) {
                if prior_key != &key || prior_parameter != parameter {
                    return Err(BootstrapReadError::ConflictingNamedParameter { name });
                }
            } else {
                if self
                    .named
                    .values()
                    .any(|(_, prior_parameter)| prior_parameter == parameter)
                    || self.inferred.values().any(|prior| prior == parameter)
                {
                    return Err(BootstrapReadError::InvalidPreparedModel(
                        "distinct binder projections collapse to one local parameter",
                    ));
                }
                self.named.insert(name, (key, parameter.clone()));
            }
        } else if let Some(prior) = self.inferred.get(&key) {
            if prior != parameter {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "equal inferred Trait vectors do not co-refer",
                ));
            }
        } else {
            if self.named.values().any(|(_, prior)| prior == parameter) {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "named and inferred binders collapse",
                ));
            }
            self.inferred.insert(key, parameter.clone());
        }
        Ok(())
    }
}

fn parse_header<'a>(
    node: &SyntaxNode,
    roots: &'a RootSchemaRegistry,
    catalog: &BootstrapCatalog,
) -> Result<(EthosHeader, &'a RootSchema), BootstrapReadError> {
    let SyntaxNode::Application { head, payload, .. } = node else {
        return Err(unexpected_node(node, "Kind.{Major Minor Patch}"));
    };
    let root = roots.resolve_header(head, catalog.metadata())?;
    let components = expect_delimited(payload, Delimiter::Brace, "three version components")?;
    let [major, minor, patch] = components else {
        return Err(unexpected_node(payload, "exactly three version components"));
    };
    let version = EthosVersion::new(
        canonical_decimal(major)?,
        canonical_decimal(minor)?,
        canonical_decimal(patch)?,
    );
    if !catalog.versions().supports(version) {
        return Err(BootstrapReadError::UnsupportedVersion {
            found: version,
            supported: catalog.versions().supported(),
        });
    }
    Ok((
        EthosHeader {
            kind: root.kind,
            version,
        },
        root,
    ))
}

fn canonical_decimal(node: &SyntaxNode) -> Result<u64, BootstrapReadError> {
    let (text, _) = expect_atom(node, "canonical decimal")?;
    if text.is_empty()
        || !text.bytes().all(|byte| byte.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return Err(BootstrapReadError::InvalidVersionComponent(text.to_owned()));
    }
    text.parse()
        .map_err(|_| BootstrapReadError::InvalidVersionComponent(text.to_owned()))
}

fn parse_imports(
    node: &SyntaxNode,
    metadata: &TextualMetadataSnapshot,
) -> Result<Vec<ImportEntry>, BootstrapReadError> {
    expect_delimited(node, Delimiter::Square, "imports square vector")?
        .iter()
        .map(|entry| {
            let SyntaxNode::Application { head, payload, .. } = entry else {
                return Err(unexpected_node(entry, "module:path.[Imported Names]"));
            };
            let module_path = head.split(':').map(str::to_owned).collect::<Vec<_>>();
            validate_module_path(&module_path)?;
            let imported =
                expect_delimited(payload, Delimiter::Square, "nonempty import selectors")?;
            if imported.is_empty() {
                return Err(BootstrapReadError::EmptyImportSelectors);
            }
            let imported_names = imported
                .iter()
                .map(|node| {
                    let (name, _) = expect_atom(node, "imported visible name")?;
                    validate_visible_name(name)?;
                    match metadata.identity_at(&module_path, None, name) {
                        None => Err(BootstrapReadError::MissingTextualLookup {
                            module_path: module_path.clone(),
                            name: name.to_owned(),
                        }),
                        Some(_) => Ok(name.to_owned()),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ImportEntry {
                module_path,
                imported_names,
            })
        })
        .collect()
}

struct ExistingVisibleEnvironment<'a> {
    imported: BTreeMap<String, Vec<VocabularyEncodedId>>,
    catalog: &'a BootstrapCatalog,
}

impl<'a> ExistingVisibleEnvironment<'a> {
    fn new(
        imports: &[ImportEntry],
        catalog: &'a BootstrapCatalog,
    ) -> Result<Self, BootstrapReadError> {
        let mut imported = BTreeMap::<String, Vec<VocabularyEncodedId>>::new();
        for import in imports {
            for name in &import.imported_names {
                let identity = catalog
                    .metadata()
                    .identity_at(&import.module_path, None, name)
                    .ok_or_else(|| BootstrapReadError::MissingTextualLookup {
                        module_path: import.module_path.clone(),
                        name: name.clone(),
                    })?;
                imported
                    .entry(name.clone())
                    .or_default()
                    .push(identity.clone());
            }
        }
        Ok(Self { imported, catalog })
    }

    fn resolve_identity(
        &self,
        spelling: &str,
        namespace: ReferenceNamespace,
    ) -> Result<VocabularyEncodedId, BootstrapReadError> {
        resolve_visible_identity(
            spelling,
            std::iter::empty(),
            &self.imported,
            namespace,
            ResolutionAuthority {
                priors: self.catalog.priors(),
                snapshot: self.catalog.metadata(),
                schemas: SchemaView {
                    existing: self.catalog.schemas(),
                    additions: &IdentitySchemaCatalog::default(),
                },
                canonical_order: self.catalog.canonical_order(),
            },
        )
    }

    fn schema(
        &self,
        identity: &VocabularyEncodedId,
    ) -> Result<&IdentitySchema, BootstrapReadError> {
        self.catalog
            .schemas()
            .get(identity)
            .ok_or_else(|| BootstrapReadError::MissingSchema(identity.clone()))
    }
}

struct OccurrencePlanner {
    plan: u64,
    declarations: Vec<PlannedDeclaration>,
    names: BTreeMap<(ScopeKey, String), DeclarationOccurrence>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ScopeKey {
    Module,
    Enum(u32),
    Trait(u32),
}

impl OccurrencePlanner {
    fn new(plan: u64) -> Self {
        Self {
            plan,
            declarations: Vec::new(),
            names: BTreeMap::new(),
        }
    }

    fn add(
        &mut self,
        spelling: String,
        bound: SourceBound,
        scope: ScopeKey,
        purpose: DeclarationPurpose,
    ) -> Result<DeclarationOccurrence, BootstrapReadError> {
        validate_visible_name(&spelling)?;
        let occurrence = DeclarationOccurrence {
            plan: self.plan,
            ordinal: self.declarations.len() as u32,
        };
        if self
            .names
            .insert((scope, spelling.clone()), occurrence)
            .is_some()
        {
            return Err(BootstrapReadError::DuplicateDeclaration {
                name: spelling,
                scope: format!("{scope:?}"),
            });
        }
        self.declarations.push(PlannedDeclaration {
            occurrence,
            spelling,
            bound,
            scope: match scope {
                ScopeKey::Module => PlannedScope::Module,
                ScopeKey::Enum(owner) => PlannedScope::Enum(DeclarationOccurrence {
                    plan: self.plan,
                    ordinal: owner,
                }),
                ScopeKey::Trait(owner) => PlannedScope::Trait(DeclarationOccurrence {
                    plan: self.plan,
                    ordinal: owner,
                }),
            },
            purpose,
        });
        Ok(occurrence)
    }

    fn discover_section(
        &mut self,
        schema: SectionSchema,
        node: &SyntaxNode,
        environment: &ExistingVisibleEnvironment<'_>,
        catalog: &BootstrapCatalog,
    ) -> Result<(), BootstrapReadError> {
        let items = expect_delimited(node, Delimiter::Square, "registered section vector")?;
        match schema {
            SectionSchema::Role(_) => {
                for entry in items {
                    match entry {
                        SyntaxNode::Atom { text, .. } => validate_visible_name(text)?,
                        _ if is_nomos_declaration(entry) => {
                            return Err(BootstrapReadError::StreamOutsideInterfaceTypes);
                        }
                        _ => self.discover_type(
                            entry,
                            DeclarationPurpose::Type,
                            false,
                            environment,
                            catalog,
                        )?,
                    }
                }
            }
            SectionSchema::Declarations { admit_nomos } => {
                for declaration in items {
                    self.discover_type(
                        declaration,
                        DeclarationPurpose::Type,
                        admit_nomos,
                        environment,
                        catalog,
                    )?;
                }
            }
            SectionSchema::PersistentDeclarations => {
                for declaration in items {
                    self.discover_type(
                        declaration,
                        DeclarationPurpose::PersistentType,
                        false,
                        environment,
                        catalog,
                    )?;
                }
            }
            SectionSchema::Traits => {
                for declaration in items {
                    self.discover_trait(declaration, environment)?;
                }
            }
            SectionSchema::Tables => {
                for declaration in items {
                    self.discover_table(declaration)?;
                }
            }
        }
        Ok(())
    }

    fn discover_type(
        &mut self,
        node: &SyntaxNode,
        purpose: DeclarationPurpose,
        admit_nomos: bool,
        environment: &ExistingVisibleEnvironment<'_>,
        catalog: &BootstrapCatalog,
    ) -> Result<(), BootstrapReadError> {
        let SyntaxNode::Application {
            head,
            head_bound,
            payload,
            ..
        } = node
        else {
            return Err(unexpected_node(node, "named declaration"));
        };
        if is_nomos_projection(payload) {
            if !admit_nomos {
                return Err(if purpose == DeclarationPurpose::PersistentType {
                    BootstrapReadError::NonPersistentDeclaration
                } else {
                    BootstrapReadError::StreamOutsideInterfaceTypes
                });
            }
            self.validate_stream_projection(payload, environment, catalog)?;
            self.add(
                head.clone(),
                *head_bound,
                ScopeKey::Module,
                DeclarationPurpose::StreamInitiation,
            )?;
            return Ok(());
        }
        let owner = self.add(head.clone(), *head_bound, ScopeKey::Module, purpose)?;
        match payload.as_ref() {
            SyntaxNode::Delimited {
                delimiter: Delimiter::Brace,
                items,
                ..
            } => {
                for field in items {
                    validate_type_expression_plan(field, environment)?;
                }
            }
            SyntaxNode::Delimited {
                delimiter: Delimiter::Square,
                items,
                ..
            } => {
                if items.is_empty() {
                    return Err(unexpected_node(payload, "nonempty enum variants"));
                }
                for variant in items {
                    self.discover_variant(variant, owner, environment)?;
                }
            }
            expression => validate_type_expression_plan(expression, environment)?,
        }
        Ok(())
    }

    fn validate_stream_projection(
        &self,
        node: &SyntaxNode,
        environment: &ExistingVisibleEnvironment<'_>,
        catalog: &BootstrapCatalog,
    ) -> Result<(), BootstrapReadError> {
        let SyntaxNode::Application {
            head: nomos,
            payload,
            ..
        } = node
        else {
            return Err(unexpected_node(node, "audited Nomos application"));
        };
        let identity = environment.resolve_identity(nomos, ReferenceNamespace::Nomos)?;
        let schema = environment.schema(&identity)?;
        let Some(NomosSchema::StreamInitiation { arity }) = schema.nomos() else {
            return Err(BootstrapReadError::WrongSchemaRole {
                identity,
                required: SchemaRole::Nomos(NomosSchema::StreamInitiation { arity: 2 }),
            });
        };
        if identity != catalog.priors().identities().stream_nomos {
            return Err(BootstrapReadError::WrongSchemaRole {
                identity,
                required: SchemaRole::Nomos(NomosSchema::StreamInitiation { arity: 2 }),
            });
        }
        let arguments = expect_delimited(payload, Delimiter::Parenthesis, "Nomos arguments")?;
        if arguments.len() != usize::from(arity) {
            return Err(BootstrapReadError::NomosArity {
                identity,
                expected: arity,
                found: arguments.len(),
            });
        }
        for argument in arguments {
            validate_type_expression_plan(argument, environment)?;
        }
        Ok(())
    }

    fn discover_variant(
        &mut self,
        node: &SyntaxNode,
        owner: DeclarationOccurrence,
        environment: &ExistingVisibleEnvironment<'_>,
    ) -> Result<(), BootstrapReadError> {
        let (name, bound, payload) = match node {
            SyntaxNode::Atom { text, bound } => (text, *bound, None),
            SyntaxNode::Application {
                head,
                head_bound,
                payload,
                ..
            } => (head, *head_bound, Some(payload.as_ref())),
            _ => return Err(unexpected_node(node, "enum variant declaration")),
        };
        self.add(
            name.clone(),
            bound,
            ScopeKey::Enum(owner.ordinal()),
            DeclarationPurpose::Variant,
        )?;
        if let Some(payload) = payload {
            match payload {
                SyntaxNode::Delimited {
                    delimiter: Delimiter::Brace,
                    items,
                    ..
                } => {
                    if items.is_empty() {
                        return Err(unexpected_node(payload, "nonempty product variant"));
                    }
                    for item in items {
                        validate_type_expression_plan(item, environment)?;
                    }
                }
                expression => validate_type_expression_plan(expression, environment)?,
            }
        }
        Ok(())
    }

    fn discover_trait(
        &mut self,
        node: &SyntaxNode,
        environment: &ExistingVisibleEnvironment<'_>,
    ) -> Result<(), BootstrapReadError> {
        let SyntaxNode::Application {
            head,
            head_bound,
            payload,
            ..
        } = node
        else {
            return Err(unexpected_node(node, "TraitName.{Methods}"));
        };
        let methods = expect_delimited(payload, Delimiter::Brace, "Trait methods")?;
        let owner = self.add(
            head.clone(),
            *head_bound,
            ScopeKey::Module,
            DeclarationPurpose::Trait,
        )?;
        for method in methods {
            let SyntaxNode::Application {
                head,
                head_bound,
                payload,
                ..
            } = method
            else {
                return Err(unexpected_node(method, "method.{Parameters Return}"));
            };
            let signature = expect_delimited(payload, Delimiter::Brace, "method signature")?;
            if signature.is_empty() {
                return Err(unexpected_node(payload, "mandatory method return"));
            }
            for expression in signature {
                validate_type_expression_plan(expression, environment)?;
            }
            self.add(
                head.clone(),
                *head_bound,
                ScopeKey::Trait(owner.ordinal()),
                DeclarationPurpose::Method,
            )?;
        }
        Ok(())
    }

    fn discover_table(&mut self, node: &SyntaxNode) -> Result<(), BootstrapReadError> {
        let SyntaxNode::Application {
            head,
            head_bound,
            payload,
            ..
        } = node
        else {
            return Err(unexpected_node(node, "table.{RecordType KeyType}"));
        };
        let fields = expect_delimited(payload, Delimiter::Brace, "table fields")?;
        let [record, key] = fields else {
            return Err(unexpected_node(payload, "exactly RecordType then KeyType"));
        };
        expect_atom(record, "persistent nominal RecordType")?;
        expect_atom(key, "persistent nominal KeyType")?;
        self.add(
            head.clone(),
            *head_bound,
            ScopeKey::Module,
            DeclarationPurpose::Table,
        )?;
        Ok(())
    }
}

fn validate_type_expression_plan(
    node: &SyntaxNode,
    environment: &ExistingVisibleEnvironment<'_>,
) -> Result<(), BootstrapReadError> {
    match node {
        SyntaxNode::Atom { text, .. } => validate_visible_name(text),
        SyntaxNode::AngleApplication {
            head, arguments, ..
        } => {
            let identity = environment.resolve_identity(head, ReferenceNamespace::Shape)?;
            let schema = environment.schema(&identity)?;
            let Some(arity) = schema.shape_arity() else {
                return Err(BootstrapReadError::WrongSchemaRole {
                    identity,
                    required: SchemaRole::Shape {
                        arity: arguments.len() as u16,
                    },
                });
            };
            if arguments.len() != usize::from(arity) {
                return Err(BootstrapReadError::ShapeArity {
                    identity,
                    expected: arity,
                    found: arguments.len(),
                });
            }
            for argument in arguments {
                validate_type_expression_plan(argument, environment)?;
            }
            Ok(())
        }
        SyntaxNode::Delimited {
            delimiter: Delimiter::Guillemets,
            items,
            ..
        } => {
            if items.is_empty() {
                return Err(unexpected_node(node, "nonempty Trait requirement"));
            }
            let mut names = BTreeSet::new();
            for (index, item) in items.iter().enumerate() {
                let trait_name = match item {
                    SyntaxNode::Atom { text, .. } => text.as_str(),
                    SyntaxNode::Application { head, payload, .. } if index == 0 => {
                        validate_visible_name(head)?;
                        expect_atom(payload, "Trait after named local binder")?.0
                    }
                    _ => {
                        return Err(unexpected_node(
                            item,
                            "Trait reference or first named binder",
                        ));
                    }
                };
                validate_visible_name(trait_name)?;
                if !names.insert(trait_name.to_owned()) {
                    return Err(BootstrapReadError::DuplicateTraitProjection(
                        trait_name.to_owned(),
                    ));
                }
            }
            Ok(())
        }
        _ => Err(unexpected_node(
            node,
            "TypeReference, Shape application, or Trait requirement",
        )),
    }
}

#[derive(Clone, Copy)]
struct SchemaView<'a> {
    existing: &'a IdentitySchemaCatalog,
    additions: &'a IdentitySchemaCatalog,
}

impl<'a> SchemaView<'a> {
    fn get(&self, identity: &VocabularyEncodedId) -> Option<&'a IdentitySchema> {
        self.additions
            .get(identity)
            .or_else(|| self.existing.get(identity))
    }
}

#[derive(Clone, Copy)]
struct ResolutionAuthority<'a> {
    priors: &'a BootstrapPriorVocabulary,
    snapshot: &'a TextualMetadataSnapshot,
    schemas: SchemaView<'a>,
    canonical_order: &'a CanonicalIdentityOrder,
}

struct ResolutionEnvironment<'a> {
    local: BTreeMap<String, Vec<VocabularyEncodedId>>,
    imported: BTreeMap<String, Vec<VocabularyEncodedId>>,
    snapshot: &'a TextualMetadataSnapshot,
    priors: &'a BootstrapPriorVocabulary,
    schemas: SchemaView<'a>,
    canonical_order: &'a CanonicalIdentityOrder,
}

impl<'a> ResolutionEnvironment<'a> {
    fn new(
        plan: &BootstrapReadPlan,
        assignments: &NamingAssignments,
        snapshot: &'a TextualMetadataSnapshot,
        catalog: &'a BootstrapCatalog,
        schemas: SchemaView<'a>,
        canonical_order: &'a CanonicalIdentityOrder,
    ) -> Result<Self, BootstrapReadError> {
        let mut local = BTreeMap::<String, Vec<VocabularyEncodedId>>::new();
        for declaration in &plan.declarations {
            if declaration.scope == PlannedScope::Module {
                local.entry(declaration.spelling.clone()).or_default().push(
                    assignments
                        .get(declaration.occurrence)
                        .expect("assignment completeness validated")
                        .encoded_name
                        .clone(),
                );
            }
        }
        let imported = ExistingVisibleEnvironment::new(&plan.imports, catalog)?.imported;
        Ok(Self {
            local,
            imported,
            snapshot,
            priors: catalog.priors(),
            schemas,
            canonical_order,
        })
    }

    fn resolve_identity(
        &self,
        spelling: &str,
        namespace: ReferenceNamespace,
    ) -> Result<VocabularyEncodedId, BootstrapReadError> {
        resolve_visible_identity(
            spelling,
            self.local
                .get(spelling)
                .into_iter()
                .flat_map(|identities| identities.iter().cloned()),
            &self.imported,
            namespace,
            ResolutionAuthority {
                priors: self.priors,
                snapshot: self.snapshot,
                schemas: self.schemas,
                canonical_order: self.canonical_order,
            },
        )
    }

    fn require(
        &self,
        spelling: &str,
        expected: ExpectedSchema,
    ) -> Result<VocabularyEncodedId, BootstrapReadError> {
        let identity = self.resolve_identity(spelling, expected.namespace())?;
        require_schema(self.schemas, &identity, expected)?;
        Ok(identity)
    }
}

#[derive(Clone, Copy)]
enum ExpectedSchema {
    Nominal,
    PersistentNominal,
    Trait,
    Shape,
    StreamNomos,
}

impl ExpectedSchema {
    const fn namespace(self) -> ReferenceNamespace {
        match self {
            Self::Nominal | Self::PersistentNominal => ReferenceNamespace::Nominal,
            Self::Trait => ReferenceNamespace::Trait,
            Self::Shape => ReferenceNamespace::Shape,
            Self::StreamNomos => ReferenceNamespace::Nomos,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReferenceNamespace {
    Nominal,
    Trait,
    Shape,
    Nomos,
}

fn seat_local_identity(
    local: &mut BTreeMap<String, Vec<VocabularyEncodedId>>,
    identity: &VocabularyEncodedId,
    snapshot: &TextualMetadataSnapshot,
    current_module: &[String],
) -> Result<(), BootstrapReadError> {
    let record = snapshot
        .record(identity)
        .ok_or_else(|| BootstrapReadError::MissingMetadataIdentity(identity.clone()))?;
    if record.address.module_path == current_module && record.address.lexical_owner.is_none() {
        local
            .entry(record.address.visible_name.clone())
            .or_default()
            .push(identity.clone());
    }
    Ok(())
}

fn require_schema<'a>(
    schemas: SchemaView<'a>,
    identity: &VocabularyEncodedId,
    expected: ExpectedSchema,
) -> Result<&'a IdentitySchema, BootstrapReadError> {
    let schema = schemas
        .get(identity)
        .ok_or_else(|| BootstrapReadError::MissingSchema(identity.clone()))?;
    let admitted = match expected {
        ExpectedSchema::Nominal => schema
            .roles()
            .iter()
            .any(|role| matches!(role, SchemaRole::Nominal { .. })),
        ExpectedSchema::PersistentNominal => {
            schema.admits(SchemaRole::Nominal { persistent: true })
        }
        ExpectedSchema::Trait => schema.admits(SchemaRole::Trait),
        ExpectedSchema::Shape => schema.shape_arity().is_some(),
        ExpectedSchema::StreamNomos => {
            schema.admits(SchemaRole::Nomos(NomosSchema::StreamInitiation {
                arity: 2,
            }))
        }
    };
    if admitted {
        Ok(schema)
    } else {
        let required = match expected {
            ExpectedSchema::Nominal => SchemaRole::Nominal { persistent: false },
            ExpectedSchema::PersistentNominal => SchemaRole::Nominal { persistent: true },
            ExpectedSchema::Trait => SchemaRole::Trait,
            ExpectedSchema::Shape => SchemaRole::Shape { arity: 0 },
            ExpectedSchema::StreamNomos => {
                SchemaRole::Nomos(NomosSchema::StreamInitiation { arity: 2 })
            }
        };
        Err(BootstrapReadError::WrongSchemaRole {
            identity: identity.clone(),
            required,
        })
    }
}

fn resolve_visible_identity(
    spelling: &str,
    local: impl IntoIterator<Item = VocabularyEncodedId>,
    imported: &BTreeMap<String, Vec<VocabularyEncodedId>>,
    namespace: ReferenceNamespace,
    authority: ResolutionAuthority<'_>,
) -> Result<VocabularyEncodedId, BootstrapReadError> {
    let mut candidates = local.into_iter().collect::<Vec<_>>();
    candidates.extend(imported.get(spelling).into_iter().flatten().cloned());
    let prior_candidates = match namespace {
        ReferenceNamespace::Nominal => authority
            .priors
            .body_nominal_identities()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
        ReferenceNamespace::Shape => authority
            .priors
            .shape_identities()
            .into_iter()
            .cloned()
            .collect(),
        ReferenceNamespace::Nomos => {
            vec![authority.priors.identities().stream_nomos.clone()]
        }
        ReferenceNamespace::Trait => Vec::new(),
    };
    candidates.extend(
        prior_candidates
            .iter()
            .filter(|identity| authority.snapshot.spelling(identity) == Some(spelling))
            .cloned(),
    );
    candidates.retain(|identity| {
        authority
            .schemas
            .get(identity)
            .is_some_and(|schema| namespace_admits(schema, namespace))
    });
    for identity in &candidates {
        authority
            .canonical_order
            .bytes(identity)
            .ok_or_else(|| BootstrapReadError::MissingCanonicalIdentity(identity.clone()))?;
    }
    candidates.sort_by(|left, right| {
        authority
            .canonical_order
            .bytes(left)
            .expect("presence checked")
            .cmp(
                authority
                    .canonical_order
                    .bytes(right)
                    .expect("presence checked"),
            )
    });
    candidates.dedup();
    let identity = match candidates.as_slice() {
        [] => Err(BootstrapReadError::UnresolvedReference {
            name: spelling.to_owned(),
        }),
        [identity] => Ok(identity.clone()),
        many => Err(BootstrapReadError::AmbiguousReference {
            name: spelling.to_owned(),
            identities: many.to_vec(),
        }),
    }?;
    match namespace {
        ReferenceNamespace::Shape
            if !authority
                .priors
                .shape_identities()
                .into_iter()
                .any(|prior| prior == &identity) =>
        {
            Err(BootstrapReadError::NonPriorShapeIdentity { identity })
        }
        ReferenceNamespace::Nomos if identity != authority.priors.identities().stream_nomos => {
            Err(BootstrapReadError::NonPriorNomosIdentity { identity })
        }
        _ => Ok(identity),
    }
}

fn namespace_admits(schema: &IdentitySchema, namespace: ReferenceNamespace) -> bool {
    match namespace {
        ReferenceNamespace::Nominal => schema
            .roles()
            .iter()
            .any(|role| matches!(role, SchemaRole::Nominal { .. })),
        ReferenceNamespace::Trait => schema.admits(SchemaRole::Trait),
        ReferenceNamespace::Shape => schema.shape_arity().is_some(),
        ReferenceNamespace::Nomos => schema.nomos().is_some(),
    }
}

struct AssignmentCursor<'a> {
    declarations: &'a [PlannedDeclaration],
    assignments: &'a NamingAssignments,
    next: usize,
}

impl<'a> AssignmentCursor<'a> {
    fn new(declarations: &'a [PlannedDeclaration], assignments: &'a NamingAssignments) -> Self {
        Self {
            declarations,
            assignments,
            next: 0,
        }
    }

    fn take(
        &mut self,
        spelling: &str,
        purpose: DeclarationPurpose,
    ) -> Result<(VocabularyEncodedId, DeclarationOccurrence), BootstrapReadError> {
        let Some(planned) = self.declarations.get(self.next) else {
            return Err(BootstrapReadError::MissingAssignment(self.next as u32));
        };
        if planned.spelling != spelling || planned.purpose != purpose {
            return Err(BootstrapReadError::UnexpectedStructure {
                expected: "the declaration order selected during planning",
                found: "different declaration while sealing",
                start: planned.bound.start(),
            });
        }
        self.next += 1;
        Ok((
            self.assignments
                .get(planned.occurrence)
                .expect("assignment completeness validated")
                .encoded_name
                .clone(),
            planned.occurrence,
        ))
    }

    fn finish(self) -> Result<(), BootstrapReadError> {
        if self.next == self.declarations.len() {
            Ok(())
        } else {
            Err(BootstrapReadError::MissingAssignment(self.next as u32))
        }
    }
}

enum ReifiedSection {
    Role {
        role: InterfaceRole,
        entries: Vec<RoleEntry>,
        memberships: Vec<InterfaceRoleMembership>,
    },
    Declarations(Vec<Declaration>),
    Traits(Vec<TraitDeclaration>),
    PersistentDeclarations(Vec<TypeDeclaration>),
    Tables(Vec<TableDeclaration>),
}

fn reify_section(
    schema: SectionSchema,
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    generated: &GeneratedStreamAssignments,
    prepared_streams: &mut Vec<PreparedStreamGeneration>,
    priors: &BootstrapPriorVocabulary,
) -> Result<ReifiedSection, BootstrapReadError> {
    let items = expect_delimited(node, Delimiter::Square, "registered section")?;
    match schema {
        SectionSchema::Role(role) => {
            let mut entries = Vec::new();
            let mut memberships = Vec::new();
            for node in items {
                let entry = match node {
                    SyntaxNode::Atom { text, .. } => {
                        RoleEntry::Reference(environment.require(text, ExpectedSchema::Nominal)?)
                    }
                    _ if is_nomos_declaration(node) => {
                        return Err(BootstrapReadError::StreamOutsideInterfaceTypes);
                    }
                    _ => RoleEntry::Declaration(reify_type_declaration(
                        node,
                        environment,
                        cursor,
                        DeclarationPurpose::Type,
                    )?),
                };
                memberships.push(InterfaceRoleMembership {
                    role,
                    target: entry.target().clone(),
                });
                entries.push(entry);
            }
            sort_by_identity(&mut entries, RoleEntry::target, environment.canonical_order)?;
            sort_memberships(&mut memberships, priors, environment.canonical_order)?;
            Ok(ReifiedSection::Role {
                role,
                entries,
                memberships,
            })
        }
        SectionSchema::Declarations { admit_nomos } => {
            let mut declarations = items
                .iter()
                .map(|node| {
                    reify_declaration(
                        node,
                        environment,
                        cursor,
                        generated,
                        prepared_streams,
                        admit_nomos,
                        priors,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            sort_by_identity(
                &mut declarations,
                declaration_identity,
                environment.canonical_order,
            )?;
            Ok(ReifiedSection::Declarations(declarations))
        }
        SectionSchema::Traits => {
            let mut traits = items
                .iter()
                .map(|node| reify_trait(node, environment, cursor))
                .collect::<Result<Vec<_>, _>>()?;
            sort_by_identity(&mut traits, |item| &item.name, environment.canonical_order)?;
            Ok(ReifiedSection::Traits(traits))
        }
        SectionSchema::PersistentDeclarations => {
            let mut declarations = items
                .iter()
                .map(|node| {
                    reify_type_declaration(
                        node,
                        environment,
                        cursor,
                        DeclarationPurpose::PersistentType,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            sort_by_identity(
                &mut declarations,
                |item| &item.name,
                environment.canonical_order,
            )?;
            Ok(ReifiedSection::PersistentDeclarations(declarations))
        }
        SectionSchema::Tables => {
            let mut tables = items
                .iter()
                .map(|node| reify_table(node, environment, cursor))
                .collect::<Result<Vec<_>, _>>()?;
            sort_by_identity(&mut tables, |item| &item.name, environment.canonical_order)?;
            Ok(ReifiedSection::Tables(tables))
        }
    }
}

fn assemble_body(
    root: &RootSchema,
    sections: Vec<ReifiedSection>,
    priors: &BootstrapPriorVocabulary,
    canonical_order: &CanonicalIdentityOrder,
) -> Result<BootstrapBody, BootstrapReadError> {
    if sections.len() != root.sections.len() {
        return Err(BootstrapReadError::InvalidPreparedModel(
            "root registry section count",
        ));
    }
    match root.kind {
        EthosKind::Interface => {
            let mut inputs = None;
            let mut outputs = None;
            let mut refusals = None;
            let mut declarations = None;
            let mut memberships = Vec::new();
            for (schema, section) in root.sections.iter().zip(sections) {
                match (schema, section) {
                    (
                        SectionSchema::Role(expected),
                        ReifiedSection::Role {
                            role,
                            entries,
                            memberships: mut section_memberships,
                        },
                    ) if expected == &role => {
                        let slot = match role {
                            InterfaceRole::Input => &mut inputs,
                            InterfaceRole::Output => &mut outputs,
                            InterfaceRole::Refusal => &mut refusals,
                        };
                        if slot.replace(entries).is_some() {
                            return Err(BootstrapReadError::InvalidPreparedModel(
                                "duplicate Interface role section in root registry",
                            ));
                        }
                        memberships.append(&mut section_memberships);
                    }
                    (SectionSchema::Declarations { .. }, ReifiedSection::Declarations(values)) => {
                        if declarations.replace(values).is_some() {
                            return Err(BootstrapReadError::InvalidPreparedModel(
                                "duplicate Interface declaration section in root registry",
                            ));
                        }
                    }
                    _ => {
                        return Err(BootstrapReadError::InvalidPreparedModel(
                            "Interface root registry section kind",
                        ));
                    }
                }
            }
            sort_memberships(&mut memberships, priors, canonical_order)?;
            Ok(BootstrapBody::Interface(InterfaceBody {
                inputs: inputs.ok_or(BootstrapReadError::InvalidPreparedModel(
                    "root registry omits Interface Input",
                ))?,
                outputs: outputs.ok_or(BootstrapReadError::InvalidPreparedModel(
                    "root registry omits Interface Output",
                ))?,
                refusals: refusals.ok_or(BootstrapReadError::InvalidPreparedModel(
                    "root registry omits Interface Refusal",
                ))?,
                types: declarations.ok_or(BootstrapReadError::InvalidPreparedModel(
                    "root registry omits Interface Types",
                ))?,
                memberships,
            }))
        }
        EthosKind::Nexus => {
            let mut traits = None;
            let mut declarations = None;
            for (schema, section) in root.sections.iter().zip(sections) {
                match (schema, section) {
                    (SectionSchema::Traits, ReifiedSection::Traits(values)) => {
                        if traits.replace(values).is_some() {
                            return Err(BootstrapReadError::InvalidPreparedModel(
                                "duplicate Nexus Trait section in root registry",
                            ));
                        }
                    }
                    (SectionSchema::Declarations { .. }, ReifiedSection::Declarations(values)) => {
                        if declarations.replace(values).is_some() {
                            return Err(BootstrapReadError::InvalidPreparedModel(
                                "duplicate Nexus declaration section in root registry",
                            ));
                        }
                    }
                    _ => {
                        return Err(BootstrapReadError::InvalidPreparedModel(
                            "Nexus root registry section kind",
                        ));
                    }
                }
            }
            Ok(BootstrapBody::Nexus(NexusBody {
                traits: traits.ok_or(BootstrapReadError::InvalidPreparedModel(
                    "root registry omits Nexus Traits",
                ))?,
                types: declarations.ok_or(BootstrapReadError::InvalidPreparedModel(
                    "root registry omits Nexus Types",
                ))?,
            }))
        }
        EthosKind::Sema => {
            let mut records = None;
            let mut tables = None;
            for (schema, section) in root.sections.iter().zip(sections) {
                match (schema, section) {
                    (
                        SectionSchema::PersistentDeclarations,
                        ReifiedSection::PersistentDeclarations(values),
                    ) => {
                        if records.replace(values).is_some() {
                            return Err(BootstrapReadError::InvalidPreparedModel(
                                "duplicate Sema declaration section in root registry",
                            ));
                        }
                    }
                    (SectionSchema::Tables, ReifiedSection::Tables(values)) => {
                        if tables.replace(values).is_some() {
                            return Err(BootstrapReadError::InvalidPreparedModel(
                                "duplicate Sema table section in root registry",
                            ));
                        }
                    }
                    _ => {
                        return Err(BootstrapReadError::InvalidPreparedModel(
                            "Sema root registry section kind",
                        ));
                    }
                }
            }
            Ok(BootstrapBody::Sema(SemaBody {
                record_types: records.ok_or(BootstrapReadError::InvalidPreparedModel(
                    "root registry omits Sema declarations",
                ))?,
                tables: tables.ok_or(BootstrapReadError::InvalidPreparedModel(
                    "root registry omits Sema tables",
                ))?,
            }))
        }
    }
}

fn reify_declaration(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    generated: &GeneratedStreamAssignments,
    prepared_streams: &mut Vec<PreparedStreamGeneration>,
    admit_nomos: bool,
    priors: &BootstrapPriorVocabulary,
) -> Result<Declaration, BootstrapReadError> {
    if is_nomos_declaration(node) {
        if !admit_nomos {
            return Err(BootstrapReadError::StreamOutsideInterfaceTypes);
        }
        let (declaration, prepared) = reify_stream(node, environment, cursor, generated, priors)?;
        prepared_streams.push(prepared);
        Ok(Declaration::Nomos(NomosDeclaration::StreamInitiation(
            declaration,
        )))
    } else {
        reify_type_declaration(node, environment, cursor, DeclarationPurpose::Type)
            .map(Declaration::Type)
    }
}

fn is_nomos_declaration(node: &SyntaxNode) -> bool {
    matches!(node, SyntaxNode::Application { payload, .. } if is_nomos_projection(payload))
}

fn is_nomos_projection(node: &SyntaxNode) -> bool {
    matches!(
        node,
        SyntaxNode::Application { payload, .. }
            if matches!(payload.as_ref(), SyntaxNode::Delimited { delimiter: Delimiter::Parenthesis, .. })
    )
}

fn reify_type_declaration(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    purpose: DeclarationPurpose,
) -> Result<TypeDeclaration, BootstrapReadError> {
    let SyntaxNode::Application { head, payload, .. } = node else {
        return Err(unexpected_node(node, "named type declaration"));
    };
    if is_nomos_projection(payload) {
        return Err(if purpose == DeclarationPurpose::PersistentType {
            BootstrapReadError::NonPersistentDeclaration
        } else {
            BootstrapReadError::StreamOutsideInterfaceTypes
        });
    }
    let (name, _) = cursor.take(head, purpose)?;
    let mut parameters = ParameterScope::new(name.clone(), environment.canonical_order);
    let body = match payload.as_ref() {
        SyntaxNode::Delimited {
            delimiter: Delimiter::Brace,
            items,
            ..
        } => TypeBody::Struct(
            items
                .iter()
                .map(|field| parse_type_expression(field, environment, &mut parameters))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        SyntaxNode::Delimited {
            delimiter: Delimiter::Square,
            items,
            ..
        } => {
            let mut variants = items
                .iter()
                .map(|variant| reify_variant(variant, environment, cursor, &mut parameters))
                .collect::<Result<Vec<_>, _>>()?;
            sort_by_identity(
                &mut variants,
                |item| &item.name,
                environment.canonical_order,
            )?;
            TypeBody::Enum(variants)
        }
        expression => TypeBody::Newtype(parse_type_expression(
            expression,
            environment,
            &mut parameters,
        )?),
    };
    Ok(TypeDeclaration { name, body })
}

fn reify_variant(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    parameters: &mut ParameterScope,
) -> Result<VariantDeclaration, BootstrapReadError> {
    match node {
        SyntaxNode::Atom { text, .. } => Ok(VariantDeclaration {
            name: cursor.take(text, DeclarationPurpose::Variant)?.0,
            body: VariantBody::Unit,
        }),
        SyntaxNode::Application { head, payload, .. } => {
            let name = cursor.take(head, DeclarationPurpose::Variant)?.0;
            let body = match payload.as_ref() {
                SyntaxNode::Delimited {
                    delimiter: Delimiter::Brace,
                    items,
                    ..
                } => VariantBody::Product(
                    items
                        .iter()
                        .map(|item| parse_type_expression(item, environment, parameters))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                expression => {
                    VariantBody::Unary(parse_type_expression(expression, environment, parameters)?)
                }
            };
            Ok(VariantDeclaration { name, body })
        }
        _ => Err(unexpected_node(node, "enum variant")),
    }
}

fn reify_stream(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    generated: &GeneratedStreamAssignments,
    priors: &BootstrapPriorVocabulary,
) -> Result<(StreamInitiationDeclaration, PreparedStreamGeneration), BootstrapReadError> {
    let SyntaxNode::Application { head, payload, .. } = node else {
        return Err(unexpected_node(node, "Name.Nomos.(Query Event)"));
    };
    let SyntaxNode::Application {
        head: nomos,
        payload,
        ..
    } = payload.as_ref()
    else {
        return Err(unexpected_node(payload, "audited Nomos application"));
    };
    let nomos_identity = environment.require(nomos, ExpectedSchema::StreamNomos)?;
    if nomos_identity != priors.identities().stream_nomos {
        return Err(BootstrapReadError::WrongSchemaRole {
            identity: nomos_identity,
            required: SchemaRole::Nomos(NomosSchema::StreamInitiation { arity: 2 }),
        });
    }
    let arguments = expect_delimited(payload, Delimiter::Parenthesis, "Query then Event")?;
    let [query, event] = arguments else {
        return Err(unexpected_node(payload, "exactly Query then Event"));
    };
    let (output_name, occurrence) = cursor.take(head, DeclarationPurpose::StreamInitiation)?;
    let generated = generated.get(occurrence).ok_or_else(|| {
        BootstrapReadError::MissingGeneratedStreamAssignment(occurrence.ordinal())
    })?;
    let mut parameters = ParameterScope::new(output_name.clone(), environment.canonical_order);
    let query = parse_type_expression(query, environment, &mut parameters)?;
    let event = parse_type_expression(event, environment, &mut parameters)?;
    let declaration = StreamInitiationDeclaration {
        name: output_name.clone(),
        query: query.clone(),
        event: event.clone(),
    };
    let prepared = PreparedStreamGeneration {
        initiation: StreamInitiationInterfaceDeclaration {
            name: generated.initiation.encoded_name.clone(),
            query,
        },
        output: StreamInterfaceDeclaration {
            name: output_name.clone(),
            stream_of_event: ShapeApplication {
                shape: priors.identities().stream_shape.clone(),
                arguments: vec![event],
            },
        },
        termination: StreamTerminationInterfaceDeclaration {
            name: generated.termination.encoded_name.clone(),
            stream_handle: output_name.clone(),
        },
        role_relations: [
            InterfaceRoleMembership {
                role: InterfaceRole::Input,
                target: generated.initiation.encoded_name.clone(),
            },
            InterfaceRoleMembership {
                role: InterfaceRole::Output,
                target: output_name,
            },
            InterfaceRoleMembership {
                role: InterfaceRole::Input,
                target: generated.termination.encoded_name.clone(),
            },
        ],
    };
    Ok((declaration, prepared))
}

fn reify_trait(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
) -> Result<TraitDeclaration, BootstrapReadError> {
    let SyntaxNode::Application { head, payload, .. } = node else {
        return Err(unexpected_node(node, "TraitName.{Methods}"));
    };
    let name = cursor.take(head, DeclarationPurpose::Trait)?.0;
    let mut methods = expect_delimited(payload, Delimiter::Brace, "Trait methods")?
        .iter()
        .map(|method| reify_method(method, environment, cursor))
        .collect::<Result<Vec<_>, _>>()?;
    sort_by_identity(&mut methods, |item| &item.name, environment.canonical_order)?;
    Ok(TraitDeclaration { name, methods })
}

fn reify_method(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
) -> Result<MethodDeclaration, BootstrapReadError> {
    let SyntaxNode::Application { head, payload, .. } = node else {
        return Err(unexpected_node(node, "method.{Parameters Return}"));
    };
    let name = cursor.take(head, DeclarationPurpose::Method)?.0;
    let signature = expect_delimited(payload, Delimiter::Brace, "method signature")?;
    let Some((return_node, parameter_nodes)) = signature.split_last() else {
        return Err(unexpected_node(payload, "mandatory method return"));
    };
    let mut parameter_scope = ParameterScope::new(name.clone(), environment.canonical_order);
    let parameters = parameter_nodes
        .iter()
        .map(|node| parse_type_expression(node, environment, &mut parameter_scope))
        .collect::<Result<Vec<_>, _>>()?;
    let return_type = parse_type_expression(return_node, environment, &mut parameter_scope)?;
    Ok(MethodDeclaration {
        name,
        parameters,
        return_type,
    })
}

fn reify_table(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
) -> Result<TableDeclaration, BootstrapReadError> {
    let SyntaxNode::Application { head, payload, .. } = node else {
        return Err(unexpected_node(node, "table.{RecordType KeyType}"));
    };
    let fields = expect_delimited(payload, Delimiter::Brace, "table fields")?;
    let [record, key] = fields else {
        return Err(unexpected_node(payload, "exactly RecordType then KeyType"));
    };
    let (record, _) = expect_atom(record, "persistent nominal RecordType")?;
    let (key, _) = expect_atom(key, "persistent nominal KeyType")?;
    Ok(TableDeclaration {
        name: cursor.take(head, DeclarationPurpose::Table)?.0,
        record_type: environment.require(record, ExpectedSchema::PersistentNominal)?,
        key_type: environment.require(key, ExpectedSchema::PersistentNominal)?,
    })
}

struct ParameterScope {
    owner: VocabularyEncodedId,
    next: u32,
    inferred: BTreeMap<Vec<u8>, LocalParameter>,
    named: BTreeMap<String, (Vec<u8>, LocalParameter)>,
    canonical_order: CanonicalIdentityOrder,
}

impl ParameterScope {
    fn new(owner: VocabularyEncodedId, canonical_order: &CanonicalIdentityOrder) -> Self {
        Self {
            owner,
            next: 0,
            inferred: BTreeMap::new(),
            named: BTreeMap::new(),
            canonical_order: canonical_order.clone(),
        }
    }

    fn binder(
        &mut self,
        name: Option<&str>,
        traits: &[VocabularyEncodedId],
    ) -> Result<ParameterBinder, BootstrapReadError> {
        let key = normalized_trait_key(traits, &self.canonical_order)?;
        if let Some(name) = name {
            validate_visible_name(name)?;
            if let Some((prior, parameter)) = self.named.get(name) {
                if prior != &key {
                    return Err(BootstrapReadError::ConflictingNamedParameter {
                        name: name.to_owned(),
                    });
                }
                return Ok(ParameterBinder::Named {
                    parameter: parameter.clone(),
                    local_name: name.to_owned(),
                });
            }
            let parameter = self.fresh();
            self.named.insert(name.to_owned(), (key, parameter.clone()));
            Ok(ParameterBinder::Named {
                parameter,
                local_name: name.to_owned(),
            })
        } else if let Some(parameter) = self.inferred.get(&key) {
            Ok(ParameterBinder::Inferred(parameter.clone()))
        } else {
            let parameter = self.fresh();
            self.inferred.insert(key, parameter.clone());
            Ok(ParameterBinder::Inferred(parameter))
        }
    }

    fn fresh(&mut self) -> LocalParameter {
        let parameter = LocalParameter {
            owner: self.owner.clone(),
            ordinal: self.next,
        };
        self.next += 1;
        parameter
    }
}

fn normalized_trait_key(
    traits: &[VocabularyEncodedId],
    canonical_order: &CanonicalIdentityOrder,
) -> Result<Vec<u8>, BootstrapReadError> {
    let mut key = Vec::new();
    for identity in traits {
        let bytes = canonical_order
            .bytes(identity)
            .ok_or_else(|| BootstrapReadError::MissingCanonicalIdentity(identity.clone()))?;
        key.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        key.extend_from_slice(bytes);
    }
    Ok(key)
}

fn parse_type_expression(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    parameters: &mut ParameterScope,
) -> Result<TypeExpression, BootstrapReadError> {
    match node {
        SyntaxNode::Atom { text, .. } => Ok(TypeExpression::Reference(
            environment.require(text, ExpectedSchema::Nominal)?,
        )),
        SyntaxNode::AngleApplication {
            head, arguments, ..
        } => {
            let identity = environment.require(head, ExpectedSchema::Shape)?;
            let schema = require_schema(environment.schemas, &identity, ExpectedSchema::Shape)?;
            let arity = schema
                .shape_arity()
                .expect("Shape requirement proved exact Shape role");
            if arguments.len() != usize::from(arity) {
                return Err(BootstrapReadError::ShapeArity {
                    identity,
                    expected: arity,
                    found: arguments.len(),
                });
            }
            Ok(TypeExpression::ShapeApplication(ShapeApplication {
                shape: identity,
                arguments: arguments
                    .iter()
                    .map(|argument| parse_type_expression(argument, environment, parameters))
                    .collect::<Result<Vec<_>, _>>()?,
            }))
        }
        SyntaxNode::Delimited {
            delimiter: Delimiter::Guillemets,
            items,
            ..
        } => {
            let mut name = None;
            let mut traits = Vec::new();
            for (index, item) in items.iter().enumerate() {
                let trait_name = match item {
                    SyntaxNode::Atom { text, .. } => text.as_str(),
                    SyntaxNode::Application { head, payload, .. } if index == 0 => {
                        name = Some(head.as_str());
                        expect_atom(payload, "Trait after named local binder")?.0
                    }
                    _ => {
                        return Err(unexpected_node(
                            item,
                            "Trait reference or first named binder",
                        ));
                    }
                };
                traits.push(environment.require(trait_name, ExpectedSchema::Trait)?);
            }
            sort_identities(&mut traits, environment.canonical_order)?;
            for pair in traits.windows(2) {
                if pair[0] == pair[1] {
                    return Err(BootstrapReadError::DuplicateTrait(pair[0].clone()));
                }
            }
            let binder = parameters.binder(name, &traits)?;
            Ok(TypeExpression::TraitRequirement(TraitRequirement {
                binder,
                required_traits: traits,
            }))
        }
        _ => Err(unexpected_node(
            node,
            "TypeReference, Shape application, or Trait requirement",
        )),
    }
}

fn expect_atom<'a>(
    node: &'a SyntaxNode,
    expected: &'static str,
) -> Result<(&'a str, SourceBound), BootstrapReadError> {
    match node {
        SyntaxNode::Atom { text, bound } => Ok((text, *bound)),
        _ => Err(unexpected_node(node, expected)),
    }
}

fn expect_delimited<'a>(
    node: &'a SyntaxNode,
    expected_delimiter: Delimiter,
    expected: &'static str,
) -> Result<&'a [SyntaxNode], BootstrapReadError> {
    match node {
        SyntaxNode::Delimited {
            delimiter, items, ..
        } if *delimiter == expected_delimiter => Ok(items),
        _ => Err(unexpected_node(node, expected)),
    }
}

fn unexpected_node(node: &SyntaxNode, expected: &'static str) -> BootstrapReadError {
    BootstrapReadError::UnexpectedStructure {
        expected,
        found: node.kind_name(),
        start: node.bound().start(),
    }
}

fn schema_role_for_purpose(purpose: DeclarationPurpose) -> SchemaRole {
    match purpose {
        DeclarationPurpose::Type | DeclarationPurpose::StreamInitiation => {
            SchemaRole::Nominal { persistent: false }
        }
        DeclarationPurpose::PersistentType => SchemaRole::Nominal { persistent: true },
        DeclarationPurpose::Variant => SchemaRole::Variant,
        DeclarationPurpose::Trait => SchemaRole::Trait,
        DeclarationPurpose::Method => SchemaRole::Method,
        DeclarationPurpose::Table => SchemaRole::Table,
    }
}

fn declaration_identity(declaration: &Declaration) -> &VocabularyEncodedId {
    match declaration {
        Declaration::Type(declaration) => &declaration.name,
        Declaration::Nomos(NomosDeclaration::StreamInitiation(declaration)) => &declaration.name,
    }
}

fn canonicalize_imports(
    imports: &[ImportEntry],
    snapshot: &TextualMetadataSnapshot,
    canonical_order: &CanonicalIdentityOrder,
) -> Result<Vec<ImportEntry>, BootstrapReadError> {
    let mut by_module = BTreeMap::<Vec<String>, BTreeMap<Vec<u8>, String>>::new();
    for import in imports {
        for name in &import.imported_names {
            let identity = snapshot
                .identity_at(&import.module_path, None, name)
                .ok_or_else(|| BootstrapReadError::MissingTextualLookup {
                    module_path: import.module_path.clone(),
                    name: name.clone(),
                })?;
            let bytes = canonical_order
                .bytes(identity)
                .ok_or_else(|| BootstrapReadError::MissingCanonicalIdentity(identity.clone()))?;
            by_module
                .entry(import.module_path.clone())
                .or_default()
                .insert(bytes.to_vec(), name.clone());
        }
    }
    Ok(by_module
        .into_iter()
        .map(|(module_path, names)| ImportEntry {
            module_path,
            imported_names: names.into_values().collect(),
        })
        .collect())
}

fn collect_document_references(
    body: &BootstrapBody,
    references: &mut Vec<(VocabularyEncodedId, ReferenceNamespace)>,
) {
    match body {
        BootstrapBody::Interface(body) => {
            for entry in body
                .inputs
                .iter()
                .chain(&body.outputs)
                .chain(&body.refusals)
            {
                match entry {
                    RoleEntry::Declaration(declaration) => {
                        collect_type_declaration_references(declaration, references)
                    }
                    RoleEntry::Reference(identity) => {
                        references.push((identity.clone(), ReferenceNamespace::Nominal));
                    }
                }
            }
            for declaration in &body.types {
                match declaration {
                    Declaration::Type(declaration) => {
                        collect_type_declaration_references(declaration, references)
                    }
                    Declaration::Nomos(NomosDeclaration::StreamInitiation(declaration)) => {
                        collect_expression_references(&declaration.query, references);
                        collect_expression_references(&declaration.event, references);
                    }
                }
            }
            for membership in &body.memberships {
                references.push((membership.target.clone(), ReferenceNamespace::Nominal));
            }
        }
        BootstrapBody::Nexus(body) => {
            for declaration in &body.traits {
                for method in &declaration.methods {
                    for parameter in &method.parameters {
                        collect_expression_references(parameter, references);
                    }
                    collect_expression_references(&method.return_type, references);
                }
            }
            for declaration in &body.types {
                match declaration {
                    Declaration::Type(declaration) => {
                        collect_type_declaration_references(declaration, references)
                    }
                    Declaration::Nomos(NomosDeclaration::StreamInitiation(_)) => {}
                }
            }
        }
        BootstrapBody::Sema(body) => {
            for declaration in &body.record_types {
                collect_type_declaration_references(declaration, references);
            }
            for table in &body.tables {
                references.push((table.record_type.clone(), ReferenceNamespace::Nominal));
                references.push((table.key_type.clone(), ReferenceNamespace::Nominal));
            }
        }
    }
}

fn collect_type_declaration_references(
    declaration: &TypeDeclaration,
    references: &mut Vec<(VocabularyEncodedId, ReferenceNamespace)>,
) {
    match &declaration.body {
        TypeBody::Newtype(expression) => collect_expression_references(expression, references),
        TypeBody::Struct(fields) => {
            for field in fields {
                collect_expression_references(field, references);
            }
        }
        TypeBody::Enum(variants) => {
            for variant in variants {
                match &variant.body {
                    VariantBody::Unit => {}
                    VariantBody::Unary(expression) => {
                        collect_expression_references(expression, references)
                    }
                    VariantBody::Product(fields) => {
                        for field in fields {
                            collect_expression_references(field, references);
                        }
                    }
                }
            }
        }
    }
}

fn collect_expression_references(
    expression: &TypeExpression,
    references: &mut Vec<(VocabularyEncodedId, ReferenceNamespace)>,
) {
    match expression {
        TypeExpression::Reference(identity) => {
            references.push((identity.clone(), ReferenceNamespace::Nominal));
        }
        TypeExpression::ShapeApplication(application) => {
            references.push((application.shape.clone(), ReferenceNamespace::Shape));
            for argument in &application.arguments {
                collect_expression_references(argument, references);
            }
        }
        TypeExpression::TraitRequirement(requirement) => {
            references.extend(
                requirement
                    .required_traits()
                    .iter()
                    .cloned()
                    .map(|identity| (identity, ReferenceNamespace::Trait)),
            );
        }
    }
}

fn sort_identities(
    identities: &mut [VocabularyEncodedId],
    canonical_order: &CanonicalIdentityOrder,
) -> Result<(), BootstrapReadError> {
    for identity in identities.iter() {
        canonical_order
            .bytes(identity)
            .ok_or_else(|| BootstrapReadError::MissingCanonicalIdentity(identity.clone()))?;
    }
    identities.sort_by(|left, right| {
        canonical_order
            .bytes(left)
            .expect("presence checked")
            .cmp(canonical_order.bytes(right).expect("presence checked"))
    });
    Ok(())
}

fn sort_by_identity<T>(
    values: &mut [T],
    identity: impl Fn(&T) -> &VocabularyEncodedId,
    canonical_order: &CanonicalIdentityOrder,
) -> Result<(), BootstrapReadError> {
    for value in values.iter() {
        let identity = identity(value);
        canonical_order
            .bytes(identity)
            .ok_or_else(|| BootstrapReadError::MissingCanonicalIdentity(identity.clone()))?;
    }
    values.sort_by(|left, right| {
        canonical_order
            .bytes(identity(left))
            .expect("presence checked")
            .cmp(
                canonical_order
                    .bytes(identity(right))
                    .expect("presence checked"),
            )
    });
    Ok(())
}

fn sort_memberships(
    memberships: &mut [InterfaceRoleMembership],
    priors: &BootstrapPriorVocabulary,
    canonical_order: &CanonicalIdentityOrder,
) -> Result<(), BootstrapReadError> {
    for membership in memberships.iter() {
        for identity in [priors.role_identity(membership.role), &membership.target] {
            canonical_order
                .bytes(identity)
                .ok_or_else(|| BootstrapReadError::MissingCanonicalIdentity(identity.clone()))?;
        }
    }
    memberships.sort_by(|left, right| {
        canonical_order
            .bytes(priors.role_identity(left.role))
            .expect("presence checked")
            .cmp(
                canonical_order
                    .bytes(priors.role_identity(right.role))
                    .expect("presence checked"),
            )
            .then_with(|| {
                canonical_order
                    .bytes(&left.target)
                    .expect("presence checked")
                    .cmp(
                        canonical_order
                            .bytes(&right.target)
                            .expect("presence checked"),
                    )
            })
    });
    Ok(())
}
