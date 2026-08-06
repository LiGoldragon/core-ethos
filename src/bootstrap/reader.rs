//! Allocation-free occurrence planning followed by exact-assignment sealing.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

use raw_discovery::SourceBound;
use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};

use super::error::{BootstrapBuildError, BootstrapReadError, ExpectedNameClass, NameClass};
use super::grammar::{
    BootstrapGrammar, BootstrapGrammarIdentity, Delimiter, StructuralDocumentPlan, SyntaxNode,
};
use super::model::*;

/// Typed identities supplied by the naming and schema authorities.
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

/// The closed bootstrap prior catalog. Roles that happen to share a visible
/// spelling (notably Stream) remain distinct typed positions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapPriorVocabulary {
    identities: BootstrapPriorIdentities,
}

impl BootstrapPriorVocabulary {
    pub fn new(identities: BootstrapPriorIdentities) -> Result<Self, BootstrapReadError> {
        macro_rules! validate {
            ($field:ident) => {
                if identities.$field.root_variant() != &VocabularyRoot::Universal {
                    return Err(BootstrapReadError::NonUniversalPrior {
                        position: stringify!($field),
                    });
                }
            };
        }
        validate!(interface_kind);
        validate!(nexus_kind);
        validate!(sema_kind);
        validate!(input_role);
        validate!(output_role);
        validate!(refusal_role);
        validate!(string_type);
        validate!(integer_type);
        validate!(boolean_type);
        validate!(unit_type);
        validate!(vector_shape);
        validate!(option_shape);
        validate!(map_shape);
        validate!(result_shape);
        validate!(stream_nomos);
        validate!(stream_shape);
        validate!(stream_identity_shape);
        Ok(Self { identities })
    }

    pub fn identities(&self) -> &BootstrapPriorIdentities {
        &self.identities
    }

    fn candidates(&self, spelling: &str) -> Vec<CatalogValue> {
        let ids = &self.identities;
        match spelling {
            "String" => vec![CatalogValue::new(
                ids.string_type.clone(),
                NameClass::PersistentNominal,
            )],
            "Integer" => vec![CatalogValue::new(
                ids.integer_type.clone(),
                NameClass::PersistentNominal,
            )],
            "Boolean" => vec![CatalogValue::new(
                ids.boolean_type.clone(),
                NameClass::PersistentNominal,
            )],
            "Unit" => vec![CatalogValue::new(
                ids.unit_type.clone(),
                NameClass::PersistentNominal,
            )],
            "Vector" => vec![CatalogValue::new(
                ids.vector_shape.clone(),
                NameClass::Shape,
            )],
            "Option" => vec![CatalogValue::new(
                ids.option_shape.clone(),
                NameClass::Shape,
            )],
            "Map" => vec![CatalogValue::new(ids.map_shape.clone(), NameClass::Shape)],
            "Result" => vec![CatalogValue::new(
                ids.result_shape.clone(),
                NameClass::Shape,
            )],
            "Stream" => vec![
                CatalogValue::new(ids.stream_nomos.clone(), NameClass::NomosHead),
                CatalogValue::new(ids.stream_shape.clone(), NameClass::Shape),
            ],
            "StreamIdentity" => vec![CatalogValue::new(
                ids.stream_identity_shape.clone(),
                NameClass::Shape,
            )],
            _ => Vec::new(),
        }
    }
}

/// One externally supplied textual lookup entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextualMetadataEntry {
    pub module_path: Vec<String>,
    pub visible_name: String,
    pub encoded_name: VocabularyEncodedId,
    pub class: NameClass,
}

/// The injected prior and dependency lookup. It allocates and stores nothing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapCatalog {
    priors: BootstrapPriorVocabulary,
    entries: Vec<TextualMetadataEntry>,
}

impl BootstrapCatalog {
    pub fn new(
        priors: BootstrapPriorVocabulary,
        entries: Vec<TextualMetadataEntry>,
    ) -> Result<Self, BootstrapReadError> {
        let mut exact = BTreeSet::new();
        for entry in &entries {
            if entry.encoded_name.root_variant() != &VocabularyRoot::Universal {
                return Err(BootstrapReadError::NonUniversalCatalogIdentity {
                    name: entry.visible_name.clone(),
                });
            }
            if !exact.insert((entry.module_path.clone(), entry.visible_name.clone())) {
                return Err(BootstrapReadError::DuplicateCatalogEntry {
                    module_path: entry.module_path.clone(),
                    name: entry.visible_name.clone(),
                });
            }
        }
        Ok(Self { priors, entries })
    }

