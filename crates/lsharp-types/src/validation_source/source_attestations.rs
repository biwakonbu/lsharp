use super::{SourceGraphError, SourceReviewAttestation, review_attestation_from_form};
use lsharp_syntax::ast::{Decl, Metadata};
use lsharp_syntax::metadata::MetadataFormKind;

pub(super) fn add_decl_attestations(
    decl: &Decl,
    attestations: &mut Vec<SourceReviewAttestation>,
) -> Result<(), SourceGraphError> {
    match decl {
        Decl::Defn {
            metadata: Some(metadata),
            ..
        }
        | Decl::TypeDef {
            metadata: Some(metadata),
            ..
        }
        | Decl::RecordDef {
            metadata: Some(metadata),
            ..
        } => add_metadata_attestations(metadata, attestations),
        Decl::ModuleDecl { body, .. } | Decl::ImplDef { methods: body, .. } => {
            for nested in body {
                add_decl_attestations(nested, attestations)?;
            }
            Ok(())
        }
        Decl::Private { inner, .. } => add_decl_attestations(inner, attestations),
        _ => Ok(()),
    }
}

fn add_metadata_attestations(
    metadata: &Metadata,
    attestations: &mut Vec<SourceReviewAttestation>,
) -> Result<(), SourceGraphError> {
    for form in &metadata.forms {
        let MetadataFormKind::ReviewAttestation { attestation } = &form.kind else {
            continue;
        };
        attestations.push(review_attestation_from_form(attestation, form.span())?);
    }
    Ok(())
}
