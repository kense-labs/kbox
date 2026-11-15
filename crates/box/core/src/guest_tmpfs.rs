//! Shared tmpfs intent; kernel data stays distinct from generic mount flags.

use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TmpfsMountFlag {
    ReadOnly,
    ReadWrite,
    NoSuid,
    Suid,
    NoDev,
    Dev,
    NoExec,
    Exec,
}

impl TmpfsMountFlag {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "ro" => Self::ReadOnly,
            "rw" => Self::ReadWrite,
            "nosuid" => Self::NoSuid,
            "suid" => Self::Suid,
            "nodev" => Self::NoDev,
            "dev" => Self::Dev,
            "noexec" => Self::NoExec,
            "exec" => Self::Exec,
            _ => return None,
        })
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "ro",
            Self::ReadWrite => "rw",
            Self::NoSuid => "nosuid",
            Self::Suid => "suid",
            Self::NoDev => "nodev",
            Self::Dev => "dev",
            Self::NoExec => "noexec",
            Self::Exec => "exec",
        }
    }

    fn group(self) -> usize {
        match self {
            Self::ReadOnly | Self::ReadWrite => 0,
            Self::NoSuid | Self::Suid => 1,
            Self::NoDev | Self::Dev => 2,
            Self::NoExec | Self::Exec => 3,
        }
    }
}

#[derive(Debug)]
pub struct GuestTmpfsMount<'a> {
    path: &'a str,
    options: Vec<&'a str>,
}

impl<'a> GuestTmpfsMount<'a> {
    pub fn parse(value: &'a str) -> io::Result<Self> {
        let invalid = |reason| io::Error::new(io::ErrorKind::InvalidInput, reason);
        if value.len() > 8192 || value.contains('\0') {
            return Err(invalid("tmpfs mount exceeds bounds or contains NUL"));
        }
        let (path, options) = value.split_once(':').unwrap_or((value, ""));
        if path.len() > 4096 || !path.starts_with('/') || path.split('/').any(|part| part == "..") {
            return Err(invalid(
                "tmpfs mount requires an absolute non-escaping path",
            ));
        }
        let normalized = path
            .split('/')
            .filter(|part| !part.is_empty() && *part != ".")
            .collect::<Vec<_>>();
        if normalized.is_empty()
            || normalized == ["run"]
            || normalized.starts_with(&["run", "kbox"])
        {
            return Err(invalid("tmpfs mount overlaps reserved runtime state"));
        }
        let options: Vec<_> = options.split(',').filter(|part| !part.is_empty()).collect();
        if options.len() > 32 {
            return Err(invalid("tmpfs mount exceeds 32 options"));
        }
        let mut groups = [false; 4];
        for flag in options
            .iter()
            .filter_map(|option| TmpfsMountFlag::parse(option))
        {
            if std::mem::replace(&mut groups[flag.group()], true) {
                return Err(invalid("tmpfs mount has duplicate or conflicting flags"));
            }
        }
        Ok(Self { path, options })
    }

    pub fn path(&self) -> &'a str {
        self.path
    }

    pub fn options(&self) -> impl Iterator<Item = &str> {
        self.options.iter().copied()
    }

    pub fn flags(&self) -> impl Iterator<Item = TmpfsMountFlag> + '_ {
        self.options().filter_map(TmpfsMountFlag::parse)
    }

    pub fn read_only(&self) -> bool {
        self.flags().any(|flag| flag == TmpfsMountFlag::ReadOnly)
    }

    /// Preserve advanced kernel data used by the legacy BoxVm consumer.
    pub fn data_options(&self) -> Option<String> {
        let data: Vec<_> = self
            .options()
            .filter(|option| TmpfsMountFlag::parse(option).is_none())
            .collect();
        (!data.is_empty()).then(|| data.join(","))
    }
}

#[cfg(test)]
#[path = "guest_tmpfs_tests.rs"]
mod tests;