    pub fn priors(&self) -> &BootstrapPriorVocabulary {
        &self.priors
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CatalogValue {
    encoded_name: VocabularyEncodedId,
    class: NameClass,
}

impl CatalogValue {
    fn new(encoded_name: VocabularyEncodedId, class: NameClass) -> Self {
        Self {
            encoded_name,
            class,
        }
    }
}

/// Ephemeral handle naming one exact identity request in this plan.
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

/// The scope in which an authored or generated visible name must be unique.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlannedScope {
    Module,
    Enum(DeclarationOccurrence),
    Trait(DeclarationOccurrence),
}

/// Why one naming-authority assignment is requested.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationPurpose {
    Type,
    PersistentType,
    Variant,
    Trait,
    Method,
    Table,
    StreamOutput,
    StreamInitiation,
    StreamTermination,
}

/// One exact declaration or generated identity request discovered before seal.
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

    /// Authored spelling, or the canonical generated projection suggestion.
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

/// Allocation-free result of structural discovery and occurrence planning.
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

/// One exact naming-authority response.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamingAssignment {
    pub occurrence: DeclarationOccurrence,
    pub encoded_name: VocabularyEncodedId,
}

/// Checked assignment set. Completeness is plan-relative and enforced by seal.
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

    fn validate_exact(&self, plan: &BootstrapReadPlan) -> Result<(), BootstrapReadError> {
        let expected = plan
            .declarations
            .iter()
            .map(PlannedDeclaration::occurrence)
            .collect::<BTreeSet<_>>();
        for occurrence in &expected {
            if !self.by_occurrence.contains_key(occurrence) {
                return Err(BootstrapReadError::MissingAssignment(occurrence.ordinal()));
            }
        }
        for occurrence in self.by_occurrence.keys() {
            if !expected.contains(occurrence) {
                return Err(BootstrapReadError::ExtraAssignment(occurrence.ordinal()));
            }
        }
        let mut identities = BTreeMap::<VocabularyEncodedId, DeclarationOccurrence>::new();
        for (occurrence, identity) in &self.by_occurrence {
            if let Some(first) = identities.insert(identity.clone(), *occurrence) {
                return Err(BootstrapReadError::DuplicateAssignedIdentity {
                    first: first.ordinal(),
                    second: occurrence.ordinal(),
                });
            }
        }
        Ok(())
    }

    fn get(&self, occurrence: DeclarationOccurrence) -> &VocabularyEncodedId {
        &self.by_occurrence[&occurrence]
    }
}

/// Shared two-phase reader. Identity allocation and catalog storage stay outside.
#[derive(Clone, Debug)]
pub struct BootstrapReader {
    grammar: BootstrapGrammar,
    catalog: BootstrapCatalog,
}

impl BootstrapReader {
    pub fn build(
        grammar_identity: BootstrapGrammarIdentity,
        catalog: BootstrapCatalog,
    ) -> Result<Self, BootstrapBuildError> {
        Ok(Self {
            grammar: BootstrapGrammar::build(grammar_identity)?,
            catalog,
        })
    }

    /// Discover boundaries, select the root, and enumerate every exact identity
    /// request without accepting or allocating an identity.
    pub fn plan(&self, source: &str) -> Result<BootstrapReadPlan, BootstrapReadError> {
        let structural = self.grammar.plan(source)?;
        let (header, imports, body) = envelope(&structural)?;
        static NEXT_PLAN: AtomicU64 = AtomicU64::new(1);
        let mut planner = OccurrencePlanner::new(NEXT_PLAN.fetch_add(1, Ordering::Relaxed));
        planner.discover_body(header.kind, body)?;
        Ok(BootstrapReadPlan {
            structural,
            header,
            imports,
            declarations: planner.declarations,
        })
    }

    /// Seal a prior plan using exactly the caller-supplied assignments.
    pub fn seal(
        &self,
        plan: &BootstrapReadPlan,
        assignments: &NamingAssignments,
    ) -> Result<DecodedBootstrap, BootstrapReadError> {
        assignments.validate_exact(plan)?;
        let (_, _, body) = envelope(&plan.structural)?;
        let environment = ResolutionEnvironment::new(plan, assignments, &self.catalog)?;
        let mut cursor = AssignmentCursor::new(&plan.declarations, assignments);
        let mut source = BootstrapSourceMetadata {
            imports: plan.imports.clone(),
            named_parameters: BTreeMap::new(),
        };
        let body = match plan.header.kind {
            EthosKind::Interface => BootstrapBody::Interface(reify_interface(
                body,
                &environment,
                &mut cursor,
                &mut source,
                &self.catalog.priors,
            )?),
            EthosKind::Nexus => {
                BootstrapBody::Nexus(reify_nexus(body, &environment, &mut cursor, &mut source)?)
            }
            EthosKind::Sema => {
                BootstrapBody::Sema(reify_sema(body, &environment, &mut cursor, &mut source)?)
            }
        };
        cursor.finish()?;
        Ok(DecodedBootstrap {
            document: BootstrapDocument {
                header: plan.header,
                body,
            },
            source,
        })
    }

