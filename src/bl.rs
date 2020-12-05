//! Bouffalo Lab firmware module

pub mod bootrom;
mod firmware;

pub const EFLASH_LOADER_24M_BIN: &[u8] = include_bytes!("../blobs/eflash_loader_24m.bin");
pub const EFLASH_LOADER_26M_BIN: &[u8] = include_bytes!("../blobs/eflash_loader_26m.bin");
pub const EFLASH_LOADER_32M_BIN: &[u8] = include_bytes!("../blobs/eflash_loader_32m.bin");
pub const EFLASH_LOADER_38P4M_BIN: &[u8] = include_bytes!("../blobs/eflash_loader_38p4m.bin");
pub const EFLASH_LOADER_40M_BIN: &[u8] = include_bytes!("../blobs/eflash_loader_40m.bin");
pub const EFLASH_LOADER_NONE_BIN: &[u8] = include_bytes!("../blobs/eflash_loader_none.bin");
pub const EFLASH_LOADER_RC32M_BIN: &[u8] = include_bytes!("../blobs/eflash_loader_rc32m.bin");

pub use firmware::{crc32, Firmware, FirmwareBuilder, Segment};
