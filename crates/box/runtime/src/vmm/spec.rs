//! InstanceSpec re-exported from `kbox-core`.
//!
//! The canonical definition lives in `kbox_core::vmm`. Re-exported here
//! so existing callers using `crate::vmm::InstanceSpec` continue to work.

pub use kbox_core::vmm::{
    Entrypoint, FsMount, InstanceSpec, NetworkInstanceConfig, RawBlockDevice, RootfsSource,
    TeeInstanceConfig,
};
