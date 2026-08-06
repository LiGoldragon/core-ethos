//! Allocation-free occurrence planning followed by exact-assignment sealing.

use std::cmp::Ordering as CompareOrdering;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

use raw_discovery::SourceBound;
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};

use super::catalog::*;
use super::error::{BootstrapBuildError, BootstrapReadError};
use super::grammar::{
    BootstrapGrammar, BootstrapGrammarIdentities, Delimiter, StructuralDocumentPlan, SyntaxNode,
};
use super::model::*;
use super::root::{RootSchema, RootSchemaRegistry, SectionSchema};

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
}

/// Checked syntactic assignment set; plan-relative exactness is enforced at seal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamingAssignments {
    by_occurrence: BTreeMap<DeclarationOccurrence, VocabularyEncodedId>,
}

impl NamingAssignments {
    pub fn new(assignments: Vec<NamingAssignment>) -> Result<Self, BootstrapReadError> {
        let mut by_occurrence = BTreeMap::new();
        for assignment in assignments {
            if by_occurrence
                .insert(assignment.occurrence, assignment.encoded_name)
                .is_some()
            {
                return Err(BootstrapReadError::DuplicateAssignment(
                    assignment.occurrence.ordinal(),
                ));
            }
        }
        Ok(Self { by_occurrence })
    }

    fn get(&self, occurrence: DeclarationOccurrence) -> Option<&VocabularyEncodedId> {
        self.by_occurrence.get(&occurrence)
    }
}

/// The two additional identities required by one authored Stream occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedStreamAssignment {
    pub source: DeclarationOccurrence,
    pub initiation: VocabularyEncodedId,
    pub termination: VocabularyEncodedId,
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

/// Complete, validated meaning and schema/name updates prepared for an external
/// authority to commit atomically. The reader has not committed storage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedBootstrapTransaction {
    pub decoded: DecodedBootstrap,
    pub generated_streams: Vec<PreparedStreamGeneration>,
    pub schema_additions: IdentitySchemaCatalog,
    pub naming_snapshot: TextualMetadataSnapshot,
}

impl PreparedBootstrapTransaction {
    pub const fn archive_status(&self) -> BootstrapArchiveStatus {
        BootstrapArchiveStatus::NotYetArchived
    }
}

/// Shared two-phase reader. Allocation, persistence, and commit authority stay
/// outside this type.
#[derive(Clone, Debug)]
pub struct BootstrapReader {
    grammar: BootstrapGrammar,
    catalog: BootstrapCatalog,
    roots: RootSchemaRegistry,
}

