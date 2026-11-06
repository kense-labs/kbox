//! `kbox update` command.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use clap::Args;
use serde::Deserialize;
use tokio::process::Command;

const RELEASES_API: &str = "https://api.github.com/repos/kense-labs/kense/releases?per_page=100";
const RELEASE_DOWNLOAD_BASE: &str = "https://github.com/kense-labs/kense/releases/download";
const RELEASE_TAG_PREFIX: &str = "kbox-v";
const MAX_RELEASES_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
const MAX_INSTALLER_BYTES: usize = 1024 * 1024;

#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Version to install, for example 3.3.0 (defaults to the latest stable release)
    #[arg(value_name = "VERSION", conflicts_with = "check")]
    version: Option<String>,

    /// Check for a newer release without installing it
    #[arg(long, conflicts_with = "force")]
    check: bool,

    /// Reinstall the selected version even if it is already running
    #[arg(long)]
    force: bool,

    /// Installation prefix (defaults to the current executable's prefix)
    #[arg(long, value_name = "PATH")]
    prefix: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Version([u64; 3]);

impl Version {
    fn parse(input: &str) -> Result<Self, String> {
        if input.is_empty() || input.len() > 64 {
            return Err(format!(
                "invalid kbox version {input:?}; expected MAJOR.MINOR.PATCH"
            ));
        }
        let parts: Vec<&str> = input.split('.').collect();
        if parts.len() != 3
            || parts
                .iter()
                .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(format!(
                "invalid kbox version {input:?}; expected MAJOR.MINOR.PATCH"
            ));
        }
        let mut numbers = [0_u64; 3];
        for (index, part) in parts.into_iter().enumerate() {
            numbers[index] = part.parse::<u64>().map_err(|_| {
                format!("invalid kbox version {input:?}; numeric components are too large")
            })?;
        }
        Ok(Self(numbers))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.0[0], self.0[1], self.0[2])
    }
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}

fn latest_stable_version(releases: impl IntoIterator<Item = Release>) -> Option<Version> {
    releases
        .into_iter()
        .filter(|release| !release.draft && !release.prerelease)
        .filter_map(|release| {
            Version::parse(release.tag_name.strip_prefix(RELEASE_TAG_PREFIX)?).ok()
        })
        .max()
}

fn prefix_from_executable(executable: &Path) -> Option<PathBuf> {
    let bin = executable.parent()?;
    if bin.file_name()? != "bin" {
        return None;
    }
    bin.parent().map(Path::to_path_buf)
}

fn install_prefix(requested: Option<PathBuf>) -> PathBuf {
    requested
        .or_else(|| {
            std::env::var_os("KBOX_PREFIX")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|executable| prefix_from_executable(&executable))
        })
        .unwrap_or_else(kbox_core::dirs_home)
}

fn installer_url(version: Version) -> String {
    format!("{RELEASE_DOWNLOAD_BASE}/{RELEASE_TAG_PREFIX}{version}/install.sh")
}

async fn latest_version() -> Result<Version, Box<dyn std::error::Error>> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--retry",
            "3",
            "--max-time",
            "60",
            "--max-filesize",
            "16777216",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
        ])
        .arg(format!("User-Agent: kbox-update/{}", kbox_core::VERSION))
        .arg(RELEASES_API)
        .output()
        .await
        .map_err(|error| format!("failed to run curl (is it installed?): {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "failed to query kbox releases: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    if output.stdout.len() > MAX_RELEASES_RESPONSE_BYTES {
        return Err("GitHub releases response exceeded the 16 MiB limit".into());
    }
    let releases: Vec<Release> = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("invalid GitHub releases response: {error}"))?;
    latest_stable_version(releases).ok_or_else(|| "no stable kbox release was found".into())
}

