//! InstanceSpec re-exported from `krun-box-core`.
//!
//! The canonical definition lives in `krun_box_core::vmm`. Re-exported here
//! so existing callers using `crate::vmm::InstanceSpec` continue to work.

pub use krun_box_core::vmm::{
    Entrypoint, FsMount, InstanceSpec, NetworkInstanceConfig, RawBlockDevice, RootfsSource,
    TeeInstanceConfig,
};