impl BootstrapReader {
    pub fn build(
        grammar_identities: BootstrapGrammarIdentities,
        catalog: BootstrapCatalog,
    ) -> Result<Self, BootstrapBuildError> {
        let roots = RootSchemaRegistry::new(catalog.priors());
        Ok(Self {
            grammar: BootstrapGrammar::build(grammar_identities)?,
            catalog,
            roots,
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

    /// Seal using exact authored assignments, exact Stream-generated assignments,
    /// and the complete post-operation textual snapshot. The result remains a
    /// prepared transaction; no authority state is mutated.
    pub fn seal(
        &self,
        plan: &BootstrapReadPlan,
        assignments: &NamingAssignments,
        generated: &GeneratedStreamAssignments,
        naming_snapshot: &TextualMetadataSnapshot,
    ) -> Result<PreparedBootstrapTransaction, BootstrapReadError> {
        self.validate_assignment_inputs(plan, assignments, generated, naming_snapshot)?;
        let schema_additions = self.schema_additions(plan, assignments, generated)?;
        let schemas = SchemaView {
            existing: self.catalog.schemas(),
            additions: &schema_additions,
        };
        let environment =
            ResolutionEnvironment::new(plan, assignments, naming_snapshot, &self.catalog, schemas)?;
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
        let decoded = DecodedBootstrap {
            document: BootstrapDocument {
                header: plan.header,
                body: assemble_body(root, sections)?,
            },
            source: BootstrapSourceProjection {
                imports: plan.imports.clone(),
            },
        };
        let transaction = PreparedBootstrapTransaction {
            decoded,
            generated_streams: prepared_streams,
            schema_additions,
            naming_snapshot: naming_snapshot.clone(),
        };
        self.validate_prepared(&transaction)?;
        Ok(transaction)
    }

    pub fn archive_status(&self) -> BootstrapArchiveStatus {
        BootstrapArchiveStatus::NotYetArchived
    }

    pub(crate) fn catalog(&self) -> &BootstrapCatalog {
        &self.catalog
    }

    pub(crate) fn roots(&self) -> &RootSchemaRegistry {
        &self.roots
    }

    pub(crate) fn validate_prepared(
        &self,
        transaction: &PreparedBootstrapTransaction,
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
        transaction
            .naming_snapshot
            .extends(self.catalog.metadata())?;
        for import in &decoded.source.imports {
            validate_module_path(&import.module_path)?;
            if import.imported_names.is_empty() {
                return Err(BootstrapReadError::EmptyImportSelectors);
            }
            for name in &import.imported_names {
                validate_visible_name(name)?;
                let ids = transaction
                    .naming_snapshot
                    .identities_at(&import.module_path, name);
                if ids.len() != 1 {
                    return Err(if ids.is_empty() {
                        BootstrapReadError::MissingTextualLookup {
                            module_path: import.module_path.clone(),
                            name: name.clone(),
                        }
                    } else {
                        BootstrapReadError::AmbiguousReference {
                            name: name.clone(),
                            identities: ids.to_vec(),
                        }
                    });
                }
            }
        }
        let schemas = SchemaView {
            existing: self.catalog.schemas(),
            additions: &transaction.schema_additions,
        };
        let mut validator = PreparedModelValidator {
            schemas,
            snapshot: &transaction.naming_snapshot,
            current_module: self.catalog.current_module_path(),
            expected_additions: BTreeMap::new(),
            streams: BTreeMap::new(),
        };
        validator.validate_body(&decoded.document.body)?;
        validator.validate_streams(&transaction.generated_streams, self.catalog.priors())?;
        validator.validate_additions(&transaction.schema_additions, self.catalog.metadata())
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
        snapshot: &TextualMetadataSnapshot,
    ) -> Result<(), BootstrapReadError> {
        snapshot.extends(self.catalog.metadata())?;
        let expected = plan
            .declarations
            .iter()
            .map(PlannedDeclaration::occurrence)
            .collect::<BTreeSet<_>>();
        for declaration in &plan.declarations {
            let Some(identity) = assignments.get(declaration.occurrence) else {
                return Err(BootstrapReadError::MissingAssignment(
                    declaration.occurrence.ordinal(),
                ));
            };
            if identity.root_variant() != &VocabularyRoot::Universal {
                return Err(BootstrapReadError::NonUniversalAssignment {
                    occurrence: declaration.occurrence.ordinal(),
                    identity: identity.clone(),
                });
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

        let mut new_ids = BTreeSet::new();
        for declaration in &plan.declarations {
            let identity = assignments
                .get(declaration.occurrence)
                .expect("completeness checked above");
            self.validate_new_identity(identity, declaration.occurrence.ordinal(), &mut new_ids)?;
            let record = snapshot
                .record(identity)
                .ok_or_else(|| BootstrapReadError::MissingMetadataIdentity(identity.clone()))?;
            if record.module_path != self.catalog.current_module_path()
                || record.visible_name != declaration.spelling
            {
                return Err(BootstrapReadError::MetadataProjectionMismatch {
                    identity: identity.clone(),
                });
            }
        }
        for assignment in generated.by_source.values() {
            for identity in [&assignment.initiation, &assignment.termination] {
                if identity.root_variant() != &VocabularyRoot::Universal {
                    return Err(BootstrapReadError::NonUniversalAssignment {
                        occurrence: assignment.source.ordinal(),
                        identity: identity.clone(),
                    });
                }
                self.validate_new_identity(identity, assignment.source.ordinal(), &mut new_ids)?;
                let record = snapshot
                    .record(identity)
                    .ok_or_else(|| BootstrapReadError::MissingMetadataIdentity(identity.clone()))?;
                if record.module_path != self.catalog.current_module_path() {
                    return Err(BootstrapReadError::MetadataProjectionMismatch {
                        identity: identity.clone(),
                    });
                }
            }
        }
        for identity in snapshot.identities() {
            if self.catalog.metadata().record(identity).is_none() && !new_ids.contains(identity) {
                return Err(BootstrapReadError::ExtraMetadataIdentity(identity.clone()));
            }
        }
        Ok(())
    }

    fn validate_new_identity(
        &self,
        identity: &VocabularyEncodedId,
        _occurrence: u32,
        new_ids: &mut BTreeSet<VocabularyEncodedId>,
    ) -> Result<(), BootstrapReadError> {
        if self.catalog.schemas().contains(identity)
            || self.catalog.metadata().record(identity).is_some()
            || !new_ids.insert(identity.clone())
        {
            return Err(BootstrapReadError::AssignedIdentityCollision {
                identity: identity.clone(),
            });
        }
        Ok(())
    }

    fn schema_additions(
        &self,
        plan: &BootstrapReadPlan,
        assignments: &NamingAssignments,
        generated: &GeneratedStreamAssignments,
    ) -> Result<IdentitySchemaCatalog, BootstrapReadError> {
        let mut additions = Vec::new();
        for declaration in &plan.declarations {
            let identity = assignments
                .get(declaration.occurrence)
                .expect("assignment inputs validated before schema preparation")
                .clone();
            let role = match declaration.purpose {
                DeclarationPurpose::Type | DeclarationPurpose::StreamInitiation => {
                    SchemaRole::Nominal { persistent: false }
                }
                DeclarationPurpose::PersistentType => SchemaRole::Nominal { persistent: true },
                DeclarationPurpose::Variant => SchemaRole::Variant,
                DeclarationPurpose::Trait => SchemaRole::Trait,
                DeclarationPurpose::Method => SchemaRole::Method,
                DeclarationPurpose::Table => SchemaRole::Table,
            };
            additions.push(IdentitySchema::new(identity, [role])?);
        }
        for assignment in generated.by_source.values() {
            additions.push(IdentitySchema::new(
                assignment.initiation.clone(),
                [SchemaRole::Nominal { persistent: false }],
            )?);
            additions.push(IdentitySchema::new(
                assignment.termination.clone(),
                [SchemaRole::Nominal { persistent: false }],
            )?);
        }
        IdentitySchemaCatalog::new(additions)
    }
}

struct PreparedModelValidator<'a> {
    schemas: SchemaView<'a>,
    snapshot: &'a TextualMetadataSnapshot,
    current_module: &'a [String],
    expected_additions: BTreeMap<VocabularyEncodedId, SchemaRole>,
    streams: BTreeMap<VocabularyEncodedId, StreamInitiationDeclaration>,
}

impl PreparedModelValidator<'_> {
    fn validate_body(&mut self, body: &BootstrapBody) -> Result<(), BootstrapReadError> {
        match body {
            BootstrapBody::Interface(body) => {
                let mut expected_memberships = Vec::new();
                for (role, entries) in [
                    (InterfaceRole::Input, body.inputs.as_slice()),
                    (InterfaceRole::Output, body.outputs.as_slice()),
                    (InterfaceRole::Refusal, body.refusals.as_slice()),
                ] {
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
                            role,
                            target: entry.target().clone(),
                        });
                    }
                }
                if body.memberships != expected_memberships {
                    return Err(BootstrapReadError::InvalidPreparedModel(
                        "Interface memberships do not exactly equal role entries",
                    ));
                }
                for declaration in &body.types {
                    self.validate_declaration(declaration, true)?;
                }
            }
            BootstrapBody::Nexus(body) => {
                for declaration in &body.traits {
                    self.validate_trait(declaration)?;
                }
                for declaration in &body.types {
                    self.validate_declaration(declaration, false)?;
                }
            }
            BootstrapBody::Sema(body) => {
                for declaration in &body.record_types {
                    self.validate_type_declaration(declaration, true)?;
                }
                for table in &body.tables {
                    self.expect_addition(&table.name, SchemaRole::Table)?;
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
            }
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
                self.expect_addition(&declaration.name, SchemaRole::Nominal { persistent: false })?;
                let mut binders = BinderValidation::new(&declaration.name);
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
        self.expect_addition(&declaration.name, SchemaRole::Nominal { persistent })?;
        let mut binders = BinderValidation::new(&declaration.name);
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
                    self.expect_addition(&variant.name, SchemaRole::Variant)?;
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
            }
        }
        Ok(())
    }

