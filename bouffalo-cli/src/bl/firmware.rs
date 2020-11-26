use std::convert::TryInto;

use thiserror::Error;

/// The default entry point when the user doesn't provide one when using the `FirmwareBuilder`
const DEFAULT_ENTRY_POINT: u32 = 0x2100_0000;

/// The size of the flash config structure, excluding the magic header and the crc32
const FLASH_CONFIG_STRUCT_SIZE: usize = 84;

/// The size of the clock config structure, excluding the magic header and the crc32
const CLOCK_CONFIG_STRUCT_SIZE: usize = 8;

/// The size of the boot header structure, excluding the magic header and the crc32
const BOOT_HEADER_STRUCT_SIZE: usize = 164;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("no error")]
    None,
}

#[derive(Error, Debug)]
pub enum BuilderError {
    #[error("Missing flash_config value in FirmwareBuilder")]
    MissingFlashConfig,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Firmware {
    /// The magic header - either 'BFNP' or 'BFAP'
    magic: [u8; 4],
    /// The boot header revision?
    revision: u32,

    /// The flash configuration magic header
    flash_magic: [u8; 4],
    /// The flash configuration parameters
    flash_config: FlashConfig,
    /// The flash configuration crc32 checksum
    flash_crc32: u32,

    /// The clock configuration magic header
    clock_magic: [u8; 4],
    /// The clock configuration parameters
    clock_config: ClockConfig,
    /// The clock configuration crc32 checksum
    clock_crc32: u32,

    /// Boot configuration flags
    boot_config: u32,

    /// Image segment info
    image_segment_info: u32,

    /// The entry point of the written firmware image
    entry_point: u32,

    /// Image RAM addr or flash offset
    image_start: u32,

    /// SHA-256 hash of the whole image
    hash: [u8; 20],

    // "rsv1" and "rsv2" which are 4 bytes each
    _reserved: u64,

    /// The CRC32 checksum for the boot header
    crc32: u32,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Default, Clone)]
pub struct ClockConfig {
    /// PLL crystal type
    // TODO: Create enum type
    // https://github.com/bouffalolab/bl_iot_sdk/blob/ee4a10b1a1e3609243bd5e7b3a45f02d768f6c14/components/bl602/bl602_std/bl602_std/StdDriver/Inc/bl602_glb.h#L286-L297
    crystal_type: u8,
    /// The PLL output clock type
    // TODO: Create enum type
    // https://github.com/bouffalolab/bl_iot_sdk/blob/ee4a10b1a1e3609243bd5e7b3a45f02d768f6c14/components/bl602/bl602_std/bl602_std/StdDriver/Inc/bl602_glb.h#L299-L312
    pll_clock: u8,
    /// HCLK divider
    hclk_divider: u8,
    /// BCLK divider
    bclk_divider: u8,
    /// Flash clock type
    // TODO: Create enum type
    // https://github.com/bouffalolab/bl_iot_sdk/blob/ee4a10b1a1e3609243bd5e7b3a45f02d768f6c14/components/bl602/bl602_std/bl602_std/StdDriver/Inc/bl602_glb.h#L101-L111
    flash_clock_type: u8,
    /// Flash clock divider
    flash_clock_divider: u8,
    // Reserved field
    _reserved: u16,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Default, Clone)]
