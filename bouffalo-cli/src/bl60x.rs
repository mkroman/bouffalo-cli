use std::convert::TryInto;
use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::time::Duration;

use log::debug;
use serial::{SerialPort, SystemPort};
use thiserror::Error;

/// The serial settings expected by the BootROM on the bl602
pub const BL602_BOOTROM_SERIAL_SETTINGS: serial::PortSettings = serial::PortSettings {
    baud_rate: serial::BaudOther(500_000),
    char_size: serial::Bits8,
    parity: serial::ParityNone,
    stop_bits: serial::Stop1,
    flow_control: serial::FlowNone,
};

pub struct Bl60xSerialPort {
    port: SystemPort,
}

pub trait SerialWritableCommand {
    fn write_cmd_to_buf(&self, buf: &mut Vec<u8>) -> Result<(), IspError>;
}

/// Command to request the BootROM information
pub struct GetBootInfo;

impl SerialWritableCommand for GetBootInfo {
    fn write_cmd_to_buf(&self, buf: &mut Vec<u8>) -> Result<(), IspError> {
        buf.extend_from_slice(&[0x10, 0x00, 0x00, 0x00]);

        Ok(())
    }
}

/// Command that will load the given bootheader onto the device
pub struct LoadBootHeader {
    pub bootheader: [u8; 176],
}

impl SerialWritableCommand for LoadBootHeader {
    fn write_cmd_to_buf(&self, buf: &mut Vec<u8>) -> Result<(), IspError> {
        let mut tmp = [0u8; 4];

        // Set the command id
        tmp[0] = 0x11;

        // Set the boot header length
        tmp[0x2..0x4].copy_from_slice(&(self.bootheader.len() as u16).to_le_bytes());

        // Copy the command id and boot header length into the output buffer
        buf.extend_from_slice(&tmp);

        // Copy the bootheader itself
        buf[0x4..].copy_from_slice(&self.bootheader);

        Ok(())
    }
}

/// The boot info returned from the device when requested
#[derive(Debug, Clone, Copy)]
pub struct BootInfo {
    /// The version of the boot ROM
    pub rom_version: u32,
    /// OTP information - ??
    pub otp_info: [u8; 16],
}

#[derive(Error, Debug)]
pub enum IspError {
    #[error("Handshake failed - expected OK, got {:?}", _0)]
    HandshakeFailed([u8; 2]),
    #[error("I/O error: {}", _0)]
    IoError(#[from] io::Error),
}

impl Bl60xSerialPort {
    /// Opens the given `port` and configures it to use the communication settings expected by the
    /// BL60x bootrom
    pub fn open<T: AsRef<OsStr> + ?Sized>(port: &T) -> Result<Bl60xSerialPort, serial::Error> {
        debug!("Opening serial port {:?}", port.as_ref());

        let mut port = serial::open(port)?;
        let settings = BL602_BOOTROM_SERIAL_SETTINGS;
        let timeout = Duration::from_millis(2000);

        debug!("Setting baud rate to {}", settings.baud_rate.speed());
        port.configure(&settings)?;
        debug!("Setting timeout to {:?}", timeout);
        port.set_timeout(timeout)?;

        Ok(Bl60xSerialPort { port })
    }

    /// Makes the BootROM enter UART mode, returns `()` on success, `IspError` otherwise
    pub fn enter_uart_mode(&mut self) -> Result<(), IspError> {
        let mut buf = [0u8; 2];

        let _ = self.port.write(&[0x55, 0x55, 0x55])?;

        self.port.read(&mut buf)?;

        if &buf != b"OK" {
            return Err(IspError::HandshakeFailed(buf));
        }

        Ok(())
    }

    /// Sends the given `command` to the device and returns `()` if it was sent successfully,
    /// without reading the response
    pub fn send_command<T: Into<Box<impl SerialWritableCommand>>>(
        &mut self,
        command: T,
    ) -> Result<(), IspError> {
        let mut buf: Vec<u8> = Vec::with_capacity(4096);

        command.into().write_cmd_to_buf(&mut buf)?;
        self.port.write(&buf)?;

        Ok(())
    }

    /// Requests boot info from the BootROM
    pub fn get_boot_info(&mut self) -> Result<BootInfo, IspError> {
        let mut buf = [0u8; 24];

        self.send_command(GetBootInfo)?;
        let _ = self.port.read(&mut buf)?;

        let rom_version = u32::from_le_bytes(buf[0x4..0x8].try_into().unwrap());
        let otp_info = buf[0x8..0x18].try_into().unwrap();

        Ok(BootInfo {
            rom_version,
            otp_info,
        })
    }
}
