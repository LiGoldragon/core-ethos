//! `TextualSchema` — source-bounded schema text and stringless encoded values.

use name_table::{Name, NameInterner, NameResolver, NameTable, NameTransaction};
use raw_discovery::{BlockTree, DiscoveredBlockTree};
use structural_codec::{
    ApplicationDelimitedBody, ApplicationDelimitedHead, ApplicationDelimitedItems,
    ApplicationDelimitedRoot, ApplicationHead, ApplicationPayload, ApplicationRoot, EncodedForm,
    FieldValue, StructuralEvaluator, StructuralValue, Textual, UnaryRoot,
};

use crate::declaration::{
    DeclarationRole, EncodedDeclaration, EncodedEnum, EncodedField, EncodedNewtype, EncodedSchema,
    EncodedStruct, EncodedType, EncodedVariant,
};
use crate::document::{
    DECLARATION, DeclarationConstructor, FIELD as DOCUMENT_FIELD, INTERFACE, INTERFACE_VARIANT,
    ReferenceConstructor, SchemaDocumentGrammar, TYPE_REFERENCE, TYPES_BLOCK,
};
use crate::error::TextualError;
use crate::fixture::{FIELD as FIXTURE_FIELD, FixtureFamily};
use crate::reference::{BuiltinReference, EncodedReference};
use crate::rules::{DelimitedItems, DelimitedRoot, SchemaRule};
use crate::universe::{ENCODED_UNIVERSE, EncodedUniverse, EncodedUniverseBuilder};

#[derive(Clone, Debug)]
pub struct TextualSchema {
    universe: EncodedUniverse,
    table: structural_codec::AddressedStructuralTable<SchemaRule>,
}

impl TextualSchema {
    pub fn fixture() -> Result<Self, TextualError> {
        let family = FixtureFamily::build();
        Ok(Self {
            universe: family.universe().clone(),
            table: family.standard_table()?,
        })
    }

    pub fn schema_document() -> Result<Self, TextualError> {
        let grammar = SchemaDocumentGrammar::build()?;
        Ok(Self {
            universe: EncodedUniverseBuilder::new().build(ENCODED_UNIVERSE)?,
            table: grammar.table().clone(),
        })
    }

    pub fn new(
        universe: EncodedUniverse,
        table: structural_codec::AddressedStructuralTable<SchemaRule>,
    ) -> Self {
        Self { universe, table }
    }

    pub fn universe(&self) -> &EncodedUniverse {
        &self.universe
    }

    pub fn table(&self) -> &structural_codec::AddressedStructuralTable<SchemaRule> {
        &self.table
    }

    /// Decode through one table-owned, source-bounded evaluator and one name-table
    /// transaction.  Reification failures therefore leave the caller's table intact.
    pub fn decode(
        &self,
        expected: structural_codec::ScopedEncodedTypeId,
        text: &str,
        names: &mut NameTable,
    ) -> Result<EncodedType, TextualError> {
        let evaluator = self.schema_evaluator()?;
        names.try_intern(|transaction| {
            let mirror = evaluator.decode_text_with_interner(expected, text, transaction)?;
            self.reify_type(expected, &mirror, transaction)
        })
    }

    /// Compatibility signature retained for callers. Reflection is lookup-only:
    /// callers must preload any scalar or projection spelling the value needs.
    pub fn encode(
        &self,
        expected: structural_codec::ScopedEncodedTypeId,
        value: &EncodedType,
        names: &mut NameTable,
    ) -> Result<String, TextualError> {
        let mirror = self.reflect_type(expected, value, names)?;
        Ok(self
            .schema_evaluator()?
            .encode_text(expected, &mirror, names)?)
    }

