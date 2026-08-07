//! Canonical textual projection over the same bidirectional name snapshot used
//! during sealing.

use name_table::EncodedName;

use super::catalog::TextualMetadataSnapshot;
use super::error::BootstrapWriteError;
use super::model::*;
use super::reader::{BootstrapNamingAuthority, BootstrapReader, PreparedBootstrapTransaction};
use super::root::{BootstrapSectionSchema as SectionSchema, RootSemanticSectionRef};

impl<Authority: BootstrapNamingAuthority> BootstrapReader<Authority> {
    /// Validate and write one canonical authored projection.
    pub fn write(
        &self,
        transaction: &PreparedBootstrapTransaction<Authority>,
    ) -> Result<String, BootstrapWriteError> {
        self.validate_transaction(transaction)?;
        let decoded = transaction.decoded();
        let snapshot = transaction.naming_transition().after();
        let root = self.roots().for_kind(decoded.document.header.kind);
        let mut output = format!(
            "{}.{{{} {} {}}}\n",
            spelling(snapshot, &root.kind_identity)?,
            decoded.document.header.version.major,
            decoded.document.header.version.minor,
            decoded.document.header.version.patch,
        );
        output.push('[');
        for (index, import) in decoded.source.imports.iter().enumerate() {
            if index > 0 {
                output.push(' ');
            }
            output.push_str(&import.module_path.join(":"));
            output.push_str(".[");
            output.push_str(&import.imported_names.join(" "));
            output.push(']');
        }
        output.push_str("]\n{\n");
        let sections = root.semantic_sections(&decoded.document.body)?;
        for (index, (schema, section)) in root.sections.iter().zip(sections).enumerate() {
            if index > 0 {
                output.push('\n');
            }
            write_section(&mut output, *schema, section, snapshot)?;
        }
        output.push_str("\n}\n");
        Ok(output)
    }
}

fn write_section(
    output: &mut String,
    schema: SectionSchema,
    section: RootSemanticSectionRef<'_>,
    snapshot: &TextualMetadataSnapshot,
) -> Result<(), BootstrapWriteError> {
    output.push_str("  [");
    match (schema, section) {
        (SectionSchema::Role(_), RootSemanticSectionRef::Role(entries)) => {
            write_separated(output, entries, |output, entry| match entry {
                RoleEntry::Declaration(declaration) => {
                    write_type_declaration(output, declaration, snapshot)
                }
                RoleEntry::Reference(reference) => {
                    output.push_str(spelling(snapshot, reference)?);
                    Ok(())
                }
            })?;
        }
        (
            SectionSchema::Declarations,
            RootSemanticSectionRef::Declarations(declarations),
        ) => {
            write_separated(output, declarations, |output, declaration| {
                write_declaration(output, declaration, snapshot)
            })?;
        }
        (SectionSchema::Traits, RootSemanticSectionRef::Traits(traits)) => {
            write_separated(output, traits, |output, declaration| {
                write_trait(output, declaration, snapshot)
            })?;
        }
        (
            SectionSchema::PersistentDeclarations,
            RootSemanticSectionRef::PersistentDeclarations(declarations),
        ) => {
            write_separated(output, declarations, |output, declaration| {
                write_type_declaration(output, declaration, snapshot)
            })?;
        }
        (SectionSchema::Tables, RootSemanticSectionRef::Tables(tables)) => {
            write_separated(output, tables, |output, table| {
                output.push_str(spelling(snapshot, &table.name)?);
                output.push_str(".{");
                output.push_str(spelling(snapshot, &table.record_type)?);
                output.push(' ');
                output.push_str(spelling(snapshot, &table.key_type)?);
                output.push('}');
                Ok(())
            })?;
        }
        _ => unreachable!("validate_prepared proves registry/body section agreement"),
    }
    output.push(']');
    Ok(())
}

