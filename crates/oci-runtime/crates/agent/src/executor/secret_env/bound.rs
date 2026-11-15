//! Pin authorized sources before entering workload-controlled namespaces.
use super::*;
use crate::vm_attachment::MemorySecretEnvironmentSource;

#[derive(Debug)]
pub(in crate::executor) enum PreparedMemorySecretEnvironment {
    Legacy,
    Bound {
        manifest_index: Option<usize>,
        files: Vec<(String, File)>,
    },
}
impl PreparedMemorySecretEnvironment {
    pub(in crate::executor) fn prepare(
        environment: &[String],
        sources: &[MemorySecretEnvironmentSource],
    ) -> Result<Self> {
        if sources.is_empty() {
            return Ok(Self::Legacy);
        }
        let Some(requested) = requested_sources(environment, sources)? else {
            return Ok(Self::Bound {
                manifest_index: None,
                files: Vec::new(),
            });
        };
        let mut files = Vec::with_capacity(requested.len());
        for source in requested {
            let file = open_secret_file(source.protected_source())?;
            validate_bound_metadata(&file.metadata().map_err(|_| {
                secret_error(
                    ErrorCode::FailedPrecondition,
                    "could not inspect pinned Secret source",
                )
            })?)?;
            files.push((source.binding().variable().into(), file));
        }
        let index = environment
            .iter()
            .position(|entry| entry.starts_with("K_BOX_SECRET_ENV_V1="));
        Ok(Self::Bound {
            manifest_index: index,
            files,
        })
    }
    pub(in crate::executor) fn materialize(&self, environment: &mut Vec<String>) -> Result<()> {
        match self {
            Self::Legacy => super::materialize(environment),
            Self::Bound {
                manifest_index,
                files,
            } => {
                let mut additions = Vec::with_capacity(files.len());
                for (variable, file) in files {
                    let metadata = file.metadata().map_err(|_| {
                        secret_error(
                            ErrorCode::FailedPrecondition,
                            "could not recheck pinned Secret source",
                        )
                    })?;
                    validate_bound_metadata(&metadata)?;
                    additions.push(format!(
                        "{variable}={}",
                        read_secret_value(file, metadata.len())?
                    ));
                }
                if let Some(index) = manifest_index {
                    environment.remove(*index);
                }
                environment.extend(additions);
                Ok(())
            }
        }
    }
}
/// Validate requested metadata before any process snapshot or source I/O.
pub(in crate::executor) fn requested_sources<'a>(
    environment: &[String],
    sources: &'a [MemorySecretEnvironmentSource],
) -> Result<Option<Vec<&'a MemorySecretEnvironmentSource>>> {
    if sources.is_empty() {
        return Ok(None);
    }
    let Some(bindings) = koci_sdk::MemorySecretEnvironmentBinding::from_environment(environment)?
    else {
        return Ok(None);
    };
    let mut requested = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let mut matches = sources.iter().filter(|source| source.binding() == &binding);
        let source = matches.next().ok_or_else(|| {
            secret_error(
                ErrorCode::PermissionDenied,
                "Secret environment binding is outside the exact authorized memory files",
            )
        })?;
        if matches.next().is_some() {
            return Err(secret_error(
                ErrorCode::FailedPrecondition,
                "Secret environment source authority is ambiguous",
            ));
        }
        source.binding().validate()?;
        requested.push(source);
    }
    Ok(Some(requested))
}

fn validate_bound_metadata(metadata: &std::fs::Metadata) -> Result<()> {
    if metadata.file_type().is_file()
        && matches!(metadata.nlink(), 1 | 2)
        && metadata.len() > 0
        && metadata.len() <= MAX_SECRET_BYTES
        && metadata.permissions().mode() & 0o777 == 0o400
    {
        Ok(())
    } else {
        Err(secret_error(ErrorCode::FailedPrecondition,"pinned memory Secret source violates its regular-file, size, mode or exact-alias contract"))
    }
}

#[cfg(test)]
#[path = "bound_tests.rs"]
mod tests;
