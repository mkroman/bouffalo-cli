//! Bouffalo Lab firmware module

mod firmware;

const EFLASH_LOADER_40M_BIN: &[u8] = include_bytes!("../blobs/eflash_loader_40m.bin");

pub use firmware::{Firmware, FirmwareBuilder};
