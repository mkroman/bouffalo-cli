use std::io::{self, Seek, SeekFrom, Write};

use byteorder::{LittleEndian, ReadBytesExt};
use thiserror::Error;

/// The default entry point when the user doesn't provide one when using the `FirmwareBuilder`
const DEFAULT_ENTRY_POINT: u32 = 0x2100_0000;

/// Calculates the crc32 checksum for the given slice of `bytes`
///
/// The crc32 is implemented with the polynomial 0xEDB88320 and the initial value of 0xFFFFFFFF
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;

    for byte in bytes {
        crc ^= *byte as u32;

        for _ in 0..8 {
            if crc & 1 > 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }

    !crc
}

/// Clock config validation errors
#[derive(Error, Debug)]
pub enum ClockConfigError {
    #[error("The magic header value is invalid: {:?}", _0)]
    InvalidMagicHeader([u8; 4]),
}

/// Boot header validation errors
#[derive(Error, Debug)]
pub enum BootHeaderError {
    #[error("The magic header value is invalid: {:?}", _0)]
    InvalidMagicHeader([u8; 4]),
}

/// Flash config validation errors
#[derive(Error, Debug)]
pub enum FlashConfigError {
    #[error("The magic header value is invalid: {:?}", _0)]
    InvalidMagicHeader([u8; 4]),
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Boot header error: {}", _0)]
    BootHeaderError(#[from] BootHeaderError),

    #[error("Flash config error: {}", _0)]
    FlashConfigError(#[from] FlashConfigError),

    #[error("Clock config error: {}", _0)]
    ClockConfigError(#[from] ClockConfigError),

    #[error("I/O error: {}", _0)]
    IoError(#[from] io::Error),
}

#[derive(Error, Debug)]
pub enum BuilderError {
    #[error("Missing flash_config value in FirmwareBuilder")]
    MissingFlashConfig,
}

/// Indicates which CPU the firmware is for
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Cpu {
    Cpu0,
    Cpu1,
}

impl Cpu {
    /// Converts the CPU to a magic header value as little endian bytes
    pub fn to_magic_bytes(self) -> [u8; 4] {
        match self {
            Cpu::Cpu0 => *b"BFNP",
            Cpu::Cpu1 => *b"BFAP",
        }
    }
}

impl Default for Cpu {
    fn default() -> Cpu {
        Cpu::Cpu0
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct Firmware {
    cpu: Cpu,
    /// The boot header revision?
    revision: u32,
    /// The flash configuration magic header
    flash_config: FlashConfig,

    /// The clock configuration parameters
    clock_config: ClockConfig,

    /// Boot configuration flags
    boot_config: u32,

    /// Image segment info
    image_segment_info: u32,

    /// The entry point of the written firmware image
    entry_point: u32,

    /// Image RAM addr or flash offset
    image_start: u32,

    /// SHA-256 hash of the whole image
    hash: [u8; 32],

    /// The CRC32 checksum for the boot header
    crc32: u32,
}

#[derive(Debug, Copy, Default, Clone, Eq, PartialEq)]
pub struct ClockConfig {
    /// PLL crystal type
    // TODO: Create enum type
    // https://github.com/bouffalolab/bl_iot_sdk/blob/ee4a10b1a1e3609243bd5e7b3a45f02d768f6c14/components/bl602/bl602_std/bl602_std/StdDriver/Inc/bl602_glb.h#L286-L297
    xtal_type: u8,
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
    /// CRC32 checksum
    crc32: u32,
}

#[derive(Debug, Copy, Default, Clone, Eq, PartialEq)]
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
    /// CRC32 checksum
    crc32: u32,
}

impl Firmware {
    pub fn from_reader<R: ReadBytesExt + Seek>(mut reader: R) -> Result<Self, ParseError> {
        let mut magic = [0u8; 4];

        // Read the magic header
        reader.read_exact(&mut magic)?;

        // Determine which CPU this firmware is for
        let cpu = match &magic {
            b"BFNP" => Cpu::Cpu0,
            b"BFAP" => Cpu::Cpu1,
            _ => {
                return Err(ParseError::BootHeaderError(
                    BootHeaderError::InvalidMagicHeader(magic),
                ))
            }
        };

        // Read the boot header revision
        let revision = reader.read_u32::<LittleEndian>()?;

        // Skip the flash config
        let flash_config = FlashConfig::from_reader(&mut reader)?;

        // Read the flash config
        let clock_config = ClockConfig::from_reader(&mut reader)?;

        // Read the boot flags
        let boot_config = reader.read_u32::<LittleEndian>()?;

        // Read the image segment info
        let image_segment_info = reader.read_u32::<LittleEndian>()?;

        // Read the entry point
        let entry_point = reader.read_u32::<LittleEndian>()?;

        // Read the image start offset
        let image_start = reader.read_u32::<LittleEndian>()?;

        // Read the image hash
        let mut hash = [0u8; 32];
        reader.read_exact(&mut hash)?;

        // Skip the 8 reserved, unused bytes
        reader.seek(SeekFrom::Current(8))?;

        // Read the crc32 checksum
        let crc32 = reader.read_u32::<LittleEndian>()?;

        Ok(Firmware {
            cpu,
            revision,
            flash_config,
            clock_config,
            boot_config,
            image_segment_info,
            entry_point,
            image_start,
            hash,
            crc32,
        })
    }
}

impl FlashConfig {
    /// Reads and parses the flash config from existing firmware by using `reader`, returning
    /// `FlashConfig` on success, `ParseError` otherwise
    pub fn from_reader<R: ReadBytesExt + Seek>(reader: &mut R) -> Result<Self, ParseError> {
        let mut magic = [0u8; 4];

        // Read the magic header
        reader.read_exact(&mut magic)?;

        // Assert that the magic header is correct
        if &magic != b"FCFG" {
            return Err(ParseError::FlashConfigError(
                FlashConfigError::InvalidMagicHeader(magic),
            ));
        }

        // Read the I/O mode
        let io_mode = reader.read_u8()?;

        // Read continuous_read_support
        let continuous_read_support = reader.read_u8()?;

        // Read clock_delay
        let clock_delay = reader.read_u8()?;

        // Read clock_invert
        let clock_invert = reader.read_u8()?;

        // Read reset_enable_cmd
        let reset_enable_cmd = reader.read_u8()?;

        // Read reset_cmd
        let reset_cmd = reader.read_u8()?;

        // Read reset_continuous_read_cmd
        let reset_continuous_read_cmd = reader.read_u8()?;

        // Read reset_continuous_read_cmd_size
        let reset_continuous_read_cmd_size = reader.read_u8()?;

        // Read jedec_id_cmd
        let jedec_id_cmd = reader.read_u8()?;

        // Read jedec_id_cmd_dummy_clock
        let jedec_id_cmd_dummy_clock = reader.read_u8()?;

        // Read qpi_jedec_id_cmd
        let qpi_jedec_id_cmd = reader.read_u8()?;

        // Read qpi_jedec_id_cmd_dummy_clock
        let qpi_jedec_id_cmd_dummy_clock = reader.read_u8()?;

        // Read sector_size
        let sector_size = reader.read_u8()?;

        // Read manufacturer_id
        let manufacturer_id = reader.read_u8()?;

        // Read page_size
        let page_size = reader.read_u16::<LittleEndian>()?;

        // Read chip_erase_cmd
        let chip_erase_cmd = reader.read_u8()?;

        // Read sector_erase_cmd
        let sector_erase_cmd = reader.read_u8()?;

        // Read block_erase_32k_cmd
        let block_erase_32k_cmd = reader.read_u8()?;

        // Read block_erase_64k_cmd
        let block_erase_64k_cmd = reader.read_u8()?;

        // Read write_enable_cmd
        let write_enable_cmd = reader.read_u8()?;

        // Read page_program_cmd
        let page_program_cmd = reader.read_u8()?;

        // Read qio_page_program_cmd
        let qio_page_program_cmd = reader.read_u8()?;

        // Read qio_page_program_address_mode
        let qio_page_program_address_mode = reader.read_u8()?;

        // Read fast_read_cmd
        let fast_read_cmd = reader.read_u8()?;

        // Read fast_read_cmd_dummy_clock
        let fast_read_cmd_dummy_clock = reader.read_u8()?;

        // Read qpi_fast_read_cmd
        let qpi_fast_read_cmd = reader.read_u8()?;

        // Read qpi_fast_read_cmd_dummy_clock
        let qpi_fast_read_cmd_dummy_clock = reader.read_u8()?;

        // Read fast_read_dual_output_cmd
        let fast_read_dual_output_cmd = reader.read_u8()?;

        // Read fast_read_dual_output_cmd_dummy_clock
        let fast_read_dual_output_cmd_dummy_clock = reader.read_u8()?;

        // Read fast_read_dual_io_cmd
        let fast_read_dual_io_cmd = reader.read_u8()?;

        // Read fast_read_dual_io_cmd_dummy_clock
        let fast_read_dual_io_cmd_dummy_clock = reader.read_u8()?;

        // Read fast_read_quad_output_cmd
        let fast_read_quad_output_cmd = reader.read_u8()?;

        // Read fast_read_quad_output_cmd_dummy_clock
        let fast_read_quad_output_cmd_dummy_clock = reader.read_u8()?;

        // Read fast_read_quad_io_cmd
        let fast_read_quad_io_cmd = reader.read_u8()?;

        // Read fast_read_quad_io_cmd_dummy_clock
        let fast_read_quad_io_cmd_dummy_clock = reader.read_u8()?;

        // Read qpi_fast_read_quad_io_cmd
        let qpi_fast_read_quad_io_cmd = reader.read_u8()?;

        // Read qpi_fast_read_quad_io_cmd_dummy_clock
        let qpi_fast_read_quad_io_cmd_dummy_clock = reader.read_u8()?;

        // Read qpi_program_cmd
        let qpi_program_cmd = reader.read_u8()?;

        // Read volatile_register_write_enable_cmd
        let volatile_register_write_enable_cmd = reader.read_u8()?;

        // Read write_enable_reg_index
        let write_enable_reg_index = reader.read_u8()?;

        // Read quad_mode_enable_reg_index
        let quad_mode_enable_reg_index = reader.read_u8()?;

        // Read busy_status_reg_index
        let busy_status_reg_index = reader.read_u8()?;

        // Read write_enable_bit_pos
        let write_enable_bit_pos = reader.read_u8()?;

        // Read quad_enable_bit_pos
        let quad_enable_bit_pos = reader.read_u8()?;

        // Read busy_status_bit_pos
        let busy_status_bit_pos = reader.read_u8()?;

        // Read write_enable_reg_write_len
        let write_enable_reg_write_len = reader.read_u8()?;

        // Read write_enable_reg_read_len
        let write_enable_reg_read_len = reader.read_u8()?;

        // Read quad_enable_reg_write_len
        let quad_enable_reg_write_len = reader.read_u8()?;

        // Read quad_enable_reg_read_len
        let quad_enable_reg_read_len = reader.read_u8()?;

        // Read release_power_down_cmd
        let release_power_down_cmd = reader.read_u8()?;

        // Read busy_status_reg_read_len
        let busy_status_reg_read_len = reader.read_u8()?;

        // Read read_reg_cmd_buffer
        let mut read_reg_cmd_buffer = [0u8; 4];
        reader.read_exact(&mut read_reg_cmd_buffer)?;

        // Read write_reg_cmd_buffer
        let mut write_reg_cmd_buffer = [0u8; 4];
        reader.read_exact(&mut write_reg_cmd_buffer)?;

        // Read enter_qpi_cmd
        let enter_qpi_cmd = reader.read_u8()?;

        // Read exit_qpi_cmd
        let exit_qpi_cmd = reader.read_u8()?;

        // Read continuous_read_mode_cfg
        let continuous_read_mode_cfg = reader.read_u8()?;

        // Read continuous_read_mode_exit_cfg
        let continuous_read_mode_exit_cfg = reader.read_u8()?;

        // Read enable_burst_wrap_cmd
        let enable_burst_wrap_cmd = reader.read_u8()?;

        // Read enable_burst_wrap_cmd_dummy_clock
        let enable_burst_wrap_cmd_dummy_clock = reader.read_u8()?;

        // Read burst_wrap_data_mode
        let burst_wrap_data_mode = reader.read_u8()?;

        // Read burst_wrap_data
        let burst_wrap_data = reader.read_u8()?;

        // Read disable_burst_wrap_cmd
        let disable_burst_wrap_cmd = reader.read_u8()?;

        // Read disable_burst_wrap_cmd_dummy_clock
        let disable_burst_wrap_cmd_dummy_clock = reader.read_u8()?;

        // Read disable_burst_wrap_data_mode
        let disable_burst_wrap_data_mode = reader.read_u8()?;

        // Read disable_burst_wrap_data
        let disable_burst_wrap_data = reader.read_u8()?;

        // Read sector_erase_time_4k
        let sector_erase_time_4k = reader.read_u16::<LittleEndian>()?;

        // Read sector_erase_time_32k
        let sector_erase_time_32k = reader.read_u16::<LittleEndian>()?;

        // Read sector_erase_time_64k
        let sector_erase_time_64k = reader.read_u16::<LittleEndian>()?;

        // Read page_program_time
        let page_program_time = reader.read_u16::<LittleEndian>()?;

        // Read chip_erase_time
        let chip_erase_time = reader.read_u16::<LittleEndian>()?;

        // Read power_down_delay
        let power_down_delay = reader.read_u8()?;

        // Read quad_enable_data
        let quad_enable_data = reader.read_u8()?;

        // Read the crc32 checksum
        let crc32 = reader.read_u32::<LittleEndian>()?;

        Ok(FlashConfig {
            io_mode,
            continuous_read_support,
            clock_delay,
            clock_invert,
            reset_enable_cmd,
            reset_cmd,
            reset_continuous_read_cmd,
            reset_continuous_read_cmd_size,
            jedec_id_cmd,
            jedec_id_cmd_dummy_clock,
            qpi_jedec_id_cmd,
            qpi_jedec_id_cmd_dummy_clock,
            sector_size,
            manufacturer_id,
            page_size,
            chip_erase_cmd,
            sector_erase_cmd,
            block_erase_32k_cmd,
            block_erase_64k_cmd,
            write_enable_cmd,
            page_program_cmd,
            qio_page_program_cmd,
            qio_page_program_address_mode,
            fast_read_cmd,
            fast_read_cmd_dummy_clock,
            qpi_fast_read_cmd,
            qpi_fast_read_cmd_dummy_clock,
            fast_read_dual_output_cmd,
            fast_read_dual_output_cmd_dummy_clock,
            fast_read_dual_io_cmd,
            fast_read_dual_io_cmd_dummy_clock,
            fast_read_quad_output_cmd,
            fast_read_quad_output_cmd_dummy_clock,
            fast_read_quad_io_cmd,
            fast_read_quad_io_cmd_dummy_clock,
            qpi_fast_read_quad_io_cmd,
            qpi_fast_read_quad_io_cmd_dummy_clock,
            qpi_program_cmd,
            volatile_register_write_enable_cmd,
            write_enable_reg_index,
            quad_mode_enable_reg_index,
            busy_status_reg_index,
            write_enable_bit_pos,
            quad_enable_bit_pos,
            busy_status_bit_pos,
            write_enable_reg_write_len,
            write_enable_reg_read_len,
            quad_enable_reg_write_len,
            quad_enable_reg_read_len,
            release_power_down_cmd,
            busy_status_reg_read_len,
            read_reg_cmd_buffer,
            write_reg_cmd_buffer,
            enter_qpi_cmd,
            exit_qpi_cmd,
            continuous_read_mode_cfg,
            continuous_read_mode_exit_cfg,
            enable_burst_wrap_cmd,
            enable_burst_wrap_cmd_dummy_clock,
            burst_wrap_data_mode,
            burst_wrap_data,
            disable_burst_wrap_cmd,
            disable_burst_wrap_cmd_dummy_clock,
            disable_burst_wrap_data_mode,
            disable_burst_wrap_data,
            sector_erase_time_4k,
            sector_erase_time_32k,
            sector_erase_time_64k,
            page_program_time,
            chip_erase_time,
            power_down_delay,
            quad_enable_data,
            crc32,
        })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), ParseError> {
        use std::io::Cursor;

        let mut buf = [0u8; 88];

        // Create a new temporary `Cursor` for writing with mutable access - if we were to move
        // it out if this scope, we can no longer access `buf` as immutable when we need to
        // calculate the crc32 checksum
        {
            let mut buf_writer = Cursor::new(&mut buf[..]);

            // Write the magic header value
            buf_writer.write_all(b"FCFG")?;

            // Write io_mode
            buf_writer.write_all(&self.io_mode.to_le_bytes())?;

            // Write continuous_read_support
            buf_writer.write_all(&self.continuous_read_support.to_le_bytes())?;

            // Write clock_delay
            buf_writer.write_all(&self.clock_delay.to_le_bytes())?;

            // Write clock_invert
            buf_writer.write_all(&self.clock_invert.to_le_bytes())?;

            // Write reset_enable_cmd
            buf_writer.write_all(&self.reset_enable_cmd.to_le_bytes())?;

            // Write reset_cmd
            buf_writer.write_all(&self.reset_cmd.to_le_bytes())?;

            // Write reset_continuous_read_cmd
            buf_writer.write_all(&self.reset_continuous_read_cmd.to_le_bytes())?;

            // Write reset_continuous_read_cmd_size
            buf_writer.write_all(&self.reset_continuous_read_cmd_size.to_le_bytes())?;

            // Write jedec_id_cmd
            buf_writer.write_all(&self.jedec_id_cmd.to_le_bytes())?;

            // Write jedec_id_cmd_dummy_clock
            buf_writer.write_all(&self.jedec_id_cmd_dummy_clock.to_le_bytes())?;

            // Write qpi_jedec_id_cmd
            buf_writer.write_all(&self.qpi_jedec_id_cmd.to_le_bytes())?;

            // Write qpi_jedec_id_cmd_dummy_clock
            buf_writer.write_all(&self.qpi_jedec_id_cmd_dummy_clock.to_le_bytes())?;

            // Write sector_size
            buf_writer.write_all(&self.sector_size.to_le_bytes())?;

            // Write manufacturer_id
            buf_writer.write_all(&self.manufacturer_id.to_le_bytes())?;

            // Write page_size
            buf_writer.write_all(&self.page_size.to_le_bytes())?;

            // Write chip_erase_cmd
            buf_writer.write_all(&self.chip_erase_cmd.to_le_bytes())?;

            // Write sector_erase_cmd
            buf_writer.write_all(&self.sector_erase_cmd.to_le_bytes())?;

            // Write block_erase_32k_cmd
            buf_writer.write_all(&self.block_erase_32k_cmd.to_le_bytes())?;

            // Write block_erase_64k_cmd
            buf_writer.write_all(&self.block_erase_64k_cmd.to_le_bytes())?;

            // Write write_enable_cmd
            buf_writer.write_all(&self.write_enable_cmd.to_le_bytes())?;

            // Write page_program_cmd
            buf_writer.write_all(&self.page_program_cmd.to_le_bytes())?;

            // Write qio_page_program_cmd
            buf_writer.write_all(&self.qio_page_program_cmd.to_le_bytes())?;

            // Write qio_page_program_address_mode
            buf_writer.write_all(&self.qio_page_program_address_mode.to_le_bytes())?;

            // Write fast_read_cmd
            buf_writer.write_all(&self.fast_read_cmd.to_le_bytes())?;

            // Write fast_read_cmd_dummy_clock
            buf_writer.write_all(&self.fast_read_cmd_dummy_clock.to_le_bytes())?;

            // Write qpi_fast_read_cmd
            buf_writer.write_all(&self.qpi_fast_read_cmd.to_le_bytes())?;

            // Write qpi_fast_read_cmd_dummy_clock
            buf_writer.write_all(&self.qpi_fast_read_cmd_dummy_clock.to_le_bytes())?;

            // Write fast_read_dual_output_cmd
            buf_writer.write_all(&self.fast_read_dual_output_cmd.to_le_bytes())?;

            // Write fast_read_dual_output_cmd_dummy_clock
            buf_writer.write_all(&self.fast_read_dual_output_cmd_dummy_clock.to_le_bytes())?;

            // Write fast_read_dual_io_cmd
            buf_writer.write_all(&self.fast_read_dual_io_cmd.to_le_bytes())?;

            // Write fast_read_dual_io_cmd_dummy_clock
            buf_writer.write_all(&self.fast_read_dual_io_cmd_dummy_clock.to_le_bytes())?;

            // Write fast_read_quad_output_cmd
            buf_writer.write_all(&self.fast_read_quad_output_cmd.to_le_bytes())?;

            // Write fast_read_quad_output_cmd_dummy_clock
            buf_writer.write_all(&self.fast_read_quad_output_cmd_dummy_clock.to_le_bytes())?;

            // Write fast_read_quad_io_cmd
            buf_writer.write_all(&self.fast_read_quad_io_cmd.to_le_bytes())?;

            // Write fast_read_quad_io_cmd_dummy_clock
            buf_writer.write_all(&self.fast_read_quad_io_cmd_dummy_clock.to_le_bytes())?;

            // Write qpi_fast_read_quad_io_cmd
            buf_writer.write_all(&self.qpi_fast_read_quad_io_cmd.to_le_bytes())?;

            // Write qpi_fast_read_quad_io_cmd_dummy_clock
            buf_writer.write_all(&self.qpi_fast_read_quad_io_cmd_dummy_clock.to_le_bytes())?;

            // Write qpi_program_cmd
            buf_writer.write_all(&self.qpi_program_cmd.to_le_bytes())?;

            // Write volatile_register_write_enable_cmd
            buf_writer.write_all(&self.volatile_register_write_enable_cmd.to_le_bytes())?;

            // Write write_enable_reg_index
            buf_writer.write_all(&self.write_enable_reg_index.to_le_bytes())?;

            // Write quad_mode_enable_reg_index
            buf_writer.write_all(&self.quad_mode_enable_reg_index.to_le_bytes())?;

            // Write busy_status_reg_index
            buf_writer.write_all(&self.busy_status_reg_index.to_le_bytes())?;

            // Write write_enable_bit_pos
            buf_writer.write_all(&self.write_enable_bit_pos.to_le_bytes())?;

            // Write quad_enable_bit_pos
            buf_writer.write_all(&self.quad_enable_bit_pos.to_le_bytes())?;

            // Write busy_status_bit_pos
            buf_writer.write_all(&self.busy_status_bit_pos.to_le_bytes())?;

            // Write write_enable_reg_write_len
            buf_writer.write_all(&self.write_enable_reg_write_len.to_le_bytes())?;

            // Write write_enable_reg_read_len
            buf_writer.write_all(&self.write_enable_reg_read_len.to_le_bytes())?;

            // Write quad_enable_reg_write_len
            buf_writer.write_all(&self.quad_enable_reg_write_len.to_le_bytes())?;

            // Write quad_enable_reg_read_len
            buf_writer.write_all(&self.quad_enable_reg_read_len.to_le_bytes())?;

            // Write release_power_down_cmd
            buf_writer.write_all(&self.release_power_down_cmd.to_le_bytes())?;

            // Write busy_status_reg_read_len
            buf_writer.write_all(&self.busy_status_reg_read_len.to_le_bytes())?;

            // Write read_reg_cmd_buffer
            buf_writer.write_all(&self.read_reg_cmd_buffer)?;

            // Write write_reg_cmd_buffer
            buf_writer.write_all(&self.write_reg_cmd_buffer)?;

            // Write enter_qpi_cmd
            buf_writer.write_all(&self.enter_qpi_cmd.to_le_bytes())?;

            // Write exit_qpi_cmd
            buf_writer.write_all(&self.exit_qpi_cmd.to_le_bytes())?;

            // Write continuous_read_mode_cfg
            buf_writer.write_all(&self.continuous_read_mode_cfg.to_le_bytes())?;

            // Write continuous_read_mode_exit_cfg
            buf_writer.write_all(&self.continuous_read_mode_exit_cfg.to_le_bytes())?;

            // Write enable_burst_wrap_cmd
            buf_writer.write_all(&self.enable_burst_wrap_cmd.to_le_bytes())?;

            // Write enable_burst_wrap_cmd_dummy_clock
            buf_writer.write_all(&self.enable_burst_wrap_cmd_dummy_clock.to_le_bytes())?;

            // Write burst_wrap_data_mode
            buf_writer.write_all(&self.burst_wrap_data_mode.to_le_bytes())?;

            // Write burst_wrap_data
            buf_writer.write_all(&self.burst_wrap_data.to_le_bytes())?;

            // Write disable_burst_wrap_cmd
            buf_writer.write_all(&self.disable_burst_wrap_cmd.to_le_bytes())?;

            // Write disable_burst_wrap_cmd_dummy_clock
            buf_writer.write_all(&self.disable_burst_wrap_cmd_dummy_clock.to_le_bytes())?;

            // Write disable_burst_wrap_data_mode
            buf_writer.write_all(&self.disable_burst_wrap_data_mode.to_le_bytes())?;

            // Write disable_burst_wrap_data
            buf_writer.write_all(&self.disable_burst_wrap_data.to_le_bytes())?;

            // Write sector_erase_time_4k
            buf_writer.write_all(&self.sector_erase_time_4k.to_le_bytes())?;

            // Write sector_erase_time_32k
            buf_writer.write_all(&self.sector_erase_time_32k.to_le_bytes())?;

            // Write sector_erase_time_64k
            buf_writer.write_all(&self.sector_erase_time_64k.to_le_bytes())?;

            // Write page_program_time
            buf_writer.write_all(&self.page_program_time.to_le_bytes())?;

            // Write chip_erase_time
            buf_writer.write_all(&self.chip_erase_time.to_le_bytes())?;

            // Write power_down_delay
            buf_writer.write_all(&self.power_down_delay.to_le_bytes())?;

            // Write quad_enable_data
            buf_writer.write_all(&self.quad_enable_data.to_le_bytes())?;
        }

        // Write our temporary memory buffer to our final writer
        writer.write_all(&buf)?;

        // Calculate and write the crc32 checksum
        let crc32 = crc32(&buf[0x4..0x58]);

        writer.write_all(&crc32.to_le_bytes())?;

        Ok(())
    }
}

impl ClockConfig {
    pub fn from_reader<R: ReadBytesExt + Seek>(reader: &mut R) -> Result<Self, ParseError> {
        let mut conf = ClockConfig::default();
        let mut magic = [0u8; 4];

        // Read the magic header
        reader.read_exact(&mut magic)?;

        // Assert that the magic header is correct
        // Currently disabled because the eflash loaders have a magic header of [0, 0, 0, 0]
        //
        if &magic != b"PCFG" {
            return Err(ParseError::ClockConfigError(
                ClockConfigError::InvalidMagicHeader(magic),
            ));
        }

        // Read the xtal type
        conf.xtal_type = reader.read_u8()?;

        // Read the PLL clock
        conf.pll_clock = reader.read_u8()?;

        // Read the HCLK divider
        conf.hclk_divider = reader.read_u8()?;

        // Read the BCLK divider
        conf.bclk_divider = reader.read_u8()?;

        // Read the flash clock type
        conf.flash_clock_type = reader.read_u8()?;

        // Read the flash clock divider
        conf.flash_clock_divider = reader.read_u8()?;

        // Skip the 2 reserved bytes that are currently unused
        reader.seek(SeekFrom::Current(2))?;

        // Read the CRC32 checksum
        conf.crc32 = reader.read_u32::<LittleEndian>()?;

        Ok(conf)
    }

    /// Writes the clock config to the given `writer`
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), ParseError> {
        use std::io::Cursor;

        let mut buf = [0u8; 12];

        // Create a new temporary `Cursor` for writing with mutable access - if we were to move
        // it out if this scope, we can no longer access `buf` as immutable when we need to
        // calculate the crc32 checksum
        {
            let mut buf_writer = Cursor::new(&mut buf[..]);

            // Write the magic header value
            buf_writer.write_all(b"PCFG")?;

            // Write the xtal type
            buf_writer.write_all(&self.xtal_type.to_le_bytes())?;

            // Write the PLL clock type
            buf_writer.write_all(&self.pll_clock.to_le_bytes())?;

            // Write the HCLK divider value
            buf_writer.write_all(&self.hclk_divider.to_le_bytes())?;

            // Write the BCLK divider value
            buf_writer.write_all(&self.bclk_divider.to_le_bytes())?;

            // Write the flash clock type
            buf_writer.write_all(&self.flash_clock_type.to_le_bytes())?;

            // Write the flash clock divider value
            buf_writer.write_all(&self.flash_clock_divider.to_le_bytes())?;

            // Write the reserved, unused fields
            buf_writer.write_all(&[0, 0])?;
        }

        // Write our temporary memory buffer to our final writer
        writer.write_all(&buf)?;

        // Calculate and write the crc32 checksum
        let crc32 = crc32(&buf[0x4..0xc]);

        writer.write_all(&crc32.to_le_bytes())?;

        Ok(())
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
            cpu: Cpu::Cpu0,
            revision: 1,
            flash_config,
            clock_config,
            boot_config,
            image_segment_info: 0,
            entry_point,
            image_start: 0,
            hash: [0; 32],
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
    use std::io::Cursor;

    const REFERENCE_FIRMWARE: &[u8] =
        include_bytes!("../../test/whole_dts40M_pt2M_boot2release_ef7015.bin");

    #[test]
    fn it_should_read_clock_config() {
        let mut cursor = Cursor::new(&REFERENCE_FIRMWARE[0x64..0x74]);
        let clock_config = ClockConfig::from_reader(&mut cursor).unwrap();

        assert_eq!(clock_config.xtal_type, 4);
        assert_eq!(clock_config.flash_clock_divider, 1);
    }

    #[test]
    fn it_should_write_valid_clock_config() {
        let mut cursor = Cursor::new(&REFERENCE_FIRMWARE[0x64..0x74]);
        let clock_config = ClockConfig::from_reader(&mut cursor).unwrap();

        let mut buf: Vec<u8> = Vec::with_capacity(1024);
        clock_config.write_to(&mut buf).unwrap();

        assert_eq!(&buf[..], &REFERENCE_FIRMWARE[0x64..0x74]);
    }

    #[test]
    fn it_should_write_valid_flash_config() {
        let mut cursor = Cursor::new(&REFERENCE_FIRMWARE[0x8..0x64]);
        let flash_config = FlashConfig::from_reader(&mut cursor).unwrap();

        let mut buf: Vec<u8> = Vec::with_capacity(1024);
        flash_config.write_to(&mut buf).unwrap();

        assert_eq!(&buf[..], &REFERENCE_FIRMWARE[0x8..0x64]);
    }

    #[test]
    fn it_should_read_flash_config() {
        let mut cursor = Cursor::new(&REFERENCE_FIRMWARE[0x08..0x64]);
        let flash_config = FlashConfig::from_reader(&mut cursor).unwrap();

        assert_eq!(flash_config.io_mode, 4);
        assert_eq!(flash_config.sector_erase_time_32k, 1200);
        assert_eq!(flash_config.quad_enable_data, 0);
        assert_eq!(flash_config.crc32, 0xC4BDD748);
    }

    #[test]
    fn it_should_read_firmware() {
        let hash: [u8; 32] = [
            0xEF, 0xBE, 0xAD, 0xDE, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        let mut cursor = Cursor::new(&REFERENCE_FIRMWARE);
        let firmware = Firmware::from_reader(&mut cursor).unwrap();

        assert_eq!(firmware.cpu, Cpu::Cpu0);
        assert_eq!(firmware.revision, 1);
        assert_eq!(firmware.boot_config, 209664);
        assert_eq!(firmware.image_segment_info, 38608);
        assert_eq!(firmware.entry_point, 0);
        assert_eq!(firmware.image_start, 0x2000);
        assert_eq!(firmware.hash, hash);
        assert_eq!(firmware.crc32, 0xDEADBEEF);
    }
}
