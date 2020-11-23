use thiserror::Error;

/// The default entry point when the user doesn't provide one when using the `FirmwareBuilder`
const DEFAULT_ENTRY_POINT: u32 = 0x2100_0000;

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

#[derive(Debug)]
pub struct Firmware {
    /// The magic header - either 'BFNP' or 'BFAP'
    magic: [u8; 4],
    /// The boot header revision?
    revision: u32,
    /// The entry point of the written firmware image
    entry_point: u32,
}

#[derive(Debug)]
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
    writeVregEnableCmd: u8,

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
    burstWrapData: u8,

    // Disable burst wrap command */
    deBurstWrapCmd: u8,

    // Disable burst wrap command dummy clock */
    deBurstWrapCmdDmyClk: u8,

    // Data and address mode for this command */
    deBurstWrapDataMode: u8,

    // Data to disable burst wrap */
    deBurstWrapData: u8,

    // 4K erase time */
    timeEsector: u16,

    // 32K erase time */
    timeE32k: u16,

    // 64K erase time */
    timeE64k: u16,

    // Page program time */
    timePagePgm: u16,

    // Chip erase time in ms */
    timeCe: u16,

    // Release power down command delay time for wake up */
    pdDelay: u8,

    // QE set data */
    qeData: u8,
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
        if self.flash_config.is_none() {
            return Err(BuilderError::MissingFlashConfig);
        }

        Ok(Firmware {
            magic: *b"BFNP", // CPU 1
            revision: 1,
            entry_point,
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
