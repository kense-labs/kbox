//! Validated Docker-style limits for Linux Guest process launch.
use std::{collections::BTreeSet, io};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuestRlimitResource {
    Cpu,
    Fsize,
    Data,
    Stack,
    Core,
    Rss,
    Nproc,
    Nofile,
    Memlock,
    Locks,
    Sigpending,
    Msgqueue,
    Nice,
    Rtprio,
    Rttime,
}

impl GuestRlimitResource {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "cpu" => Self::Cpu,
            "fsize" => Self::Fsize,
            "data" => Self::Data,
            "stack" => Self::Stack,
            "core" => Self::Core,
            "rss" => Self::Rss,
            "nproc" => Self::Nproc,
            "nofile" => Self::Nofile,
            "memlock" => Self::Memlock,
            "locks" => Self::Locks,
            "sigpending" => Self::Sigpending,
            "msgqueue" => Self::Msgqueue,
            "nice" => Self::Nice,
            "rtprio" => Self::Rtprio,
            "rttime" => Self::Rttime,
            _ => return None,
        })
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Fsize => "fsize",
            Self::Data => "data",
            Self::Stack => "stack",
            Self::Core => "core",
            Self::Rss => "rss",
            Self::Nproc => "nproc",
            Self::Nofile => "nofile",
            Self::Memlock => "memlock",
            Self::Locks => "locks",
            Self::Sigpending => "sigpending",
            Self::Msgqueue => "msgqueue",
            Self::Nice => "nice",
            Self::Rtprio => "rtprio",
            Self::Rttime => "rttime",
        }
    }

    /// Linux Guest IDs, independent of the host OS's libc numbering.
    pub const fn linux_id(self) -> u32 {
        match self {
            Self::Cpu => 0,
            Self::Fsize => 1,
            Self::Data => 2,
            Self::Stack => 3,
            Self::Core => 4,
            Self::Rss => 5,
            Self::Nproc => 6,
            Self::Nofile => 7,
            Self::Memlock => 8,
            Self::Locks => 10,
            Self::Sigpending => 11,
            Self::Msgqueue => 12,
            Self::Nice => 13,
            Self::Rtprio => 14,
            Self::Rttime => 15,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestRlimit {
    resource: GuestRlimitResource,
    soft: u64,
    hard: u64,
}

impl GuestRlimit {
    pub fn parse(value: &str) -> io::Result<Self> {
        let invalid = || {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid Guest ulimit; expected known RESOURCE=SOFT:HARD with soft <= hard",
            )
        };
        if value.len() > 256 || value.contains('\0') {
            return Err(invalid());
        }
        let (name, values) = value.split_once('=').ok_or_else(invalid)?;
        let resource = GuestRlimitResource::parse(name).ok_or_else(invalid)?;
        let (soft, hard) = values.split_once(':').ok_or_else(invalid)?;
        let number = |value: &str| {
            if value == "-1" {
                Some(u64::MAX)
            } else if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
                value.parse().ok()
            } else {
                None
            }
        };
        let soft = number(soft).ok_or_else(invalid)?;
        let hard = number(hard).ok_or_else(invalid)?;
        if soft > hard {
            return Err(invalid());
        }
        Ok(Self {
            resource,
            soft,
            hard,
        })
    }

    pub fn parse_specs(values: &[String]) -> io::Result<Vec<Self>> {
        if values.len() > 15 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "too many Guest ulimits",
            ));
        }
        let mut seen = BTreeSet::new();
        let mut limits = Vec::with_capacity(values.len());
        for value in values {
            let limit = Self::parse(value)?;
            if !seen.insert(limit.resource) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "duplicate Guest ulimit resource",
                ));
            }
            limits.push(limit);
        }
        Ok(limits)
    }

    pub fn resource(self) -> GuestRlimitResource {
        self.resource
    }
    pub fn soft(self) -> u64 {
        self.soft
    }
    pub fn hard(self) -> u64 {
        self.hard
    }
    pub fn guest_spec(self) -> String {
        format!("{}={}:{}", self.resource.linux_id(), self.soft, self.hard)
    }
}

#[cfg(test)]
#[path = "guest_rlimit_tests.rs"]
mod tests;