    pub(crate) fn priors(&self) -> &BootstrapPriorVocabulary {
        &self.catalog.priors
    }
}

fn envelope(
    structural: &StructuralDocumentPlan,
) -> Result<(EthosHeader, Vec<ImportEntry>, &SyntaxNode), BootstrapReadError> {
    let [header, imports, body] = structural.roots.as_slice() else {
        return Err(BootstrapReadError::UnexpectedStructure {
            expected: "exactly Header, Imports, and Body",
            found: "different top-level arity",
            start: 0,
        });
    };
    let header = parse_header(header)?;
    let imports = parse_imports(imports)?;
    expect_delimited(body, Delimiter::Brace, "kind-selected body")?;
    Ok((header, imports, body))
}

fn parse_header(node: &SyntaxNode) -> Result<EthosHeader, BootstrapReadError> {
    let SyntaxNode::Application { head, payload, .. } = node else {
        return Err(unexpected_node(node, "Kind.{Major Minor Patch}"));
    };
    let kind = match head.as_str() {
        "Interface" => EthosKind::Interface,
        "Nexus" => EthosKind::Nexus,
        "Sema" => EthosKind::Sema,
        _ => return Err(BootstrapReadError::UnknownFileKind(head.clone())),
    };
    let components = expect_delimited(payload, Delimiter::Brace, "three version components")?;
    let [major, minor, patch] = components else {
        return Err(unexpected_node(payload, "exactly three version components"));
    };
    Ok(EthosHeader {
        kind,
        version: EthosVersion {
            major: canonical_decimal(major)?,
            minor: canonical_decimal(minor)?,
            patch: canonical_decimal(patch)?,
        },
    })
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

fn parse_imports(node: &SyntaxNode) -> Result<Vec<ImportEntry>, BootstrapReadError> {
    expect_delimited(node, Delimiter::Square, "imports square vector")?
        .iter()
        .map(|entry| {
            let SyntaxNode::Application { head, payload, .. } = entry else {
                return Err(unexpected_node(entry, "module:path.[Imported Names]"));
            };
            let module_path = head.split(':').map(str::to_owned).collect::<Vec<_>>();
            if module_path.iter().any(|part| !valid_selector(part)) {
                return Err(BootstrapReadError::InvalidModulePath(head.clone()));
            }
            let imported =
                expect_delimited(payload, Delimiter::Square, "nonempty import selectors")?;
            if imported.is_empty() {
                return Err(BootstrapReadError::EmptyImportSelectors);
            }
            let imported_names = imported
                .iter()
                .map(|node| {
                    let (name, _) = expect_atom(node, "imported visible name")?;
                    if !valid_selector(name) {
                        return Err(BootstrapReadError::InvalidModulePath(name.to_owned()));
                    }
                    Ok(name.to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(ImportEntry {
                module_path,
                imported_names,
            })
        })
        .collect()
}

fn valid_selector(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_alphabetic())
        && characters.all(|character| character == '_' || character.is_alphanumeric())
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
        let occurrence = DeclarationOccurrence {
            plan: self.plan,
            ordinal: self.declarations.len() as u32,
        };
        // Initiation and termination spellings are explanatory projections, not
        // authored names. Naming authority may attach different textual metadata.
        if !matches!(
            purpose,
            DeclarationPurpose::StreamInitiation | DeclarationPurpose::StreamTermination
        ) && self
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

    fn discover_body(
        &mut self,
        kind: EthosKind,
        body: &SyntaxNode,
    ) -> Result<(), BootstrapReadError> {
        let fields = expect_delimited(body, Delimiter::Brace, "body product")?;
        match kind {
            EthosKind::Interface => {
                let [inputs, outputs, refusals, types] = fields else {
                    return Err(unexpected_node(
                        body,
                        "Interface {Inputs Outputs Refusals Types}",
                    ));
                };
                for section in [inputs, outputs, refusals] {
                    for entry in
                        expect_delimited(section, Delimiter::Square, "Interface role vector")?
                    {
                        if !matches!(entry, SyntaxNode::Atom { .. }) {
                            self.discover_type(entry, DeclarationPurpose::Type, false)?;
                        }
                    }
                }
                for declaration in expect_delimited(types, Delimiter::Square, "Interface Types")? {
                    self.discover_type(declaration, DeclarationPurpose::Type, true)?;
                }
            }
            EthosKind::Nexus => {
                let [traits, types] = fields else {
                    return Err(unexpected_node(body, "Nexus {Traits Types}"));
                };
                for declaration in expect_delimited(traits, Delimiter::Square, "Nexus Traits")? {
                    self.discover_trait(declaration)?;
                }
                for declaration in expect_delimited(types, Delimiter::Square, "Nexus Types")? {
                    self.discover_type(declaration, DeclarationPurpose::Type, false)?;
                }
            }
            EthosKind::Sema => {
                let [records, tables] = fields else {
                    return Err(unexpected_node(body, "Sema {RecordTypes Tables}"));
                };
                for declaration in expect_delimited(records, Delimiter::Square, "Sema RecordTypes")?
                {
                    self.discover_type(declaration, DeclarationPurpose::PersistentType, false)?;
                }
                for declaration in expect_delimited(tables, Delimiter::Square, "Sema Tables")? {
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
        admit_stream: bool,
    ) -> Result<(), BootstrapReadError> {
        let SyntaxNode::Application {
            head,
            head_bound,
            payload,
            ..
        } = node
        else {
            return Err(unexpected_node(node, "named type declaration"));
        };
        if is_nomos_projection(payload) {
            if !admit_stream {
                return Err(BootstrapReadError::StreamOutsideInterfaceTypes);
            }
            self.add(
                head.clone(),
                *head_bound,
                ScopeKey::Module,
                DeclarationPurpose::StreamOutput,
            )?;
            self.add(
                format!("{head}Initiation"),
                *head_bound,
                ScopeKey::Module,
                DeclarationPurpose::StreamInitiation,
            )?;
            self.add(
                format!("{head}Termination"),
                *head_bound,
                ScopeKey::Module,
                DeclarationPurpose::StreamTermination,
            )?;
            return Ok(());
        }
        let owner = self.add(head.clone(), *head_bound, ScopeKey::Module, purpose)?;
        if let SyntaxNode::Delimited {
            delimiter: Delimiter::Square,
            items,
            ..
        } = payload.as_ref()
        {
            if items.is_empty() {
                return Err(unexpected_node(payload, "nonempty enum variants"));
            }
            for variant in items {
                self.discover_variant(variant, owner)?;
            }
        }
        Ok(())
    }

    fn discover_variant(
        &mut self,
        node: &SyntaxNode,
        owner: DeclarationOccurrence,
    ) -> Result<(), BootstrapReadError> {
        let (name, bound) = match node {
            SyntaxNode::Atom { text, bound } => (text, *bound),
            SyntaxNode::Application {
                head, head_bound, ..
            } => (head, *head_bound),
            _ => return Err(unexpected_node(node, "enum variant declaration")),
        };
        self.add(
            name.clone(),
            bound,
            ScopeKey::Enum(owner.ordinal()),
            DeclarationPurpose::Variant,
        )?;
        Ok(())
    }

    fn discover_trait(&mut self, node: &SyntaxNode) -> Result<(), BootstrapReadError> {
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
            expect_delimited(payload, Delimiter::Brace, "method signature")?;
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
        if fields.len() != 2 {
            return Err(unexpected_node(payload, "exactly RecordType and KeyType"));
        }
        self.add(
            head.clone(),
            *head_bound,
            ScopeKey::Module,
            DeclarationPurpose::Table,
        )?;
        Ok(())
    }
}

fn is_nomos_projection(node: &SyntaxNode) -> bool {
    matches!(
        node,
        SyntaxNode::Application {
            payload,
            ..
        } if matches!(payload.as_ref(), SyntaxNode::Delimited { delimiter: Delimiter::Parenthesis, .. })
    )
}

struct ResolutionEnvironment<'a> {
    local: BTreeMap<String, Vec<CatalogValue>>,
    imported: BTreeMap<String, Vec<CatalogValue>>,
    catalog: &'a BootstrapCatalog,
}

impl<'a> ResolutionEnvironment<'a> {
    fn new(
        plan: &BootstrapReadPlan,
        assignments: &NamingAssignments,
        catalog: &'a BootstrapCatalog,
    ) -> Result<Self, BootstrapReadError> {
        let mut local = BTreeMap::<String, Vec<CatalogValue>>::new();
        for declaration in &plan.declarations {
            if declaration.scope != PlannedScope::Module {
                continue;
            }
            let class = match declaration.purpose {
                DeclarationPurpose::Type | DeclarationPurpose::StreamOutput => NameClass::Nominal,
                DeclarationPurpose::PersistentType => NameClass::PersistentNominal,
                DeclarationPurpose::Trait => NameClass::Trait,
                DeclarationPurpose::Table => NameClass::Table,
                DeclarationPurpose::Variant
                | DeclarationPurpose::Method
                | DeclarationPurpose::StreamInitiation
                | DeclarationPurpose::StreamTermination => continue,
            };
            local
                .entry(declaration.spelling.clone())
                .or_default()
                .push(CatalogValue::new(
                    assignments.get(declaration.occurrence).clone(),
                    class,
                ));
        }
        let mut imported = BTreeMap::<String, Vec<CatalogValue>>::new();
        for import in &plan.imports {
            for name in &import.imported_names {
                let Some(entry) = catalog.entries.iter().find(|entry| {
                    entry.module_path == import.module_path && entry.visible_name == *name
                }) else {
                    return Err(BootstrapReadError::UnresolvedReference { name: name.clone() });
                };
                imported
                    .entry(name.clone())
                    .or_default()
                    .push(CatalogValue::new(entry.encoded_name.clone(), entry.class));
            }
        }
        Ok(Self {
            local,
            imported,
            catalog,
        })
    }

    fn resolve(
        &self,
        spelling: &str,
        expected: ExpectedNameClass,
    ) -> Result<VocabularyEncodedId, BootstrapReadError> {
        let mut candidates = Vec::new();
        candidates.extend(self.local.get(spelling).into_iter().flatten().cloned());
        candidates.extend(self.imported.get(spelling).into_iter().flatten().cloned());
        candidates.extend(self.catalog.priors.candidates(spelling));
        candidates.retain(|candidate| class_matches(candidate.class, expected));
        candidates.sort_by(|left, right| left.encoded_name.cmp(&right.encoded_name));
        candidates.dedup_by(|left, right| left.encoded_name == right.encoded_name);
        match candidates.as_slice() {
            [] => {
                let mut all = Vec::new();
                all.extend(self.local.get(spelling).into_iter().flatten().cloned());
                all.extend(self.imported.get(spelling).into_iter().flatten().cloned());
                all.extend(self.catalog.priors.candidates(spelling));
                if let Some(found) = all.first() {
                    Err(BootstrapReadError::WrongReferenceClass {
                        name: spelling.to_owned(),
                        actual: found.class,
                        expected,
                    })
                } else {
                    Err(BootstrapReadError::UnresolvedReference {
                        name: spelling.to_owned(),
                    })
                }
            }
            [one] => Ok(one.encoded_name.clone()),
            _ => Err(BootstrapReadError::AmbiguousReference {
                name: spelling.to_owned(),
            }),
        }
    }
}

fn class_matches(actual: NameClass, expected: ExpectedNameClass) -> bool {
    match expected {
        ExpectedNameClass::Nominal => {
            matches!(actual, NameClass::Nominal | NameClass::PersistentNominal)
        }
        ExpectedNameClass::PersistentNominal => actual == NameClass::PersistentNominal,
        ExpectedNameClass::Shape => actual == NameClass::Shape,
        ExpectedNameClass::Trait => actual == NameClass::Trait,
        ExpectedNameClass::StreamNomosHead => actual == NameClass::NomosHead,
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
    ) -> Result<VocabularyEncodedId, BootstrapReadError> {
        let Some(planned) = self.declarations.get(self.next) else {
            return Err(BootstrapReadError::MissingAssignment(self.next as u32));
        };
        if planned.spelling != spelling || planned.purpose != purpose {
            return Err(BootstrapReadError::UnexpectedStructure {
                expected: "the declaration ordering selected during planning",
                found: "a different declaration while sealing",
                start: planned.bound.start(),
            });
        }
        self.next += 1;
        Ok(self.assignments.get(planned.occurrence).clone())
    }

    fn finish(self) -> Result<(), BootstrapReadError> {
        if self.next == self.declarations.len() {
            Ok(())
        } else {
            Err(BootstrapReadError::MissingAssignment(self.next as u32))
        }
    }
}

fn reify_interface(
    body: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    source: &mut BootstrapSourceMetadata,
    priors: &BootstrapPriorVocabulary,
) -> Result<InterfaceBody, BootstrapReadError> {
    let fields = expect_delimited(body, Delimiter::Brace, "Interface body")?;
    let [inputs, outputs, refusals, types] = fields else {
        return Err(unexpected_node(
            body,
            "Interface {Inputs Outputs Refusals Types}",
        ));
    };
    let mut memberships = Vec::new();
    let inputs = reify_role_entries(
        inputs,
        InterfaceRole::Input,
        environment,
        cursor,
        source,
        &mut memberships,
    )?;
    let outputs = reify_role_entries(
        outputs,
        InterfaceRole::Output,
        environment,
        cursor,
        source,
        &mut memberships,
    )?;
    let refusals = reify_role_entries(
        refusals,
        InterfaceRole::Refusal,
        environment,
        cursor,
        source,
        &mut memberships,
    )?;
    let mut declarations = Vec::new();
    for node in expect_delimited(types, Delimiter::Square, "Interface Types")? {
        let declaration = reify_declaration(node, environment, cursor, source, true, priors)?;
        if let Declaration::Stream(stream) = &declaration {
            memberships.extend([
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
            ]);
        }
        declarations.push(declaration);
    }
    Ok(InterfaceBody {
        inputs,
        outputs,
        refusals,
        types: declarations,
        memberships,
    })
}

fn reify_role_entries(
    node: &SyntaxNode,
    role: InterfaceRole,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    source: &mut BootstrapSourceMetadata,
    memberships: &mut Vec<InterfaceRoleMembership>,
) -> Result<Vec<RoleEntry>, BootstrapReadError> {
    let mut entries = Vec::new();
    for entry in expect_delimited(node, Delimiter::Square, "Interface role vector")? {
        let entry = match entry {
            SyntaxNode::Atom { text, .. } => {
                RoleEntry::Reference(environment.resolve(text, ExpectedNameClass::Nominal)?)
            }
            _ if is_nomos_declaration(entry) => {
                return Err(BootstrapReadError::StreamOutsideInterfaceTypes);
            }
            _ => RoleEntry::Declaration(reify_type_declaration(
                entry,
                environment,
                cursor,
                source,
                DeclarationPurpose::Type,
            )?),
        };
        memberships.push(InterfaceRoleMembership {
            role,
            target: entry.target().clone(),
        });
        entries.push(entry);
    }
    Ok(entries)
}

fn reify_nexus(
    body: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    source: &mut BootstrapSourceMetadata,
) -> Result<NexusBody, BootstrapReadError> {
    let fields = expect_delimited(body, Delimiter::Brace, "Nexus body")?;
    let [traits, types] = fields else {
        return Err(unexpected_node(body, "Nexus {Traits Types}"));
    };
    let traits = expect_delimited(traits, Delimiter::Square, "Nexus Traits")?
        .iter()
        .map(|node| reify_trait(node, environment, cursor, source))
        .collect::<Result<Vec<_>, _>>()?;
    let types = expect_delimited(types, Delimiter::Square, "Nexus Types")?
        .iter()
        .map(|node| {
            reify_declaration(
                node,
                environment,
                cursor,
                source,
                false,
                environment.catalog.priors(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(NexusBody { traits, types })
}

fn reify_sema(
    body: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    source: &mut BootstrapSourceMetadata,
) -> Result<SemaBody, BootstrapReadError> {
    let fields = expect_delimited(body, Delimiter::Brace, "Sema body")?;
    let [records, tables] = fields else {
        return Err(unexpected_node(body, "Sema {RecordTypes Tables}"));
    };
    let record_types = expect_delimited(records, Delimiter::Square, "Sema RecordTypes")?
        .iter()
        .map(|node| {
            reify_type_declaration(
                node,
                environment,
                cursor,
                source,
                DeclarationPurpose::PersistentType,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let tables = expect_delimited(tables, Delimiter::Square, "Sema Tables")?
        .iter()
        .map(|node| reify_table(node, environment, cursor))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SemaBody {
        record_types,
        tables,
    })
}

fn reify_declaration(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    source: &mut BootstrapSourceMetadata,
    admit_stream: bool,
    priors: &BootstrapPriorVocabulary,
) -> Result<Declaration, BootstrapReadError> {
    if is_nomos_declaration(node) {
        if !admit_stream {
            return Err(BootstrapReadError::StreamOutsideInterfaceTypes);
        }
        reify_stream(node, environment, cursor, source, priors).map(Declaration::Stream)
    } else {
        reify_type_declaration(node, environment, cursor, source, DeclarationPurpose::Type)
            .map(Declaration::Type)
    }
}

fn is_nomos_declaration(node: &SyntaxNode) -> bool {
    matches!(node, SyntaxNode::Application { payload, .. } if is_nomos_projection(payload))
}

fn reify_type_declaration(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    source: &mut BootstrapSourceMetadata,
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
    let name = cursor.take(head, purpose)?;
    let mut parameters = ParameterScope::new(name.clone());
    let body = match payload.as_ref() {
        SyntaxNode::Delimited {
            delimiter: Delimiter::Brace,
            items,
            ..
        } => TypeBody::Struct(
            items
                .iter()
                .map(|field| parse_type_expression(field, environment, &mut parameters, source))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        SyntaxNode::Delimited {
            delimiter: Delimiter::Square,
            items,
            ..
        } => {
            if items.is_empty() {
                return Err(unexpected_node(payload, "nonempty enum variants"));
            }
            TypeBody::Enum(
                items
                    .iter()
                    .map(|variant| {
                        reify_variant(variant, environment, cursor, &mut parameters, source)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )
        }
        expression => TypeBody::Newtype(parse_type_expression(
            expression,
            environment,
            &mut parameters,
            source,
        )?),
    };
    Ok(TypeDeclaration { name, body })
}

fn reify_variant(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    parameters: &mut ParameterScope,
    source: &mut BootstrapSourceMetadata,
) -> Result<VariantDeclaration, BootstrapReadError> {
    match node {
        SyntaxNode::Atom { text, .. } => Ok(VariantDeclaration {
            name: cursor.take(text, DeclarationPurpose::Variant)?,
            body: VariantBody::Unit,
        }),
        SyntaxNode::Application { head, payload, .. } => {
            let name = cursor.take(head, DeclarationPurpose::Variant)?;
            let body = match payload.as_ref() {
                SyntaxNode::Delimited {
                    delimiter: Delimiter::Brace,
                    items,
                    ..
                } => {
                    if items.is_empty() {
                        return Err(unexpected_node(payload, "nonempty product variant"));
                    }
                    VariantBody::Product(
                        items
                            .iter()
                            .map(|item| {
                                parse_type_expression(item, environment, parameters, source)
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    )
                }
                expression => VariantBody::Unary(parse_type_expression(
                    expression,
                    environment,
                    parameters,
                    source,
                )?),
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
    source: &mut BootstrapSourceMetadata,
    priors: &BootstrapPriorVocabulary,
) -> Result<GeneratedStreamDeclarations, BootstrapReadError> {
    let SyntaxNode::Application { head, payload, .. } = node else {
        return Err(unexpected_node(node, "Name.Stream.(Query Event)"));
    };
    let SyntaxNode::Application {
        head: nomos,
        payload,
        ..
    } = payload.as_ref()
    else {
        return Err(unexpected_node(payload, "Stream.(Query Event)"));
    };
    let resolved_head = environment.resolve(nomos, ExpectedNameClass::StreamNomosHead)?;
    if resolved_head != priors.identities.stream_nomos {
        return Err(BootstrapReadError::WrongReferenceClass {
            name: nomos.clone(),
            actual: NameClass::NomosHead,
            expected: ExpectedNameClass::StreamNomosHead,
        });
    }
    let arguments = expect_delimited(payload, Delimiter::Parenthesis, "Query then Event")?;
    let [query, event] = arguments else {
        return Err(unexpected_node(payload, "exactly Query then Event"));
    };
    let output_name = cursor.take(head, DeclarationPurpose::StreamOutput)?;
    let initiation_name = cursor.take(
        &format!("{head}Initiation"),
        DeclarationPurpose::StreamInitiation,
    )?;
    let termination_name = cursor.take(
        &format!("{head}Termination"),
        DeclarationPurpose::StreamTermination,
    )?;
    let mut parameters = ParameterScope::new(output_name.clone());
    let query = parse_type_expression(query, environment, &mut parameters, source)?;
    let event = parse_type_expression(event, environment, &mut parameters, source)?;
    let stream_of_event = ShapeApplication {
        shape: priors.identities.stream_shape.clone(),
        arguments: vec![event],
    };
    Ok(GeneratedStreamDeclarations {
        initiation: StreamInitiationInterfaceDeclaration {
            name: initiation_name,
            query,
        },
        output: StreamInterfaceDeclaration {
            name: output_name.clone(),
            stream_of_event,
        },
        termination: StreamTerminationInterfaceDeclaration {
            name: termination_name,
            stream_handle: output_name,
        },
    })
}

fn reify_trait(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    source: &mut BootstrapSourceMetadata,
) -> Result<TraitDeclaration, BootstrapReadError> {
    let SyntaxNode::Application { head, payload, .. } = node else {
        return Err(unexpected_node(node, "TraitName.{Methods}"));
    };
    let name = cursor.take(head, DeclarationPurpose::Trait)?;
    let methods = expect_delimited(payload, Delimiter::Brace, "Trait methods")?
        .iter()
        .map(|method| reify_method(method, environment, cursor, source))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TraitDeclaration { name, methods })
}

fn reify_method(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    cursor: &mut AssignmentCursor<'_>,
    source: &mut BootstrapSourceMetadata,
) -> Result<MethodDeclaration, BootstrapReadError> {
    let SyntaxNode::Application { head, payload, .. } = node else {
        return Err(unexpected_node(node, "method.{Parameters Return}"));
    };
    let name = cursor.take(head, DeclarationPurpose::Method)?;
    let signature = expect_delimited(payload, Delimiter::Brace, "method signature")?;
    let Some((return_node, parameter_nodes)) = signature.split_last() else {
        return Err(unexpected_node(payload, "mandatory method return"));
    };
    let mut parameters_scope = ParameterScope::new(name.clone());
    let parameters = parameter_nodes
        .iter()
        .map(|node| parse_type_expression(node, environment, &mut parameters_scope, source))
        .collect::<Result<Vec<_>, _>>()?;
    let return_type =
        parse_type_expression(return_node, environment, &mut parameters_scope, source)?;
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
        name: cursor.take(head, DeclarationPurpose::Table)?,
        record_type: environment.resolve(record, ExpectedNameClass::PersistentNominal)?,
        key_type: environment.resolve(key, ExpectedNameClass::PersistentNominal)?,
    })
}

struct ParameterScope {
    owner: VocabularyEncodedId,
    next: u32,
    inferred: BTreeMap<Vec<VocabularyEncodedId>, LocalParameter>,
    named: BTreeMap<String, (Vec<VocabularyEncodedId>, LocalParameter)>,
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

    fn parameter(
        &mut self,
        name: Option<&str>,
        traits: &[VocabularyEncodedId],
        source: &mut BootstrapSourceMetadata,
    ) -> Result<LocalParameter, BootstrapReadError> {
        if let Some(name) = name {
            if let Some((prior, parameter)) = self.named.get(name) {
                if prior != traits {
                    return Err(BootstrapReadError::ConflictingNamedParameter {
                        name: name.to_owned(),
                    });
                }
                return Ok(parameter.clone());
            }
            let parameter = self.fresh();
            self.named
                .insert(name.to_owned(), (traits.to_vec(), parameter.clone()));
            source
                .named_parameters
                .insert(parameter.clone(), name.to_owned());
            Ok(parameter)
        } else if let Some(parameter) = self.inferred.get(traits) {
            Ok(parameter.clone())
        } else {
            let parameter = self.fresh();
            self.inferred.insert(traits.to_vec(), parameter.clone());
            Ok(parameter)
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

fn parse_type_expression(
    node: &SyntaxNode,
    environment: &ResolutionEnvironment<'_>,
    parameters: &mut ParameterScope,
    source: &mut BootstrapSourceMetadata,
) -> Result<TypeExpression, BootstrapReadError> {
    match node {
        SyntaxNode::Atom { text, .. } => Ok(TypeExpression::Reference(
            environment.resolve(text, ExpectedNameClass::Nominal)?,
        )),
        SyntaxNode::AngleApplication {
            head, arguments, ..
        } => {
            let arguments = arguments
                .iter()
                .map(|argument| parse_type_expression(argument, environment, parameters, source))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(TypeExpression::ShapeApplication(ShapeApplication {
                shape: environment.resolve(head, ExpectedNameClass::Shape)?,
                arguments,
            }))
        }
        SyntaxNode::Delimited {
            delimiter: Delimiter::Guillemets,
            items,
            ..
        } => {
            if items.is_empty() {
                return Err(unexpected_node(node, "nonempty Trait requirement"));
            }
            let mut binder = None;
            let mut required_traits = Vec::new();
            for (index, item) in items.iter().enumerate() {
                let trait_name = match item {
                    SyntaxNode::Atom { text, .. } => text.as_str(),
                    SyntaxNode::Application { head, payload, .. } if index == 0 => {
                        let (trait_name, _) =
                            expect_atom(payload, "Trait reference after local binder")?;
                        binder = Some(head.as_str());
                        trait_name
                    }
                    _ => {
                        return Err(unexpected_node(
                            item,
                            "Trait reference or first named binder",
                        ));
                    }
                };
                required_traits.push(environment.resolve(trait_name, ExpectedNameClass::Trait)?);
            }
            required_traits.sort();
            for pair in required_traits.windows(2) {
                if pair[0] == pair[1] {
                    return Err(BootstrapReadError::DuplicateTrait(pair[0].clone()));
                }
            }
            let parameter = parameters.parameter(binder, &required_traits, source)?;
            Ok(TypeExpression::TraitRequirement(TraitRequirement {
                parameter,
                required_traits,
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
