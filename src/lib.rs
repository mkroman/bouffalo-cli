pub mod bl;
pub mod bl60x;
mod error;
pub mod isp;

pub use error::Error;

/// Provides static typing for a virtual address on the target platform.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct VirtAddr(u32);