    fn validate_trait(&mut self, declaration: &TraitDeclaration) -> Result<(), BootstrapReadError> {
        self.expect_addition(&declaration.name, SchemaRole::Trait)?;
        for method in &declaration.methods {
            self.expect_addition(&method.name, SchemaRole::Method)?;
            let mut binders = BinderValidation::new(&method.name);
            for parameter in &method.parameters {
                self.validate_expression(parameter, &mut binders)?;
            }
            self.validate_expression(&method.return_type, &mut binders)?;
        }
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
                if !requirement.required_traits.windows(2).all(|pair| {
                    canonical_encoded_name_cmp(&pair[0], &pair[1]) == CompareOrdering::Less
                }) {
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
            self.expect_addition(
                &stream.initiation.name,
                SchemaRole::Nominal { persistent: false },
            )?;
            self.expect_addition(
                &stream.termination.name,
                SchemaRole::Nominal { persistent: false },
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
        Ok(())
    }

    fn expect_addition(
        &mut self,
        identity: &VocabularyEncodedId,
        role: SchemaRole,
    ) -> Result<(), BootstrapReadError> {
        let schema = self
            .schemas
            .additions
            .get(identity)
            .ok_or_else(|| BootstrapReadError::MissingSchema(identity.clone()))?;
        if schema.roles() != &BTreeSet::from([role]) {
            return Err(BootstrapReadError::WrongSchemaRole {
                identity: identity.clone(),
                required: role,
            });
        }
        let record = self
            .snapshot
            .record(identity)
            .ok_or_else(|| BootstrapReadError::MissingMetadataIdentity(identity.clone()))?;
        if record.module_path != self.current_module {
            return Err(BootstrapReadError::MetadataProjectionMismatch {
                identity: identity.clone(),
            });
        }
        if self
            .expected_additions
            .insert(identity.clone(), role)
            .is_some()
        {
            return Err(BootstrapReadError::AssignedIdentityCollision {
                identity: identity.clone(),
            });
        }
        Ok(())
    }

    fn validate_additions(
        &self,
        additions: &IdentitySchemaCatalog,
        base_snapshot: &TextualMetadataSnapshot,
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
        for identity in self.snapshot.identities() {
            if base_snapshot.record(identity).is_none() && !expected.contains(identity) {
                return Err(BootstrapReadError::ExtraMetadataIdentity(identity.clone()));
            }
        }
        Ok(())
    }
}

struct BinderValidation {
    owner: VocabularyEncodedId,
    by_parameter: BTreeMap<LocalParameter, (Option<String>, Vec<u8>)>,
    inferred: BTreeMap<Vec<u8>, LocalParameter>,
    named: BTreeMap<String, (Vec<u8>, LocalParameter)>,
}

impl BinderValidation {
    fn new(owner: &VocabularyEncodedId) -> Self {
        Self {
            owner: owner.clone(),
            by_parameter: BTreeMap::new(),
            inferred: BTreeMap::new(),
            named: BTreeMap::new(),
        }
    }

    fn observe(&mut self, requirement: &TraitRequirement) -> Result<(), BootstrapReadError> {
        let parameter = requirement.binder.parameter();
        if parameter.owner != self.owner {
            return Err(BootstrapReadError::InvalidPreparedModel(
                "local parameter escapes its containing declaration",
            ));
        }
        let key = normalized_trait_key(&requirement.required_traits);
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
                    let ids = metadata.identities_at(&module_path, name);
                    match ids {
                        [] => Err(BootstrapReadError::MissingTextualLookup {
                            module_path: module_path.clone(),
                            name: name.to_owned(),
                        }),
                        [_] => Ok(name.to_owned()),
                        many => Err(BootstrapReadError::AmbiguousReference {
                            name: name.to_owned(),
                            identities: many.to_vec(),
                        }),
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
                let ids = catalog.metadata().identities_at(&import.module_path, name);
                let [identity] = ids else {
                    return Err(if ids.is_empty() {
                        BootstrapReadError::MissingTextualLookup {
                            module_path: import.module_path.clone(),
                            name: name.clone(),
                        }
                    } else {
                        BootstrapReadError::AmbiguousReference {
                            name: name.clone(),
                            identities: ids.to_vec(),
                        }
                    });
                };
                imported
                    .entry(name.clone())
                    .or_default()
                    .push(identity.clone());
            }
        }
        Ok(Self { imported, catalog })
    }

    fn resolve_identity(&self, spelling: &str) -> Result<VocabularyEncodedId, BootstrapReadError> {
        resolve_visible_identity(
            spelling,
            std::iter::empty(),
            &self.imported,
            self.catalog.priors(),
            self.catalog.metadata(),
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
        let identity = environment.resolve_identity(nomos)?;
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
            let identity = environment.resolve_identity(head)?;
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

struct ResolutionEnvironment<'a> {
    local: BTreeMap<String, Vec<VocabularyEncodedId>>,
    imported: BTreeMap<String, Vec<VocabularyEncodedId>>,
    snapshot: &'a TextualMetadataSnapshot,
    priors: &'a BootstrapPriorVocabulary,
    schemas: SchemaView<'a>,
}

impl<'a> ResolutionEnvironment<'a> {
    fn new(
        plan: &BootstrapReadPlan,
        assignments: &NamingAssignments,
        snapshot: &'a TextualMetadataSnapshot,
        catalog: &'a BootstrapCatalog,
        schemas: SchemaView<'a>,
    ) -> Result<Self, BootstrapReadError> {
        let mut local = BTreeMap::<String, Vec<VocabularyEncodedId>>::new();
        for declaration in &plan.declarations {
            if declaration.scope == PlannedScope::Module {
                local.entry(declaration.spelling.clone()).or_default().push(
                    assignments
                        .get(declaration.occurrence)
                        .expect("assignment completeness validated")
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
        })
    }

    fn resolve_identity(&self, spelling: &str) -> Result<VocabularyEncodedId, BootstrapReadError> {
        resolve_visible_identity(
            spelling,
            self.local
                .get(spelling)
                .into_iter()
                .flat_map(|identities| identities.iter().cloned()),
            &self.imported,
            self.priors,
            self.snapshot,
        )
    }

    fn require(
        &self,
        spelling: &str,
        expected: ExpectedSchema,
    ) -> Result<VocabularyEncodedId, BootstrapReadError> {
        let identity = self.resolve_identity(spelling)?;
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
    priors: &BootstrapPriorVocabulary,
    snapshot: &TextualMetadataSnapshot,
) -> Result<VocabularyEncodedId, BootstrapReadError> {
    let mut candidates = local.into_iter().collect::<Vec<_>>();
    candidates.extend(imported.get(spelling).into_iter().flatten().cloned());
    candidates.extend(
        priors
            .all_identities()
            .into_iter()
            .filter(|identity| snapshot.spelling(identity) == Some(spelling)),
    );
    candidates.sort_by(canonical_encoded_name_cmp);
    candidates.dedup();
    match candidates.as_slice() {
        [] => Err(BootstrapReadError::UnresolvedReference {
            name: spelling.to_owned(),
        }),
        [identity] => Ok(identity.clone()),
        many => Err(BootstrapReadError::AmbiguousReference {
            name: spelling.to_owned(),
            identities: many.to_vec(),
        }),
    }
}

/// Explicit canonical EncodedName byte ordering: root tag followed by each
/// table-local u16 in network byte order. No carrier-derived `Ord` participates.
fn canonical_encoded_name_cmp(
    left: &VocabularyEncodedId,
    right: &VocabularyEncodedId,
) -> CompareOrdering {
    canonical_encoded_name_bytes(left).cmp(&canonical_encoded_name_bytes(right))
}

fn canonical_encoded_name_bytes(identity: &VocabularyEncodedId) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(1 + identity.chain().len() * 2);
    bytes.push(identity.root_variant().tag());
    for local in identity.chain() {
        bytes.extend_from_slice(&local.value().to_be_bytes());
    }
    bytes
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
            Ok(ReifiedSection::Role {
                role,
                entries,
                memberships,
            })
        }
        SectionSchema::Declarations { admit_nomos } => Ok(ReifiedSection::Declarations(
            items
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
                .collect::<Result<Vec<_>, _>>()?,
        )),
        SectionSchema::Traits => Ok(ReifiedSection::Traits(
            items
                .iter()
                .map(|node| reify_trait(node, environment, cursor))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        SectionSchema::PersistentDeclarations => Ok(ReifiedSection::PersistentDeclarations(
            items
                .iter()
                .map(|node| {
                    reify_type_declaration(
                        node,
                        environment,
                        cursor,
                        DeclarationPurpose::PersistentType,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        )),
        SectionSchema::Tables => Ok(ReifiedSection::Tables(
            items
                .iter()
                .map(|node| reify_table(node, environment, cursor))
                .collect::<Result<Vec<_>, _>>()?,
        )),
    }
}

fn assemble_body(
    root: &RootSchema,
    mut sections: Vec<ReifiedSection>,
) -> Result<BootstrapBody, BootstrapReadError> {
    match root.kind {
        EthosKind::Interface => {
            let [inputs, outputs, refusals, declarations] = sections.as_mut_slice() else {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "Interface root section count",
                ));
            };
            let (
                ReifiedSection::Role {
                    role: InterfaceRole::Input,
                    entries: inputs,
                    memberships: input_memberships,
                },
                ReifiedSection::Role {
                    role: InterfaceRole::Output,
                    entries: outputs,
                    memberships: output_memberships,
                },
                ReifiedSection::Role {
                    role: InterfaceRole::Refusal,
                    entries: refusals,
                    memberships: refusal_memberships,
                },
                ReifiedSection::Declarations(declarations),
            ) = (inputs, outputs, refusals, declarations)
            else {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "Interface root section kinds",
                ));
            };
            let mut memberships = Vec::new();
            memberships.append(input_memberships);
            memberships.append(output_memberships);
            memberships.append(refusal_memberships);
            Ok(BootstrapBody::Interface(InterfaceBody {
                inputs: std::mem::take(inputs),
                outputs: std::mem::take(outputs),
                refusals: std::mem::take(refusals),
                types: std::mem::take(declarations),
                memberships,
            }))
        }
        EthosKind::Nexus => {
            let [traits, declarations] = sections.as_mut_slice() else {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "Nexus root section count",
                ));
            };
            let (ReifiedSection::Traits(traits), ReifiedSection::Declarations(declarations)) =
                (traits, declarations)
            else {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "Nexus root section kinds",
                ));
            };
            Ok(BootstrapBody::Nexus(NexusBody {
                traits: std::mem::take(traits),
                types: std::mem::take(declarations),
            }))
        }
        EthosKind::Sema => {
            let [records, tables] = sections.as_mut_slice() else {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "Sema root section count",
                ));
            };
            let (ReifiedSection::PersistentDeclarations(records), ReifiedSection::Tables(tables)) =
                (records, tables)
            else {
                return Err(BootstrapReadError::InvalidPreparedModel(
                    "Sema root section kinds",
                ));
            };
            Ok(BootstrapBody::Sema(SemaBody {
                record_types: std::mem::take(records),
                tables: std::mem::take(tables),
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
    let mut parameters = ParameterScope::new(name.clone());
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
        } => TypeBody::Enum(
            items
                .iter()
                .map(|variant| reify_variant(variant, environment, cursor, &mut parameters))
                .collect::<Result<Vec<_>, _>>()?,
        ),
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
    let mut parameters = ParameterScope::new(output_name.clone());
    let query = parse_type_expression(query, environment, &mut parameters)?;
    let event = parse_type_expression(event, environment, &mut parameters)?;
    let declaration = StreamInitiationDeclaration {
        name: output_name.clone(),
        query: query.clone(),
        event: event.clone(),
    };
    let prepared = PreparedStreamGeneration {
        initiation: StreamInitiationInterfaceDeclaration {
            name: generated.initiation.clone(),
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
            name: generated.termination.clone(),
            stream_handle: output_name.clone(),
        },
        role_relations: [
            InterfaceRoleMembership {
                role: InterfaceRole::Input,
                target: generated.initiation.clone(),
            },
            InterfaceRoleMembership {
                role: InterfaceRole::Output,
                target: output_name,
            },
            InterfaceRoleMembership {
                role: InterfaceRole::Input,
                target: generated.termination.clone(),
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
    let methods = expect_delimited(payload, Delimiter::Brace, "Trait methods")?
        .iter()
        .map(|method| reify_method(method, environment, cursor))
        .collect::<Result<Vec<_>, _>>()?;
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
    let mut parameter_scope = ParameterScope::new(name.clone());
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
}

impl ParameterScope {
    fn new(owner: VocabularyEncodedId) -> Self {
        Self {
            owner,
            next: 0,
            inferred: BTreeMap::new(),
            named: BTreeMap::new(),
        }
    }

    fn binder(
        &mut self,
        name: Option<&str>,
        traits: &[VocabularyEncodedId],
    ) -> Result<ParameterBinder, BootstrapReadError> {
        let key = normalized_trait_key(traits);
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

fn normalized_trait_key(traits: &[VocabularyEncodedId]) -> Vec<u8> {
    let mut key = Vec::new();
    for identity in traits {
        let bytes = canonical_encoded_name_bytes(identity);
        key.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        key.extend_from_slice(&bytes);
    }
    key
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
            traits.sort_by(canonical_encoded_name_cmp);
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
