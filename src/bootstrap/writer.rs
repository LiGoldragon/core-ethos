//! Canonical textual projection for the strict bootstrap model.

use signal_sema_translator::{VocabularyEncodedId, VocabularyRoot};
use structural_codec::EncodedNameResolver;

use super::error::BootstrapWriteError;
use super::model::*;
use super::reader::{BootstrapPriorVocabulary, BootstrapReader};

impl BootstrapReader {
    /// Write one canonical textual projection. Declaration spellings come from
    /// the injected resolver; source-only imports and local binder spellings come
    /// from the decoded textual metadata.
    pub fn write<Resolver: EncodedNameResolver<VocabularyRoot> + ?Sized>(
        &self,
        decoded: &DecodedBootstrap,
        resolver: &Resolver,
    ) -> Result<String, BootstrapWriteError> {
        write_decoded(decoded, self.priors(), resolver)
    }
}

fn write_decoded<Resolver: EncodedNameResolver<VocabularyRoot> + ?Sized>(
    decoded: &DecodedBootstrap,
    priors: &BootstrapPriorVocabulary,
    resolver: &Resolver,
) -> Result<String, BootstrapWriteError> {
    let header = decoded.document.header;
    let mut output = format!(
        "{}.{{{} {} {}}}\n",
        header.kind.spelling(),
        header.version.major,
        header.version.minor,
        header.version.patch,
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
    match &decoded.document.body {
        BootstrapBody::Interface(body) => {
            write_role_entries(&mut output, &body.inputs, resolver, &decoded.source, priors)?;
            output.push('\n');
            write_role_entries(
                &mut output,
                &body.outputs,
                resolver,
                &decoded.source,
                priors,
            )?;
            output.push('\n');
            write_role_entries(
                &mut output,
                &body.refusals,
                resolver,
                &decoded.source,
                priors,
            )?;
            output.push('\n');
            output.push_str("  [");
            write_separated(&mut output, &body.types, |output, declaration| {
                write_declaration(output, declaration, resolver, &decoded.source, priors)
            })?;
            output.push(']');
        }
        BootstrapBody::Nexus(body) => {
            output.push_str("  [");
            write_separated(&mut output, &body.traits, |output, declaration| {
                write_trait(output, declaration, resolver, &decoded.source)
            })?;
            output.push_str("]\n  [");
            write_separated(&mut output, &body.types, |output, declaration| {
                write_declaration(output, declaration, resolver, &decoded.source, priors)
            })?;
            output.push(']');
        }
        BootstrapBody::Sema(body) => {
            output.push_str("  [");
            write_separated(&mut output, &body.record_types, |output, declaration| {
                write_type_declaration(output, declaration, resolver, &decoded.source)
            })?;
            output.push_str("]\n  [");
            write_separated(&mut output, &body.tables, |output, table| {
                output.push_str(spelling(resolver, &table.name)?);
                output.push_str(".{");
                output.push_str(spelling(resolver, &table.record_type)?);
                output.push(' ');
                output.push_str(spelling(resolver, &table.key_type)?);
                output.push('}');
                Ok(())
            })?;
            output.push(']');
        }
    }
    output.push_str("\n}\n");
    Ok(output)
}

fn write_role_entries<Resolver: EncodedNameResolver<VocabularyRoot> + ?Sized>(
    output: &mut String,
    entries: &[RoleEntry],
    resolver: &Resolver,
    source: &BootstrapSourceMetadata,
    _priors: &BootstrapPriorVocabulary,
) -> Result<(), BootstrapWriteError> {
    output.push_str("  [");
    write_separated(output, entries, |output, entry| match entry {
        RoleEntry::Declaration(declaration) => {
            write_type_declaration(output, declaration, resolver, source)
        }
        RoleEntry::Reference(reference) => {
            output.push_str(spelling(resolver, reference)?);
            Ok(())
        }
    })?;
    output.push(']');
    Ok(())
}

fn write_declaration<Resolver: EncodedNameResolver<VocabularyRoot> + ?Sized>(
    output: &mut String,
    declaration: &Declaration,
    resolver: &Resolver,
    source: &BootstrapSourceMetadata,
    priors: &BootstrapPriorVocabulary,
) -> Result<(), BootstrapWriteError> {
    match declaration {
        Declaration::Type(declaration) => {
            write_type_declaration(output, declaration, resolver, source)
        }
        Declaration::Stream(stream) => {
            output.push_str(spelling(resolver, &stream.output.name)?);
            output.push('.');
            output.push_str(spelling(resolver, &priors.identities().stream_nomos)?);
            output.push_str(".(");
            write_type_expression(output, &stream.initiation.query, resolver, source)?;
            output.push(' ');
            let event = stream.output.stream_of_event.arguments.first().ok_or(
                BootstrapWriteError::InvalidModel("Stream output Shape has no Event argument"),
            )?;
            write_type_expression(output, event, resolver, source)?;
            output.push(')');
            Ok(())
        }
    }
}

fn write_type_declaration<Resolver: EncodedNameResolver<VocabularyRoot> + ?Sized>(
    output: &mut String,
    declaration: &TypeDeclaration,
    resolver: &Resolver,
    source: &BootstrapSourceMetadata,
) -> Result<(), BootstrapWriteError> {
    output.push_str(spelling(resolver, &declaration.name)?);
    match &declaration.body {
        TypeBody::Newtype(expression) => {
            output.push('.');
            write_type_expression(output, expression, resolver, source)?;
        }
        TypeBody::Struct(fields) => {
            output.push_str(".{");
            write_separated(output, fields, |output, field| {
                write_type_expression(output, field, resolver, source)
            })?;
            output.push('}');
        }
        TypeBody::Enum(variants) => {
            output.push_str(".[");
            write_separated(output, variants, |output, variant| {
                output.push_str(spelling(resolver, &variant.name)?);
                match &variant.body {
                    VariantBody::Unit => Ok(()),
                    VariantBody::Unary(expression) => {
                        output.push('.');
                        write_type_expression(output, expression, resolver, source)
                    }
                    VariantBody::Product(fields) => {
                        output.push_str(".{");
                        write_separated(output, fields, |output, field| {
                            write_type_expression(output, field, resolver, source)
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

fn write_trait<Resolver: EncodedNameResolver<VocabularyRoot> + ?Sized>(
    output: &mut String,
    declaration: &TraitDeclaration,
    resolver: &Resolver,
    source: &BootstrapSourceMetadata,
) -> Result<(), BootstrapWriteError> {
    output.push_str(spelling(resolver, &declaration.name)?);
    output.push_str(".{");
    write_separated(output, &declaration.methods, |output, method| {
        output.push_str(spelling(resolver, &method.name)?);
        output.push_str(".{");
        for parameter in &method.parameters {
            write_type_expression(output, parameter, resolver, source)?;
            output.push(' ');
        }
        write_type_expression(output, &method.return_type, resolver, source)?;
        output.push('}');
        Ok(())
    })?;
    output.push('}');
    Ok(())
}

fn write_type_expression<Resolver: EncodedNameResolver<VocabularyRoot> + ?Sized>(
    output: &mut String,
    expression: &TypeExpression,
    resolver: &Resolver,
    source: &BootstrapSourceMetadata,
) -> Result<(), BootstrapWriteError> {
    match expression {
        TypeExpression::Reference(reference) => output.push_str(spelling(resolver, reference)?),
        TypeExpression::ShapeApplication(application) => {
            output.push_str(spelling(resolver, &application.shape)?);
            output.push('<');
            write_separated(output, &application.arguments, |output, argument| {
                write_type_expression(output, argument, resolver, source)
            })?;
            output.push('>');
        }
        TypeExpression::TraitRequirement(requirement) => {
            output.push('«');
            for (index, required) in requirement.required_traits.iter().enumerate() {
                if index > 0 {
                    output.push(' ');
                }
                if index == 0 {
                    if let Some(name) = source.named_parameters.get(&requirement.parameter) {
                        output.push_str(name);
                        output.push('.');
                    }
                }
                output.push_str(spelling(resolver, required)?);
            }
            output.push('»');
        }
    }
    Ok(())
}

fn spelling<'a, Resolver: EncodedNameResolver<VocabularyRoot> + ?Sized>(
    resolver: &'a Resolver,
    identity: &VocabularyEncodedId,
) -> Result<&'a str, BootstrapWriteError> {
    resolver
        .resolve(identity)
        .map(encoded_name_table::Name::as_str)
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