pub struct FlashConfig {
    // Serail flash interface mode,bit0-3:IF mode,bit4:unwrap */
    io_mode: u8,
    // Support continuous read mode,bit0:continuous read mode support,bit1:read mode cfg
    continuous_read_support: u8,
    // SPI clock delay,bit0-3:delay,bit4-6:pad delay
    clock_delay: u8,
    // SPI clock phase invert,bit0:clck invert,bit1:rx invert,bit2-4:pad delay,bit5-7:pad delay */
    clock_invert: u8,
    // Flash enable reset command */
    reset_enable_cmd: u8,
    // Flash reset command */
    reset_cmd: u8,
    // Flash reset continuous read command */
    reset_continuous_read_cmd: u8,
    // Flash reset continuous read command size */
    reset_continuous_read_cmd_size: u8,
    // JEDEC ID command */
    jedec_id_cmd: u8,
    // JEDEC ID command dummy clock */
    jedec_id_cmd_dummy_clock: u8,
    // QPI JEDEC ID comamnd */
    qpi_jedec_id_cmd: u8,
    // QPI JEDEC ID command dummy clock */
    qpi_jedec_id_cmd_dummy_clock: u8,
    // Sector size - 1024 bytes
    sector_size: u8,
    // Manufacturer ID
    manufacturer_id: u8,
    // Page size
    page_size: u16,
    // Chip erase command
    chip_erase_cmd: u8,
    // Sector erase command
    sector_erase_cmd: u8,
    // Block 32K erase command,some Micron not support */
    block_erase_32k_cmd: u8,
    // Block 64K erase command */
    block_erase_64k_cmd: u8,
    // Need before every erase or program */
    write_enable_cmd: u8,
    // Page program cmd */
    page_program_cmd: u8,
    // QIO page program cmd */
    qio_page_program_cmd: u8,
    // QIO page program address mode */
    qio_page_program_address_mode: u8,
    // Fast read command */
    fast_read_cmd: u8,
    // Fast read command dummy clock */
    fast_read_cmd_dummy_clock: u8,
    // QPI fast read command */
    qpi_fast_read_cmd: u8,
    // QPI fast read command dummy clock */
    qpi_fast_read_cmd_dummy_clock: u8,
    // Fast read dual output command */
    fast_read_dual_output_cmd: u8,
    // Fast read dual output command dummy clock */
    fast_read_dual_output_cmd_dummy_clock: u8,
    // Fast read dual io comamnd */
    fast_read_dual_io_cmd: u8,
    // Fast read dual io command dummy clock */
    fast_read_dual_io_cmd_dummy_clock: u8,
    // Fast read quad output comamnd */
    fast_read_quad_output_cmd: u8,
    // Fast read quad output comamnd dummy clock */
    fast_read_quad_output_cmd_dummy_clock: u8,
    // Fast read quad io comamnd */
    fast_read_quad_io_cmd: u8,
    // Fast read quad io comamnd dummy clock */
    fast_read_quad_io_cmd_dummy_clock: u8,
    // QPI fast read quad io comamnd */
    qpi_fast_read_quad_io_cmd: u8,
    // QPI fast read QIO dummy clock */
    qpi_fast_read_quad_io_cmd_dummy_clock: u8,
    // QPI program command */
    qpi_program_cmd: u8,
    // Enable write reg */
    // writeVregEnableCmd
    volatile_register_write_enable_cmd: u8,
    // Write enable register index */
    write_enable_reg_index: u8,
    // Quad mode enable register index */
    quad_mode_enable_reg_index: u8,
    // Busy status register index */
    busy_status_reg_index: u8,
    // Write enable bit pos */
    write_enable_bit_pos: u8,
    // Quad enable bit pos */
    quad_enable_bit_pos: u8,
    // Busy status bit pos */
    busy_status_bit_pos: u8,
    // Register length of write enable */
    write_enable_reg_write_len: u8,
    // Register length of write enable status */
    write_enable_reg_read_len: u8,
    // Register length of contain quad enable */
    quad_enable_reg_write_len: u8,
    // Register length of contain quad enable status */
    quad_enable_reg_read_len: u8,
    // Release power down command */
    release_power_down_cmd: u8,
    // Register length of contain busy status */
    busy_status_reg_read_len: u8,
    // Read register command buffer */
    read_reg_cmd_buffer: [u8; 4],
    // Write register command buffer */
    write_reg_cmd_buffer: [u8; 4],
    // Enter qpi command */
    enter_qpi_cmd: u8,
    // Exit qpi command */
    exit_qpi_cmd: u8,
    // Config data for continuous read mode */
    continuous_read_mode_cfg: u8,
    // Config data for exit continuous read mode */
    continuous_read_mode_exit_cfg: u8,
    // Enable burst wrap command */
    enable_burst_wrap_cmd: u8,
    // Enable burst wrap command dummy clock */
    enable_burst_wrap_cmd_dummy_clock: u8,
    // Data and address mode for this command */
    burst_wrap_data_mode: u8,
    // Data to enable burst wrap */
    burst_wrap_data: u8,
    // Disable burst wrap command */
    disable_burst_wrap_cmd: u8,
    // Disable burst wrap command dummy clock */
    disable_burst_wrap_cmd_dummy_clock: u8,
    // Data and address mode for this command */
    disable_burst_wrap_data_mode: u8,
    // Data to disable burst wrap */
    disable_burst_wrap_data: u8,
    // 4K erase time */
    sector_erase_time_4k: u16,
    // 32K erase time */
    sector_erase_time_32k: u16,
    // 64K erase time */
    sector_erase_time_64k: u16,
    // Page program time */
    page_program_time: u16,
    // Chip erase time in ms */
    chip_erase_time: u16,
    // Release power down command delay time for wake up */
    power_down_delay: u8,
    // QE set data */
    quad_enable_data: u8,
}