async fn download_installer(
    version: Version,
    destination: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--retry",
            "3",
            "--max-time",
            "60",
            "--max-filesize",
            "1048576",
            "-o",
        ])
        .arg(destination)
        .arg(installer_url(version))
        .output()
        .await
        .map_err(|error| format!("failed to run curl (is it installed?): {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "failed to download the kbox {version} installer: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    let metadata = std::fs::metadata(destination)
        .map_err(|error| format!("failed to inspect the downloaded installer: {error}"))?;
    if metadata.len() > MAX_INSTALLER_BYTES as u64 {
        return Err("downloaded installer exceeded the 1 MiB limit".into());
    }
    Ok(())
}

struct WorkDir(PathBuf);

impl WorkDir {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!("kbox-update-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
        Ok(Self(path))
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            eprintln!(
                "Warning: failed to remove temporary update directory {}: {error}",
                self.0.display()
            );
        }
    }
}

pub async fn execute(args: UpdateArgs) -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(windows) {
        return Err("kbox update currently supports macOS and Linux".into());
    }

    let current = Version::parse(kbox_core::VERSION)
        .map_err(|error| format!("invalid current kbox version: {error}"))?;
    let target = match args.version {
        Some(version) => Version::parse(&version)?,
        None => {
            println!("Checking for kbox updates...");
            latest_version().await?
        }
    };

    println!("Current version: {current}");
    println!("Selected version: {target}");
    if args.check {
        if target > current {
            println!("Update available: {current} -> {target}");
            println!("Run `kbox update` to install it.");
        } else {
            println!("kbox is up to date.");
        }
        return Ok(());
    }
    if target == current && !args.force {
        println!("kbox is already up to date.");
        return Ok(());
    }

    let prefix = install_prefix(args.prefix);
    let work = WorkDir::create()?;
    let installer = work.0.join("install.sh");
    download_installer(target, &installer).await?;

    println!("Installing kbox {target} to {}", prefix.display());
    let status = Command::new("sh")
        .arg(&installer)
        .env("KBOX_VERSION", target.to_string())
        .env("KBOX_PREFIX", &prefix)
        .stdin(Stdio::null())
        .status()
        .await
        .map_err(|error| format!("failed to run the kbox installer: {error}"))?;
    if !status.success() {
        return Err(format!("kbox installer exited with {status}").into());
    }

    let installed = Command::new(prefix.join("bin").join("kbox"))
        .arg("version")
        .output()
        .await
        .map_err(|error| format!("failed to verify the installed kbox: {error}"))?;
    let installed_version = String::from_utf8_lossy(&installed.stdout);
    if !installed.status.success() || installed_version.trim() != format!("kbox version {target}") {
        return Err(format!(
            "post-update version check failed: {}{}",
            installed_version.trim(),
            String::from_utf8_lossy(&installed.stderr)
        )
        .into());
    }

    println!("Successfully installed kbox {target}.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag_name: &str, draft: bool, prerelease: bool) -> Release {
        Release {
            tag_name: tag_name.to_string(),
            draft,
            prerelease,
        }
    }

    #[test]
    fn version_requires_three_numeric_components() {
        assert_eq!(Version::parse("3.3.0").unwrap(), Version([3, 3, 0]));
        for invalid in ["", "3", "3.3", "v3.3.0", "3.3.0;rm", "3.3.-1", "3.3.0.0"] {
            assert!(Version::parse(invalid).is_err(), "accepted {invalid:?}");
        }
    }

    #[test]
    fn latest_release_uses_numeric_order_and_stable_kbox_tags() {
        let releases = vec![
            release("kbox-v3.9.0", false, false),
            release("kbox-v3.10.0", false, false),
            release("kbox-v4.0.0", false, true),
            release("kbox-v5.0.0", true, false),
            release("other-v9.0.0", false, false),
        ];
        assert_eq!(latest_stable_version(releases), Some(Version([3, 10, 0])));
    }

    #[test]
    fn executable_prefix_requires_a_bin_directory() {
        assert_eq!(
            prefix_from_executable(Path::new("/opt/kbox/bin/kbox")),
            Some(PathBuf::from("/opt/kbox"))
        );
        assert_eq!(prefix_from_executable(Path::new("/opt/kbox/kbox")), None);
    }

    #[test]
    fn installer_url_contains_only_the_validated_release_version() {
        assert_eq!(
            installer_url(Version([3, 3, 0])),
            "https://github.com/kense-labs/kense/releases/download/kbox-v3.3.0/install.sh"
        );
    }
}
