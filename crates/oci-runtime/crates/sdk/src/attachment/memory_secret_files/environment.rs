//! Metadata-only bindings for the explicit memory-file environment contract.
use super::super::invalid_attachment;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const MANIFEST: &str = "K_BOX_SECRET_ENV_V1";

/// A non-sensitive variable and exact reserved Guest file path; never a value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemorySecretEnvironmentBinding {
    variable: String,
    path: String,
}
impl MemorySecretEnvironmentBinding {
    /// Authorized environment variable name.
    pub fn variable(&self) -> &str {
        &self.variable
    }
    /// Exact mounted Guest file path.
    pub fn path(&self) -> &str {
        &self.path
    }
    /// Check the metadata-only variable and reserved canonical file identity.
    pub fn validate(&self) -> Result<()> {
        let mut bytes = self.variable.bytes();
        let valid_name = bytes
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
            && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
            && self.variable.len() <= 255
            && self.variable != MANIFEST;
        let valid_path = self
            .path
            .strip_prefix("/.kbox-secrets/")
            .and_then(|relative| relative.split_once('/'))
            .is_some_and(|(digest, file)| {
                digest.len() == 64
                    && digest.bytes().all(|b| b.is_ascii_hexdigit())
                    && file.len() == 10
                    && file.as_bytes()[..3].iter().all(u8::is_ascii_digit)
                    && &file.as_bytes()[3..] == b".secret"
                    && file[..3].parse::<usize>().is_ok_and(|index| index < 128)
            });
        if !valid_name || !valid_path {
            return Err(invalid_attachment(
                "Secret environment binding name or file identity is invalid",
            ));
        }
        Ok(())
    }
    /// Parse bounded metadata without reading material or changing the environment.
    pub fn from_environment(environment: &[String]) -> Result<Option<Vec<Self>>> {
        if environment.iter().any(|entry| entry == MANIFEST) {
            return Err(invalid_attachment(
                "Secret environment manifest is missing its value",
            ));
        }
        let manifests: Vec<_> = environment
            .iter()
            .filter_map(|entry| {
                entry
                    .split_once('=')
                    .filter(|(key, _)| *key == MANIFEST)
                    .map(|(_, value)| value)
            })
            .collect();
        if manifests.is_empty() {
            return Ok(None);
        }
        if manifests.len() != 1 || manifests[0].len() > 64 * 1024 {
            return Err(invalid_attachment(
                "Secret environment manifest is duplicate or exceeds its byte bound",
            ));
        }
        let bindings: Vec<Self> = serde_json::from_str(manifests[0]).map_err(|_| {
            invalid_attachment("Secret environment manifest is not valid binding JSON")
        })?;
        if bindings.is_empty() || bindings.len() > 128 {
            return Err(invalid_attachment(
                "Secret environment manifest has an invalid binding count",
            ));
        }
        let mut variables: BTreeSet<_> = environment
            .iter()
            .filter_map(|entry| entry.split_once('=').map(|(key, _)| key))
            .filter(|key| *key != MANIFEST)
            .collect();
        let mut paths = BTreeSet::new();
        for binding in &bindings {
            binding.validate()?;
            if !variables.insert(binding.variable.as_str()) || !paths.insert(binding.path.as_str())
            {
                return Err(invalid_attachment("Secret environment binding is invalid, duplicate or conflicts with an existing value"));
            }
        }
        Ok(Some(bindings))
    }
}