    fn schema_evaluator(&self) -> Result<StructuralEvaluator<'_, SchemaRule>, TextualError> {
        Ok(StructuralEvaluator::new(&self.table)?)
    }

    // ===== single declaration reification =====

    fn reify_type<Names: NameInterner + NameResolver + ?Sized>(
        &self,
        expected: structural_codec::ScopedEncodedTypeId,
        value: &StructuralValue,
        names: &mut Names,
    ) -> Result<EncodedType, TextualError> {
        match self.universe.encoded_type(expected) {
            Some(EncodedType::Newtype(_)) => self.reify_newtype(value, names),
            Some(EncodedType::Struct(_)) => self.reify_struct(value, names),
            Some(EncodedType::Enumeration(_)) => self.reify_enumeration(value),
            None => Err(TextualError::ReifyShape("non-declaration expected type")),
        }
    }

    fn reify_newtype<Resolver: NameResolver + ?Sized>(
        &self,
        value: &StructuralValue,
        names: &Resolver,
    ) -> Result<EncodedType, TextualError> {
        let (name, body) = Self::application_delimited(value, "newtype")?;
        let [FieldValue::Atom(inner)] = body else {
            return Err(TextualError::ReifyShape("newtype body"));
        };
        Ok(EncodedType::Newtype(EncodedNewtype::new(
            name,
            self.reference_from_atom(*inner, names)?,
        )))
    }

    fn reify_struct<Names: NameInterner + NameResolver + ?Sized>(
        &self,
        value: &StructuralValue,
        names: &mut Names,
    ) -> Result<EncodedType, TextualError> {
        let (name, body) = Self::application_delimited(value, "struct")?;
        let fields = body
            .iter()
            .map(|field| self.reify_field(field, names))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(EncodedType::Struct(EncodedStruct::new(name, fields)))
    }

    fn reify_enumeration(&self, value: &StructuralValue) -> Result<EncodedType, TextualError> {
        let (name, body) = Self::application_delimited(value, "enumeration")?;
        let variants = body
            .iter()
            .map(|field| match field {
                FieldValue::Atom(identifier) => Ok(EncodedVariant::new(*identifier, None)),
                _ => Err(TextualError::ReifyShape("enumeration variant")),
            })
            .collect::<Result<_, _>>()?;
        Ok(EncodedType::Enumeration(EncodedEnum::new(name, variants)))
    }

    fn application_delimited<'value>(
        value: &'value StructuralValue,
        what: &'static str,
    ) -> Result<(name_table::Identifier, &'value [FieldValue]), TextualError> {
        let Some(FieldValue::Atom(name)) = value.field::<ApplicationDelimitedHead>() else {
            return Err(TextualError::ReifyShape(what));
        };
        let Some(FieldValue::Repeated(items)) = value.field::<ApplicationDelimitedItems>() else {
            return Err(TextualError::ReifyShape(what));
        };
        Ok((*name, items))
    }

    fn reify_field<Names: NameInterner + NameResolver + ?Sized>(
        &self,
        field: &FieldValue,
        names: &mut Names,
    ) -> Result<EncodedField, TextualError> {
        let FieldValue::Delegated(inner) = field else {
            return Err(TextualError::ReifyShape("struct field delegate"));
        };
        let Some(FieldValue::Atom(type_id)) = inner.field::<UnaryRoot>() else {
            return Err(TextualError::ReifyShape("struct field type"));
        };
        let reference = self.reference_from_atom(*type_id, names)?;
        let identifier = names.intern(Name::new(reference.derived_field_name(names)?))?;
        Ok(EncodedField::new(identifier, reference))
    }

    fn reference_from_atom<Resolver: NameResolver + ?Sized>(
        &self,
        type_id: name_table::Identifier,
        names: &Resolver,
    ) -> Result<EncodedReference, TextualError> {
        Ok(self.universe.reference_from_name(type_id, names)?)
    }

    // ===== single declaration reflection =====

    /// The sole reflection path for inherent encoding, document encoding, and
    /// `Textual::view`. It only resolves existing names and never interns.
    fn reflect_type(
        &self,
        expected: structural_codec::ScopedEncodedTypeId,
        value: &EncodedType,
        names: &NameTable,
    ) -> Result<StructuralValue, TextualError> {
        match value {
            EncodedType::Newtype(newtype) => self.reflect_newtype(expected, newtype, names),
            EncodedType::Struct(structure) => self.reflect_struct(expected, structure, names),
            EncodedType::Enumeration(enumeration) => {
                Self::reflect_enumeration(expected, enumeration)
            }
        }
    }

    fn reflect_newtype(
        &self,
        expected: structural_codec::ScopedEncodedTypeId,
        newtype: &EncodedNewtype,
        names: &NameTable,
    ) -> Result<StructuralValue, TextualError> {
        let inner = Self::type_atom_from_table(newtype.reference(), names)?
            .ok_or(TextualError::ReifyShape("newtype inner reference"))?;
        Self::application_delimited_mirror(
            expected,
            0,
            newtype.identifier(),
            vec![FieldValue::Atom(inner)],
        )
    }

    fn reflect_struct(
        &self,
        expected: structural_codec::ScopedEncodedTypeId,
        structure: &EncodedStruct,
        names: &NameTable,
    ) -> Result<StructuralValue, TextualError> {
        let fields = structure
            .fields()
            .iter()
            .map(|field| Self::reflect_field(field, names, FIXTURE_FIELD))
            .collect::<Result<_, _>>()?;
        Self::application_delimited_mirror(expected, 0, structure.identifier(), fields)
    }

    fn reflect_field(
        field: &EncodedField,
        names: &NameTable,
        field_type: structural_codec::ScopedEncodedTypeId,
    ) -> Result<FieldValue, TextualError> {
        let type_id = Self::type_atom_from_table(field.reference(), names)?
            .ok_or(TextualError::ReifyShape("field type reference"))?;
        Ok(FieldValue::Delegated(Box::new(Self::unary_mirror(
            field_type,
            0,
            FieldValue::Atom(type_id),
        )?)))
    }

    fn type_atom_from_table(
        reference: &EncodedReference,
        names: &NameTable,
    ) -> Result<Option<name_table::Identifier>, TextualError> {
        let builtin = match reference {
            EncodedReference::Integer => Some(BuiltinReference::Integer),
            EncodedReference::String => Some(BuiltinReference::String),
            EncodedReference::Boolean => Some(BuiltinReference::Boolean),
            EncodedReference::Bytes => Some(BuiltinReference::Bytes),
            EncodedReference::Plain(identifier) => return Ok(Some(*identifier)),
            EncodedReference::SingleTypeApplication { .. }
            | EncodedReference::MultiTypeApplication { .. }
            | EncodedReference::ValueApplication { .. } => return Ok(None),
        };
        Ok(Some(Self::builtin_identifier(
            builtin.expect("scalar builtin"),
            names,
        )?))
    }

    fn builtin_identifier(
        builtin: BuiltinReference,
        names: &NameTable,
    ) -> Result<name_table::Identifier, TextualError> {
        names
            .lookup(&Name::new(builtin.spelling()))
            .ok_or(TextualError::ReflectionNameAbsent {
                spelling: builtin.spelling(),
            })
    }

    fn reflect_enumeration(
        expected: structural_codec::ScopedEncodedTypeId,
        enumeration: &EncodedEnum,
    ) -> Result<StructuralValue, TextualError> {
        let variants = enumeration
            .variants()
            .iter()
            .map(|variant| {
                if variant.payload().is_some() {
                    Err(TextualError::ReifyShape(
                        "enumeration declaration payload variant",
                    ))
                } else {
                    Ok(FieldValue::Atom(variant.identifier()))
                }
            })
            .collect::<Result<_, _>>()?;
        Self::application_delimited_mirror(expected, 0, enumeration.identifier(), variants)
    }

    // ===== document reification and reflection =====

    pub fn decode_document(
        &self,
        text: &str,
        names: &mut NameTable,
    ) -> Result<EncodedSchema, TextualError> {
        let roots = self.document_roots(text)?;
        if roots.len() != crate::document::DOCUMENT_SLOTS {
            return Err(TextualError::DocumentArity(roots.len()));
        }
        if !Self::empty_brace(roots[0]) {
            return Err(TextualError::DocumentSlot("imports"));
        }
        if !Self::empty_brace(roots[4]) {
            return Err(TextualError::DocumentSlot("generics"));
        }
        if !Self::empty_brace(roots[5]) {
            return Err(TextualError::DocumentSlot("impls"));
        }
        let evaluator = self.schema_evaluator()?;
        names.try_intern(|transaction| {
            let input = self.decode_interface_slot(
                &evaluator,
                roots[1],
                DeclarationRole::InterfaceInput,
                transaction,
            )?;
            let output = self.decode_interface_slot(
                &evaluator,
                roots[2],
                DeclarationRole::InterfaceOutput,
                transaction,
            )?;
            let types = self.decode_types_slot(&evaluator, roots[3], transaction)?;
            let mut declarations = Vec::with_capacity(types.len() + 2);
            declarations.push(input);
            declarations.push(output);
            declarations.extend(types);
            for declaration in &declarations {
                self.universe
                    .validate_declaration_name(declaration.identifier(), transaction)?;
            }
            Ok(EncodedSchema::new(declarations))
        })
    }

    pub fn encode_document(
        &self,
        schema: &EncodedSchema,
        names: &mut NameTable,
    ) -> Result<String, TextualError> {
        let input = schema
            .input()
            .ok_or(TextualError::MissingInterfaceRoot("input"))?;
        let output = schema
            .output()
            .ok_or(TextualError::MissingInterfaceRoot("output"))?;
        let evaluator = self.schema_evaluator()?;
        Ok([
            "{}".to_owned(),
            evaluator.encode_text(
                INTERFACE,
                &self.reflect_interface(input.value(), names)?,
                names,
            )?,
            evaluator.encode_text(
                INTERFACE,
                &self.reflect_interface(output.value(), names)?,
                names,
            )?,
            evaluator.encode_text(
                TYPES_BLOCK,
                &self.reflect_types(schema.data_declarations(), names)?,
                names,
            )?,
            "{}".to_owned(),
            "{}".to_owned(),
        ]
        .join("\n"))
    }

    fn document_roots<'source>(
        &self,
        source: &'source str,
    ) -> Result<Vec<&'source str>, TextualError> {
        let tree = DiscoveredBlockTree::discover(
            source,
            self.table.token_profile(),
            self.table.block_discovery(),
        )
        .map_err(structural_codec::DecodeError::from)?;
        tree.root_blocks()
            .iter()
            .map(|block| {
                let bound = block.source_bound();
                source
                    .get(bound.start()..bound.end())
                    .ok_or(TextualError::ReifyShape("document source bound"))
            })
            .collect()
    }

    fn empty_brace(source: &str) -> bool {
        source.trim() == "{}"
    }

    fn decode_interface_slot<Names: NameInterner + NameResolver>(
        &self,
        evaluator: &StructuralEvaluator<'_, SchemaRule>,
        source: &str,
        role: DeclarationRole,
        names: &mut Names,
    ) -> Result<EncodedDeclaration, TextualError> {
        let mirror = evaluator.decode_text_with_interner(INTERFACE, source, names)?;
        let variants = self.reify_interface_variants(&mirror, names)?;
        let name = names.intern(Name::new(
            role.interface_root_name()
                .ok_or(TextualError::ReifyShape("interface role"))?,
        ))?;
        Ok(EncodedDeclaration::interface(
            role,
            EncodedType::Enumeration(EncodedEnum::new(name, variants)),
        ))
    }

    fn decode_types_slot<Names: NameInterner + NameResolver>(
        &self,
        evaluator: &StructuralEvaluator<'_, SchemaRule>,
        source: &str,
        names: &mut Names,
    ) -> Result<Vec<EncodedDeclaration>, TextualError> {
        let mirror = evaluator.decode_text_with_interner(TYPES_BLOCK, source, names)?;
        self.reify_types(&mirror, names)
    }

    fn reify_types<Names: NameInterner + NameResolver + ?Sized>(
        &self,
        value: &StructuralValue,
        names: &mut Names,
    ) -> Result<Vec<EncodedDeclaration>, TextualError> {
        let Some(FieldValue::Repeated(items)) = value.field::<DelimitedItems>() else {
            return Err(TextualError::ReifyShape("types block declarations"));
        };
        items
            .iter()
            .map(|item| {
                let FieldValue::Delegated(declaration) = item else {
                    return Err(TextualError::ReifyShape("declaration delegate"));
                };
                self.reify_declaration(declaration, names)
            })
            .collect()
    }

    fn reify_declaration<Names: NameInterner + NameResolver + ?Sized>(
        &self,
        value: &StructuralValue,
        names: &mut Names,
    ) -> Result<EncodedDeclaration, TextualError> {
        let constructor = DeclarationConstructor::from_index(value.constructor().local())
            .ok_or(TextualError::ReifyShape("declaration constructor"))?;
        let Some(FieldValue::Atom(name)) = (match constructor {
            DeclarationConstructor::Newtype => value.field::<ApplicationHead>(),
            DeclarationConstructor::Struct | DeclarationConstructor::Enumeration => {
                value.field::<ApplicationDelimitedHead>()
            }
        }) else {
            return Err(TextualError::ReifyShape("declaration name"));
        };
        let encoded = match constructor {
            DeclarationConstructor::Newtype => {
                let Some(FieldValue::Delegated(reference)) = value.field::<ApplicationPayload>()
                else {
                    return Err(TextualError::ReifyShape("newtype reference"));
                };
                EncodedType::Newtype(EncodedNewtype::new(
                    *name,
                    self.reify_reference(reference, names)?,
                ))
            }
            DeclarationConstructor::Struct => {
                let Some(FieldValue::Repeated(fields)) = value.field::<ApplicationDelimitedItems>()
                else {
                    return Err(TextualError::ReifyShape("struct fields"));
                };
                EncodedType::from_braced_body(
                    *name,
                    fields
                        .iter()
                        .map(|field| self.reify_field(field, names))
                        .collect::<Result<_, _>>()?,
                )
            }
            DeclarationConstructor::Enumeration => {
                let Some(FieldValue::Repeated(variants)) =
                    value.field::<ApplicationDelimitedItems>()
                else {
                    return Err(TextualError::ReifyShape("enumeration variants"));
                };
                EncodedType::Enumeration(EncodedEnum::new(
                    *name,
                    variants
                        .iter()
                        .map(|variant| match variant {
                            FieldValue::Atom(identifier) => {
                                Ok(EncodedVariant::new(*identifier, None))
                            }
                            _ => Err(TextualError::ReifyShape("enumeration variant")),
                        })
                        .collect::<Result<_, _>>()?,
                ))
            }
        };
        Ok(EncodedDeclaration::public(encoded))
    }

    fn reify_reference<Resolver: NameResolver + ?Sized>(
        &self,
        value: &StructuralValue,
        names: &Resolver,
    ) -> Result<EncodedReference, TextualError> {
        match ReferenceConstructor::from_index(value.constructor().local()) {
            Some(ReferenceConstructor::Name) => {
                let Some(FieldValue::Atom(identifier)) = value.field::<UnaryRoot>() else {
                    return Err(TextualError::ReifyShape("bare reference name"));
                };
                self.reference_from_atom(*identifier, names)
            }
            Some(ReferenceConstructor::Application) => {
                let Some(FieldValue::Atom(head)) = value.field::<ApplicationHead>() else {
                    return Err(TextualError::ReifyShape("reference application head"));
                };
                let Some(FieldValue::Delegated(argument)) = value.field::<ApplicationPayload>()
                else {
                    return Err(TextualError::ReifyShape("reference application"));
                };
                let projection = self
                    .universe
                    .builtin_from_name(*head, names)?
                    .and_then(BuiltinReference::single_projection)
                    .ok_or(TextualError::ReifyShape("universe projection definition"))?;
                Ok(EncodedReference::SingleTypeApplication {
                    projection,
                    argument: Box::new(self.reify_reference(argument, names)?),
                })
            }
            None => Err(TextualError::ReifyShape("type reference constructor")),
        }
    }

    fn reify_interface_variants<Resolver: NameResolver + ?Sized>(
        &self,
        value: &StructuralValue,
        names: &Resolver,
    ) -> Result<Vec<EncodedVariant>, TextualError> {
        let Some(FieldValue::Repeated(entries)) = value.field::<DelimitedItems>() else {
            return Err(TextualError::ReifyShape("interface entries"));
        };
        entries
            .iter()
            .map(|entry| {
                let FieldValue::Delegated(entry) = entry else {
                    return Err(TextualError::ReifyShape("interface entry delegate"));
                };
                let Some(FieldValue::Atom(name)) = entry.field::<ApplicationHead>() else {
                    return Err(TextualError::ReifyShape("interface entry name"));
                };
                let Some(FieldValue::Delegated(reference)) = entry.field::<ApplicationPayload>()
                else {
                    return Err(TextualError::ReifyShape("interface entry payload"));
                };
                Ok(EncodedVariant::new(
                    *name,
                    Some(self.reify_reference(reference, names)?),
                ))
            })
            .collect()
    }

    fn reflect_types<'declaration>(
        &self,
        declarations: impl Iterator<Item = &'declaration EncodedDeclaration>,
        names: &NameTable,
    ) -> Result<StructuralValue, TextualError> {
        let items = declarations
            .map(|declaration| {
                self.reflect_declaration(declaration, names)
                    .map(|value| FieldValue::Delegated(Box::new(value)))
            })
            .collect::<Result<_, _>>()?;
        Self::delimited_mirror(TYPES_BLOCK, items)
    }

    fn reflect_declaration(
        &self,
        declaration: &EncodedDeclaration,
        names: &NameTable,
    ) -> Result<StructuralValue, TextualError> {
        match declaration.value() {
            EncodedType::Newtype(newtype) => Self::application_mirror(
                DECLARATION,
                DeclarationConstructor::Newtype.index(),
                newtype.identifier(),
                FieldValue::Delegated(Box::new(
                    self.reflect_reference(newtype.reference(), names)?,
                )),
            ),
            EncodedType::Struct(structure) => Self::application_delimited_mirror(
                DECLARATION,
                DeclarationConstructor::Struct.index(),
                structure.identifier(),
                structure
                    .fields()
                    .iter()
                    .map(|field| Self::reflect_field(field, names, DOCUMENT_FIELD))
                    .collect::<Result<_, _>>()?,
            ),
            EncodedType::Enumeration(enumeration) => Self::application_delimited_mirror(
                DECLARATION,
                DeclarationConstructor::Enumeration.index(),
                enumeration.identifier(),
                enumeration
                    .variants()
                    .iter()
                    .map(|variant| {
                        if variant.payload().is_some() {
                            Err(TextualError::ReifyShape(
                                "enumeration declaration payload variant",
                            ))
                        } else {
                            Ok(FieldValue::Atom(variant.identifier()))
                        }
                    })
                    .collect::<Result<_, _>>()?,
            ),
        }
    }

    fn reflect_interface(
        &self,
        interface: &EncodedType,
        names: &NameTable,
    ) -> Result<StructuralValue, TextualError> {
        let EncodedType::Enumeration(enumeration) = interface else {
            return Err(TextualError::ReifyShape("interface root enumeration"));
        };
        let items = enumeration
            .variants()
            .iter()
            .map(|variant| {
                let reference = variant
                    .payload()
                    .ok_or(TextualError::ReifyShape("interface entry payload"))?;
                Ok(FieldValue::Delegated(Box::new(Self::application_mirror(
                    INTERFACE_VARIANT,
                    0,
                    variant.identifier(),
                    FieldValue::Delegated(Box::new(self.reflect_reference(reference, names)?)),
                )?)))
            })
            .collect::<Result<_, TextualError>>()?;
        Self::delimited_mirror(INTERFACE, items)
    }

    fn reflect_reference(
        &self,
        reference: &EncodedReference,
        names: &NameTable,
    ) -> Result<StructuralValue, TextualError> {
        match reference {
            EncodedReference::Integer => Self::unary_mirror(
                TYPE_REFERENCE,
                0,
                FieldValue::Atom(Self::builtin_identifier(BuiltinReference::Integer, names)?),
            ),
            EncodedReference::String => Self::unary_mirror(
                TYPE_REFERENCE,
                0,
                FieldValue::Atom(Self::builtin_identifier(BuiltinReference::String, names)?),
            ),
            EncodedReference::Boolean => Self::unary_mirror(
                TYPE_REFERENCE,
                0,
                FieldValue::Atom(Self::builtin_identifier(BuiltinReference::Boolean, names)?),
            ),
            EncodedReference::Bytes => Self::unary_mirror(
                TYPE_REFERENCE,
                0,
                FieldValue::Atom(Self::builtin_identifier(BuiltinReference::Bytes, names)?),
            ),
            EncodedReference::Plain(identifier) => {
                Self::unary_mirror(TYPE_REFERENCE, 0, FieldValue::Atom(*identifier))
            }
            EncodedReference::SingleTypeApplication {
                projection,
                argument,
            } => {
                let builtin = match projection {
                    crate::reference::SingleTypeReferenceProjection::Vector => {
                        BuiltinReference::Vector
                    }
                    crate::reference::SingleTypeReferenceProjection::Optional => {
                        BuiltinReference::Optional
                    }
                    crate::reference::SingleTypeReferenceProjection::ScopeOf => {
                        BuiltinReference::ScopeOf
                    }
                };
                Self::application_mirror(
                    TYPE_REFERENCE,
                    1,
                    Self::builtin_identifier(builtin, names)?,
                    FieldValue::Delegated(Box::new(self.reflect_reference(argument, names)?)),
                )
            }
            EncodedReference::MultiTypeApplication { .. } => {
                Err(TextualError::ReifyShape("multi-type application encode"))
            }
            EncodedReference::ValueApplication { .. } => {
                Err(TextualError::ReifyShape("value application encode"))
            }
        }
    }

    // ===== checked typed mirror construction =====

    fn unary_mirror(
        type_id: structural_codec::ScopedEncodedTypeId,
        constructor: u16,
        root: FieldValue,
    ) -> Result<StructuralValue, TextualError> {
        let mut record = StructuralValue::record(structural_codec::EncodedConstructorId::under(
            type_id,
            constructor,
        ));
        record.insert::<UnaryRoot>(root)?;
        Ok(record.finish())
    }

    fn application_mirror(
        type_id: structural_codec::ScopedEncodedTypeId,
        constructor: u16,
        head: name_table::Identifier,
        payload: FieldValue,
    ) -> Result<StructuralValue, TextualError> {
        let head_value = FieldValue::Atom(head);
        let root = FieldValue::Application {
            head: Box::new(head_value.clone()),
            payload: Box::new(payload.clone()),
        };
        let mut record = StructuralValue::record(structural_codec::EncodedConstructorId::under(
            type_id,
            constructor,
        ));
        record.insert::<ApplicationRoot>(root)?;
        record.insert::<ApplicationHead>(head_value)?;
        record.insert::<ApplicationPayload>(payload)?;
        Ok(record.finish())
    }

    fn application_delimited_mirror(
        type_id: structural_codec::ScopedEncodedTypeId,
        constructor: u16,
        head: name_table::Identifier,
        items: Vec<FieldValue>,
    ) -> Result<StructuralValue, TextualError> {
        let head_value = FieldValue::Atom(head);
        let item_value = FieldValue::Repeated(items);
        let body_value = FieldValue::Delimited(Box::new(item_value.clone()));
        let root = FieldValue::Application {
            head: Box::new(head_value.clone()),
            payload: Box::new(body_value.clone()),
        };
        let mut record = StructuralValue::record(structural_codec::EncodedConstructorId::under(
            type_id,
            constructor,
        ));
        record.insert::<ApplicationDelimitedRoot>(root)?;
        record.insert::<ApplicationDelimitedHead>(head_value)?;
        record.insert::<ApplicationDelimitedBody>(body_value)?;
        record.insert::<ApplicationDelimitedItems>(item_value)?;
        Ok(record.finish())
    }

    fn delimited_mirror(
        type_id: structural_codec::ScopedEncodedTypeId,
        items: Vec<FieldValue>,
    ) -> Result<StructuralValue, TextualError> {
        let item_value = FieldValue::Repeated(items);
        let root = FieldValue::Delimited(Box::new(item_value.clone()));
        let mut record =
            StructuralValue::record(structural_codec::EncodedConstructorId::under(type_id, 0));
        record.insert::<DelimitedRoot>(root)?;
        record.insert::<DelimitedItems>(item_value)?;
        Ok(record.finish())
    }
}

impl Textual<SchemaRule> for TextualSchema {
    type Encoded = EncodedType;
    type Language = SchemaLanguage;
    type Error = TextualError;

    fn structuretree(&self) -> &structural_codec::AddressedStructuralTable<SchemaRule> {
        &self.table
    }

    fn missing_root_object(&self) -> Self::Error {
        TextualError::EmptySource
    }

    fn reify(
        &self,
        expected: structural_codec::ScopedEncodedTypeId,
        mirror: &StructuralValue,
        names: &mut NameTransaction<'_>,
    ) -> Result<Self::Encoded, Self::Error> {
        self.reify_type(expected, mirror, names)
    }

    fn reflect(
        &self,
        expected: structural_codec::ScopedEncodedTypeId,
        encoded: &Self::Encoded,
        names: &NameTable,
    ) -> Result<StructuralValue, Self::Error> {
        self.reflect_type(expected, encoded, names)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchemaLanguage;

impl EncodedForm for EncodedType {
    type Language = SchemaLanguage;
}

impl EncodedForm for EncodedSchema {
    type Language = SchemaLanguage;
}
