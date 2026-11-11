//! `kbox image-inspect` command — display detailed image metadata as JSON.

use clap::Args;

use crate::image_usage;

#[derive(Args)]
pub struct ImageInspectArgs {
    /// Image reference to inspect
    pub image: String,
}

pub async fn execute(args: ImageInspectArgs) -> Result<(), Box<dyn std::error::Error>> {
    let store = super::open_image_store()?;
    let images = store.list().await;
    let stored = resolve_inspect_image(&images, &args.image)?
        .ok_or_else(|| format!("Image not found: {}", args.image))?;
    println!("{}", build_image_inspect_json(&stored)?);
    Ok(())
}

/// Try to inspect `reference` as an image. Returns `Ok(None)` when no image
/// matches (so a polymorphic `inspect` can fall back to other object types).
pub(crate) async fn try_image_inspect_json(
    reference: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let store = super::open_image_store()?;
    let images = store.list().await;
    match resolve_inspect_image(&images, reference)? {
        Some(stored) => Ok(Some(build_image_inspect_json(&stored)?)),
        None => Ok(None),
    }
}

fn resolve_inspect_image(
    images: &[kbox_runtime::StoredImage],
    query: &str,
) -> Result<Option<kbox_runtime::StoredImage>, String> {
    let query = query.trim();
    if let Some((_, digest)) = query.rsplit_once('@') {
        kbox_runtime::ImageReference::parse(query)
            .map_err(|_| "invalid immutable image reference".to_owned())?;
        let hex = digest
            .strip_prefix("sha256:")
            .ok_or("immutable image reference requires SHA-256")?;
        if hex.len() != 64
            || !hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("immutable image reference requires a complete SHA-256 digest".into());
        }
        // Inspect content identity even when several mutable tags refer to the same bytes.
        return Ok(images
            .iter()
            .filter(|image| image.digest == digest)
            .min_by(|a, b| a.reference.cmp(&b.reference))
            .cloned());
    }
    image_usage::resolve_stored_image(images, query)
}

/// Build the JSON inspection document for a stored image.
fn build_image_inspect_json(
    stored: &kbox_runtime::StoredImage,
) -> Result<String, Box<dyn std::error::Error>> {
    // Load OCI image to get full config
    let oci = kbox_runtime::OciImage::from_path(&stored.path)?;
    let config = oci.config();

    let env_map: serde_json::Map<String, serde_json::Value> = config
        .env
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
        .collect();
    let healthcheck = config.health_check.as_ref().map(|hc| {
        serde_json::json!({
            "Test": hc.test.clone(),
            "Interval": hc.interval,
            "Timeout": hc.timeout,
            "Retries": hc.retries,
            "StartPeriod": hc.start_period,
        })
    });

    let output = serde_json::json!({
        "Reference": stored.reference,
        "Digest": stored.digest,
        "Size": stored.size_bytes,
        "PulledAt": stored.pulled_at.to_rfc3339(),
        "Config": {
            "Entrypoint": config.entrypoint,
            "Cmd": config.cmd,
            "Env": env_map,
            "WorkingDir": config.working_dir,
            "User": config.user,
            "ExposedPorts": config.exposed_ports,
            "Volumes": config.volumes,
            "StopSignal": config.stop_signal,
            "Healthcheck": healthcheck,
            "OnBuild": config.onbuild,
            "Labels": config.labels,
        },
        "LayerCount": oci.layer_paths().len(),
    });

    // Docker's inspect family always returns a top-level JSON array (even for a
    // single image), so tooling can do `inspect X | jq '.[0].Config'`.
    Ok(serde_json::to_string_pretty(&serde_json::json!([output]))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use kbox_runtime::StoredImage;
    use std::path::PathBuf;

    #[test]
    fn test_image_inspect_resolution_accepts_normalized_alias() {
        let images = vec![StoredImage {
            reference: "docker.io/library/alpine:latest".to_string(),
            digest: "sha256:abc".to_string(),
            size_bytes: 1024,
            pulled_at: Utc::now(),
            last_used: Utc::now(),
            path: PathBuf::from("/tmp/image"),
        }];

        let stored = image_usage::resolve_required_stored_image(&images, "alpine:latest").unwrap();

        assert_eq!(stored.reference, "docker.io/library/alpine:latest");
    }

    #[test]
    fn image_inspect_uses_complete_immutable_digests_without_changing_destructive_resolution() {
        let make = |reference: &str, hex: char| StoredImage {
            reference: reference.into(),
            digest: format!("sha256:{}", hex.to_string().repeat(64)),
            size_bytes: 1,
            pulled_at: Utc::now(),
            last_used: Utc::now(),
            path: PathBuf::from("/unused"),
        };
        let images = vec![
            make("docker.io/library/base:latest", '2'),
            make("docker.io/library/base:old", '1'),
            make("docker.io/library/base:alias", '1'),
        ];
        let pin = format!("base@sha256:{}", "1".repeat(64));
        let resolved = resolve_inspect_image(&images, &pin).unwrap().unwrap();
        assert_eq!(resolved.digest, format!("sha256:{}", "1".repeat(64)));
        assert_eq!(resolved.reference, "docker.io/library/base:alias");
        assert!(image_usage::resolve_stored_image(&images, &pin)
            .unwrap()
            .is_none());
        assert!(
            resolve_inspect_image(&images, &format!("base@sha256:{}", "3".repeat(64)))
                .unwrap()
                .is_none()
        );
        for invalid in [
            "base@sha256:1".to_owned(),
            format!("base@sha256:{}", "A".repeat(64)),
            format!("base@sha512:{}", "1".repeat(64)),
        ] {
            assert!(resolve_inspect_image(&images, &invalid).is_err());
        }
    }
}