fn write_declaration(
    output: &mut String,
    declaration: &Declaration,
    snapshot: &TextualMetadataSnapshot,
) -> Result<(), BootstrapWriteError> {
    let Declaration::Type(declaration) = declaration;
    write_type_declaration(output, declaration, snapshot)
}

fn write_type_declaration(
    output: &mut String,
    declaration: &TypeDeclaration,
    snapshot: &TextualMetadataSnapshot,
) -> Result<(), BootstrapWriteError> {
    output.push_str(spelling(snapshot, &declaration.name)?);
    match &declaration.body {
        TypeBody::Newtype(expression) => {
            output.push('.');
            write_type_expression(output, expression, snapshot)?;
        }
        TypeBody::Struct(fields) => {
            output.push_str(".{");
            write_separated(output, fields, |output, field| {
                write_type_expression(output, field, snapshot)
            })?;
            output.push('}');
        }
        TypeBody::Enum(variants) => {
            output.push_str(".[");
            write_separated(output, variants, |output, variant| {
                output.push_str(spelling(snapshot, &variant.name)?);
                match &variant.body {
                    VariantBody::Unit => Ok(()),
                    VariantBody::Unary(expression) => {
                        output.push('.');
                        write_type_expression(output, expression, snapshot)
                    }
                    VariantBody::Product(fields) => {
                        output.push_str(".{");
                        write_separated(output, fields, |output, field| {
                            write_type_expression(output, field, snapshot)
                        })?;
                        output.push('}');
                        Ok(())
                    }
                }
            })?;
            output.push(']');
        }
    }
    Ok(())
}

fn write_trait(
    output: &mut String,
    declaration: &TraitDeclaration,
    snapshot: &TextualMetadataSnapshot,
) -> Result<(), BootstrapWriteError> {
    output.push_str(spelling(snapshot, &declaration.name)?);
    output.push_str(".{");
    write_separated(output, &declaration.methods, |output, method| {
        output.push_str(spelling(snapshot, &method.name)?);
        output.push_str(".{");
        for parameter in &method.parameters {
            write_type_expression(output, parameter, snapshot)?;
            output.push(' ');
        }
        write_type_expression(output, &method.return_type, snapshot)?;
        output.push('}');
        Ok(())
    })?;
    output.push('}');
    Ok(())
}

fn write_type_expression(
    output: &mut String,
    expression: &TypeExpression,
    snapshot: &TextualMetadataSnapshot,
) -> Result<(), BootstrapWriteError> {
    match expression {
        TypeExpression::Reference(reference) => output.push_str(spelling(snapshot, reference)?),
        TypeExpression::ShapeApplication(application) => {
            output.push_str(spelling(snapshot, &application.shape)?);
            output.push('<');
            write_separated(output, &application.arguments, |output, argument| {
                write_type_expression(output, argument, snapshot)
            })?;
            output.push('>');
        }
        TypeExpression::TraitRequirement(requirement) => {
            output.push('«');
            for (index, required) in requirement.required_traits().iter().enumerate() {
                if index > 0 {
                    output.push(' ');
                }
                if index == 0 {
                    if let ParameterBinder::Named { local_name, .. } = requirement.binder() {
                        output.push_str(local_name);
                        output.push('.');
                    }
                }
                output.push_str(spelling(snapshot, required)?);
            }
            output.push('»');
        }
    }
    Ok(())
}

fn spelling<'a>(
    snapshot: &'a TextualMetadataSnapshot,
    identity: &EncodedName,
) -> Result<&'a str, BootstrapWriteError> {
    snapshot
        .spelling(identity)
        .ok_or_else(|| BootstrapWriteError::MissingSpelling(identity.clone()))
}

fn write_separated<T, Write>(
    output: &mut String,
    values: &[T],
    mut write: Write,
) -> Result<(), BootstrapWriteError>
where
    Write: FnMut(&mut String, &T) -> Result<(), BootstrapWriteError>,
{
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push(' ');
        }
        write(output, value)?;
    }
    Ok(())
}
