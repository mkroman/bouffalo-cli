use std::convert::TryInto;
use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::time::Duration;

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

/// The boot info returned from the device when requested
#[derive(Debug, Clone, Copy)]
pub struct BootInfo {
    /// The version of the boot ROM
    bootrom_version: u32,
    /// OTP information - ??
    otp_info: [u8; 16],
}

#[derive(Error, Debug)]
pub enum IspError {
    #[error("Handshake failed - expected OK, got {:?}", _0)]
    HandshakeFailed([u8; 2]),
    #[error("I/O error: {}", _0)]
    IoError(#[from] io::Error),
}

/// Commands that can be sent to the device in UART mode
pub enum Command<'a> {
    /// Fetches the BootROM info
    GetBootInfo,
    /// Loads the given boot header to the device
    LoadBootHeader(&'a [u8; 176]),
    /// Loads the given public key to the device
    LoadPublicKey(&'a [u8; 68]),
    /// Loads the given signature to the device
    LoadSignature(&'a [u8]),
    LoadAesIv(&'a [u8]),
    LoadSegmentHeader {
        dest_addr: u32,
        len: u32,
        crc: u32,
    },
    LoadSegmentData(&'a [u8]),
    CheckImage,
    RunImage,
}

impl Bl60xSerialPort {
    /// Opens the given `port` and configures it to use the communication settings expected by the
    /// BL60x bootrom
    pub fn open<T: AsRef<OsStr> + ?Sized>(port: &T) -> Result<Bl60xSerialPort, serial::Error> {
        let mut port = serial::open(port)?;

        port.configure(&BL602_BOOTROM_SERIAL_SETTINGS)?;
        port.set_timeout(Duration::from_millis(2000))?;

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

    /// Requests boot info from the BootROM
    pub fn get_boot_info(&mut self) -> Result<BootInfo, IspError> {
        let mut buf = [0u8; 24];

        self.port.write(&[0x10, 0x00, 0x00, 0x00])?;
        let _ = self.port.read(&mut buf)?;

        let bootrom_version = u32::from_le_bytes(buf[0x4..0x8].try_into().unwrap());
        let otp_info = buf[0x8..0x18].try_into().unwrap();

        for (i, b) in buf[0x8..0x18].iter().enumerate() {
            println!("otp[{:2}]: {:08b}", i, b);
        }

        Ok(BootInfo {
            bootrom_version,
            otp_info,
        })
    }
}