impl FlashConfig {
    pub fn from_slice<T: TryInto<[u8; FLASH_CONFIG_STRUCT_SIZE]>>(
        slice: T,
    ) -> Result<FlashConfig, T::Error> {
        let fixed_size_ary = slice.try_into()?;
        let config = unsafe {
            std::mem::transmute::<[u8; FLASH_CONFIG_STRUCT_SIZE], FlashConfig>(fixed_size_ary)
        };

        Ok(config)
    }
}

pub struct FirmwareBuilder {
    /// The entry point of the firmware image
    entry_point: Option<u32>,
    /// Flash configuration
    flash_config: Option<FlashConfig>,
}

impl FirmwareBuilder {
    /// Sets the firmwares entry point to `entry_point`
    pub fn entry_point(&mut self, entry_point: u32) -> &mut FirmwareBuilder {
        self.entry_point = Some(entry_point);
        self
    }

    /// Sets the flash config to `flash_config`
    pub fn flash_config(&mut self, flash_config: FlashConfig) -> &mut FirmwareBuilder {
        self.flash_config = Some(flash_config);
        self
    }

    /// Builds the final Firmware from this FirmwareBuilder
    ///
    /// Returns the Firmware instance on success, a BuilderError otherwise
    pub fn build(&self) -> Result<Firmware, BuilderError> {
        let entry_point = self.entry_point.unwrap_or(DEFAULT_ENTRY_POINT);

        // Assert that a flash configuration has been set
        let flash_config = match self.flash_config {
            Some(flash_config) => flash_config,
            None => return Err(BuilderError::MissingFlashConfig),
        };

        let clock_config = ClockConfig::default();
        let boot_config = 0;

        Ok(Firmware {
            magic: *b"BFNP", // CPU 1
            revision: 1,
            flash_magic: *b"FCFG",
            flash_config,
            flash_crc32: 0,
            clock_magic: *b"PCFG",
            clock_config,
            clock_crc32: 0,
            boot_config,
            image_segment_info: 0,
            entry_point,
            image_start: 0,
            hash: [0; 20],
            _reserved: 0,
            crc32: 0,
        })
    }
}

impl Default for FirmwareBuilder {
    fn default() -> FirmwareBuilder {
        FirmwareBuilder {
            entry_point: None,
            flash_config: None,
        }
    }
}

impl Firmware {
    pub fn builder() -> FirmwareBuilder {
        FirmwareBuilder::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_be_same_size_as_c_struct() {
        assert_eq!(std::mem::size_of::<FlashConfig>(), FLASH_CONFIG_STRUCT_SIZE);
        assert_eq!(std::mem::size_of::<ClockConfig>(), CLOCK_CONFIG_STRUCT_SIZE);
        assert_eq!(std::mem::size_of::<Firmware>(), BOOT_HEADER_STRUCT_SIZE);
    }

    #[test]
    fn it_should_deserialize_and_serialize_flash_config() {
        let flash_bin_slice = &crate::bl::EFLASH_LOADER_40M_BIN[0x0c..0x60];
        let flash_cfg = FlashConfig::from_slice(flash_bin_slice).unwrap();
        let flash_cfg_mem = unsafe {
            std::mem::transmute::<FlashConfig, [u8; FLASH_CONFIG_STRUCT_SIZE]>(flash_cfg)
        };

        assert_eq!(flash_cfg_mem, flash_bin_slice);
        println!("flash_cfg: {:#?}", flash_cfg);
    }
}
