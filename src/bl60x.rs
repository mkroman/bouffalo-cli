use std::convert::TryInto;
use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::time::Duration;

use log::debug;
use num_enum::FromPrimitive;
use serial::{SerialPort, SystemPort};
use thiserror::Error;

use crate::bl::bootrom;

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
        buf.extend_from_slice(&self.bootheader);

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
    #[error("Boot ROM error: {}", _0)]
    BootRomError(bootrom::Error),
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

    pub fn set_baud_rate(&mut self, baud_rate: serial::BaudRate) -> Result<(), serial::Error> {
        self.port.reconfigure(&|settings| {
            settings.set_baud_rate(baud_rate)?;

            Ok(())
        })?;

        Ok(())
    }

    pub fn open_with_baud_rate<T: AsRef<OsStr> + ?Sized>(
        port: &T,
        baud_rate: usize,
    ) -> Result<Bl60xSerialPort, serial::Error> {
        debug!("Opening serial port {:?}", port.as_ref());

        let mut port = serial::open(port)?;
        let mut settings = BL602_BOOTROM_SERIAL_SETTINGS;
        let timeout = Duration::from_millis(2000);
        settings.baud_rate = serial::BaudOther(baud_rate);

        debug!("Setting baud rate to {}", settings.baud_rate.speed());
        port.configure(&settings)?;
        debug!("Setting timeout to {:?}", timeout);
        port.set_timeout(timeout)?;

        Ok(Bl60xSerialPort { port })
    }

    /// Makes the BootROM enter UART mode, returns `()` on success, `IspError` otherwise
    pub fn enter_uart_mode(&mut self) -> Result<(), IspError> {
        let mut buf = [0u8; 2];
        let _ = self.port.write_all(&[
            0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
            0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
            0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
            0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
            0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
            0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
            0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
            0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
        ])?;

        std::thread::sleep(Duration::from_millis(1));
        self.port.read_exact(&mut buf)?;

        if &buf[0x0..0x2] != b"OK" {
            return Err(IspError::HandshakeFailed([buf[0], buf[1]]));
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

        self.port.write_all(&buf)?;

        Ok(())
    }

    /// Reads the error code from the serial port
    ///
    /// Note: only call this if the response is not 'OK', otherwise it'll just time out
    pub fn read_error(&mut self) -> Result<u16, IspError> {
        let mut buf = [0u8; 2];

        self.port.read_exact(&mut buf)?;

        Ok(u16::from_le_bytes([buf[0], buf[1]]))
    }

    /// Sends the given buf as a boot header and attempts to load it
    pub fn load_boot_header(&mut self, boot_header: [u8; 176]) -> Result<(), IspError> {
        debug!("Sending LoadBootHeader command");

        self.send_command(LoadBootHeader {
            bootheader: boot_header,
        })?;
        std::thread::sleep(Duration::from_millis(1));

        let mut buf = [0u8; 2];
        let _ = self.port.read_exact(&mut buf)?;

        if &buf != b"OK" {
            let err = self.read_error()?;

            println!("error code: {:#x}", err);

            return Err(IspError::HandshakeFailed([buf[0], buf[1]]));
        }

        Ok(())
    }

    pub fn check_image(&mut self) -> Result<(), IspError> {
        let mut buf = [0u8; 4];

        buf[0] = 0x19;

        self.port.write_all(&buf)?;
        std::thread::sleep(Duration::from_millis(100));

        let mut res_buf = [0u8; 2];
        self.port.read_exact(&mut res_buf)?;

        debug!("check_image: {:?}", res_buf);

        Ok(())
    }

    pub fn run_image(&mut self) -> Result<(), IspError> {
        let mut buf = [0u8; 4];

        buf[0] = 0x1a;

        self.port.write_all(&buf)?;

        std::thread::sleep(Duration::from_millis(1));
        let mut res_buf = [0u8; 2];
        self.port.read_exact(&mut res_buf)?;

        debug!("run_image: {:?}", res_buf);

        Ok(())
    }

    /// Reads exactly `num_bytes` from the serial port into a heap-allocated `Vec<u8>` and returns
    /// it when successful, otherwise returns an `io::Error`
    pub fn read_exact(&mut self, num_bytes: usize) -> Result<Vec<u8>, io::Error> {
        let mut buf = vec![0u8; num_bytes];

        self.port.read_exact(&mut buf)?;

        Ok(buf)
    }

    /// Loads the given segment into RAM on the device
    pub fn load_segment(&mut self, segment: &crate::bl::Segment) -> Result<(), IspError> {
        let mut buf: Vec<u8> = Vec::with_capacity(4096);
        let mut hdr_buf = [0u8; 20];

        let dest_addr = segment.dest_addr;
        let chunks = segment.data.chunks(4092);
        let num_chunks = chunks.len();

        debug!(
            "Starting load of {} byte segment over {} chunked transfers to {:08x?}",
            segment.data.len(),
            num_chunks,
            dest_addr
        );

        // Write the segment header
        hdr_buf[0] = 0x17;
        hdr_buf[0x2..0x4].copy_from_slice(&16u16.to_le_bytes());

        // Write the dest_addr
        hdr_buf[0x4..0x8].copy_from_slice(&dest_addr.0.to_le_bytes());
        // Write the segment size
        hdr_buf[0x8..0xc].copy_from_slice(&(segment.data.len() as u32).to_le_bytes());
        // Write the reserved bytes since they're apparently used for something
        hdr_buf[0xc..0x10].copy_from_slice(&segment.reserved.to_le_bytes());
        // Write the crc32
        let crc32 = crate::bl::crc32(&hdr_buf[0x4..0x10]);
        hdr_buf[0x10..0x14].copy_from_slice(&crc32.to_le_bytes());

        self.port.write_all(&hdr_buf)?;

        let mut res_buf = [0u8; 2];
        self.port.read_exact(&mut res_buf)?;
        // println!("hdr res {:?} hdr: {:x?}", res_buf, hdr_buf);

        if &res_buf != b"OK" {
            let err = self.read_error()?;

            return Err(IspError::BootRomError(bootrom::Error::from_primitive(err)));
        }

        for (idx, chunk) in chunks.enumerate() {
            std::thread::sleep(Duration::from_millis(100));

            debug!(
                "Loading segment {}/{} of {} bytes",
                idx + 1,
                num_chunks,
                chunk.len()
            );

            // Load segment data
            buf.clear();

            // Write command id
            buf.push(0x18);
            // Write reserved field
            buf.push(0x00);
            // Write the length
            buf.extend_from_slice(&(chunk.len() as u16).to_le_bytes());
            // Write the segment
            buf.extend_from_slice(&chunk);

            println!("{}", idx);
            self.port.write_all(&buf)?;

            self.port.read_exact(&mut res_buf)?;

            if &res_buf == b"FL" {
                let err = self.read_error()?;

                println!("Error when sending segment header: {:#x}", err);
                println!("hdr res {:?} hdr: {:x?}", res_buf, hdr_buf);
            } else if &res_buf == b"OK" {
                // Do nothing
                // debug!("OK");
            } else {
                debug!("non-fl res_buf: {:?}", res_buf);

                let num_res = u16::from_le_bytes(res_buf);
                // Read num_res bytes
                let mut tmp = vec![0u8; num_res as usize];
                self.port.read_exact(&mut tmp)?;
                debug!("tmp: {:?}", tmp);

                self.port.read_exact(&mut res_buf)?;
                // TODO: handle response
                println!("res_buf {:?}", res_buf);
            }
        }

        Ok(())
    }

    /// Requests boot info from the BootROM
    pub fn get_boot_info(&mut self) -> Result<BootInfo, IspError> {
        self.send_command(GetBootInfo)?;

        let mut buf = [0u8; 24];
        let _ = self.port.read_exact(&mut buf)?;

        let rom_version = u32::from_le_bytes(buf[0x4..0x8].try_into().unwrap());
        let otp_info = buf[0x8..0x18].try_into().unwrap();

        Ok(BootInfo {
            rom_version,
            otp_info,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bl::{self, Firmware};
    use std::io::Cursor;

    #[test]
    fn it_should_serialize_get_boot_info_cmd() {
        let mut buf = vec![];

        let cmd = GetBootInfo;
        cmd.write_cmd_to_buf(&mut buf).unwrap();

        assert_eq!(&buf, &[0x10, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn it_should_serialize_load_boot_header() {
        let mut buf: Vec<u8> = vec![];

        let boot_header = {
            let mut tmp = vec![];
            let fw = Firmware::from_reader(Cursor::new(&bl::EFLASH_LOADER_NONE_BIN)).unwrap();

            fw.write_to(&mut tmp).unwrap();
            tmp
        };

        let cmd = LoadBootHeader {
            bootheader: boot_header.try_into().unwrap(),
        };

        cmd.write_cmd_to_buf(&mut buf).unwrap();
        assert_eq!(&buf[..4], &[0x11, 0x0, 0xb0, 0x00]);
        assert_eq!(&buf[4..], &bl::EFLASH_LOADER_NONE_BIN[0..176]);
    }
}
