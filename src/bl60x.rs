use std::convert::TryInto;
use std::ffi::OsStr;
use std::io::{self, Read, Write};
use std::time::Duration;

use log::{debug, trace, warn};
use num_enum::FromPrimitive;
use serialport::prelude::*;
use thiserror::Error;

use crate::bl::bootrom;
pub use crate::error::SerialError;

/// The serial settings expected by the BootROM on the bl602
pub const BL602_BOOTROM_SERIAL_SETTINGS: SerialPortSettings = SerialPortSettings {
    baud_rate: 500_000,
    data_bits: DataBits::Eight,
    flow_control: FlowControl::None,
    parity: Parity::None,
    stop_bits: StopBits::One,
    timeout: Duration::from_secs(2),
};

pub struct Bl60xSerialPort {
    port: Box<dyn SerialPort>,
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
    #[error("The device returned an unexpected reply")]
    UnexpectedReply,

    #[error("Handshake failed - expected OK, got {:x?}", _0)]
    HandshakeFailed([u8; 2]),
    #[error("Boot ROM error: {}", _0)]
    BootRomError(bootrom::Error),
    #[error("I/O error: {}", _0)]
    IoError(#[from] io::Error),
}

impl Bl60xSerialPort {
    /// Opens the given `port` and configures it to use the communication settings expected by the
    /// BL60x bootrom
    pub fn open_with_baud_rate<T: AsRef<OsStr> + ?Sized>(
        port: &T,
        baud_rate: usize,
    ) -> Result<Bl60xSerialPort, SerialError> {
        debug!("Opening serial port {:?}", port.as_ref());

        let mut settings = BL602_BOOTROM_SERIAL_SETTINGS;
        let timeout = Duration::from_millis(2000);

        settings.baud_rate = baud_rate as u32;

        debug!("Setting baud rate to {}", settings.baud_rate);
        debug!("Setting timeout to {:?}", timeout);

        let port = serialport::open_with_settings(port, &settings).map_err(|err| {
            SerialError::OpenError(port.as_ref().to_string_lossy().into_owned(), err)
        })?;

        Ok(Bl60xSerialPort { port })
    }

    pub fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), serialport::Error> {
        debug!("Setting serial port baud rate to {}", baud_rate);

        self.port.set_baud_rate(baud_rate)?;

        Ok(())
    }

    /// Sets the timeout of the serial port
    pub fn set_timeout(&mut self, timeout: Duration) -> Result<(), serialport::Error> {
        debug!("Setting serial timeout to {}", timeout.as_secs());

        self.port.set_timeout(timeout)
    }

    /// Makes the BootROM enter UART mode, returns `()` on success, `IspError` otherwise
    ///
    /// Note:
    /// After getting a successful result, the user should wait 20ms before proceeding with
    /// communication
    pub fn enter_uart_mode(&mut self) -> Result<(), IspError> {
        let mut buf = [0u8; 2];

        // Calculate the number of bytes to send in order to to keep the UART busy for 5ms
        // bauds * 3s / (8 data bits + 1 start bit + 1 stop bit) / 1000 ms
        let num_bytes = self
            .port
            .baud_rate()
            .expect("Could not get baud rate for serial port")
            .saturating_mul(3)
            / 10
            / 1000;

        trace!("Trying to put device in UART mode");
        trace!("Sending {} x 0x55 bytes", num_bytes);

        self.port.write_all(&vec![0x55u8; num_bytes as usize])?;
        self.port.read_exact(&mut buf)?;

        if &buf != b"OK" {
            return Err(IspError::HandshakeFailed([buf[0], buf[1]]));
        }

        trace!("Device successfully entered UART mode");

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
        trace!("Trying to load boot header to RAM");

        self.send_command(LoadBootHeader {
            bootheader: boot_header,
        })?;

        let mut buf = [0u8; 2];
        let _ = self.port.read_exact(&mut buf)?;

        if &buf != b"OK" {
            let err = self.read_error()?;

            println!("error code: {:#x}", err);

            return Err(IspError::HandshakeFailed([buf[0], buf[1]]));
        }

        Ok(())
    }

    /// Reads the 2-byte response from the ROM, returning Ok(()) if the device replies with b"OK",
    /// returns err otherwise
    pub fn read_reply(&mut self) -> Result<(), IspError> {
        let mut buf = [0u8; 2];

        trace!("Reading device reply");
        self.port.read_exact(&mut buf)?;

        if &buf != b"OK" {
            if &buf != b"FL" {
                warn!("Unexpected reply from device");

                return Err(IspError::UnexpectedReply);
            }

            trace!("Reading error code from device");

            // Read the next 2 bytes which is an error code
            self.port.read_exact(&mut buf)?;
            let error_code = u16::from_le_bytes([buf[0], buf[1]]);

            debug!("Error code from device: {}", error_code);

            return Err(IspError::BootRomError(bootrom::Error::from_primitive(
                error_code,
            )));
        }

        trace!("Device replied with OK");

        Ok(())
    }

    /// Reads the exact number of bytes at flash `addr` to full `buf`.
    pub fn read_flash_exact(&mut self, addr: u32, out_buf: &mut [u8]) -> Result<(), IspError> {
        let mut buf = [0u8; 12];

        debug!(
            "Reading flash 0x{:08x}..{:#08x}",
            addr,
            addr as usize + out_buf.len()
        );

        buf[0x00] = 0x32;

        buf[0x02] = 0x08;
        buf[0x04..0x08].copy_from_slice(&addr.to_le_bytes());
        buf[0x08..0x0c].copy_from_slice(&(out_buf.len() as u32).to_le_bytes());

        // Calculate the 8-bit checksum
        buf[0x01] = buf[0x02..0x0c]
            .iter()
            .fold(0u8, |acc, &x| acc.wrapping_add(x));

        // Write the command to the serial device
        self.port.write_all(&buf)?;

        // Read the response and assert that it is OK
        let _ = self.read_reply()?;

        // Read the flash data length
        let mut len_buf = [0u8; 2];
        self.port.read_exact(&mut len_buf)?;
        let length = u16::from_le_bytes([len_buf[0], len_buf[1]]);

        // Assert that the returned length is smaller than the buffer used in the eflash loader
        assert!(length <= 8192);

        // Read the flash data
        self.port.read_exact(out_buf)?;

        trace!("Successfully read {} bytes from flash", length);

        Ok(())
    }

    /// Erases the flash at the given `address` and the following `size` bytes
    pub fn erase_flash(&mut self, addr: u32, size: u32) -> Result<(), IspError> {
        let mut cmd = [0u8; 12];

        let start = addr;
        let end = start + size;

        // Write the command id
        cmd[0x00] = 0x30;
        // Write the length of the command
        cmd[0x02] = 0x08;
        // Write the start address we want to erase from
        cmd[0x04..0x08].copy_from_slice(&start.to_le_bytes());
        // Write the end address we want to erase to
        cmd[0x08..0x0c].copy_from_slice(&end.to_le_bytes());

        // Calculate and write the checksum for the command data
        cmd[0x01] = cmd[0x02..0x0c]
            .iter()
            .fold(0u8, |acc, &x| acc.wrapping_add(x));

        trace!(
            "Erasing flash regions 0x{:08x}..0x{:08x}",
            addr,
            addr + size
        );

        self.port.write_all(&cmd)?;
        self.read_reply()?;

        trace!("Flash regions successfully erased");

        Ok(())
    }

    /// Writes the given `data` to the flash at offset `addr`, starting from 0
    pub fn write_flash(&mut self, addr: u32, data: &[u8]) -> Result<(), IspError> {
        const WRITE_SIZE: usize = 8192;
        let mut cmd = [0u8; 8];

        // Erase the flash we want to write to, to ensure that it's all zeros
        self.erase_flash(addr, data.len().try_into().unwrap())?;

        let mut remaining = data.len();
        let mut start = addr;
        let n = remaining / WRITE_SIZE;
        let n = if remaining % WRITE_SIZE > 0 { n + 1 } else { n };

        for i in 0..n {
            let num_bytes = std::cmp::min(remaining, WRITE_SIZE);

            // Write the command id
            cmd[0x00] = 0x31;
            // Write the length of the payload
            cmd[0x02..0x04].copy_from_slice(&((num_bytes as u16) + 4).to_le_bytes());
            // Write the start address
            cmd[0x04..0x08].copy_from_slice(&start.to_le_bytes());

            let off = i * WRITE_SIZE;
            let payload = &data[off..off + num_bytes];

            // Calculate and write the checksum
            let chksum = cmd[0x02..0x08]
                .iter()
                .fold(0u8, |acc, &x| acc.wrapping_add(x));

            let chksum = payload.iter().fold(chksum, |acc, &x| acc.wrapping_add(x));

            cmd[0x01] = chksum;

            trace!("Writing {} bytes to flash @ 0x{:08x}", num_bytes, start);

            self.port.write_all(&cmd)?;
            self.port.write_all(&payload)?;

            self.read_reply()?;

            trace!("Successfully wrote {} bytes", num_bytes);

            start += num_bytes as u32;
            remaining -= num_bytes;
        }

        Ok(())
    }

    /// Attempts to have the device calculate the sha256 hash of the flash contents at `addr` up
    /// until the `addr` + `len` bytes and return it
    pub fn flash_sha256(&mut self, addr: u32, len: u32) -> Result<[u8; 32], IspError> {
        let mut cmd = [0u8; 12];

        cmd[0x00] = 0x3d;
        // …
        cmd[0x02] = 0x08;
        // …
        cmd[0x04..0x08].copy_from_slice(&addr.to_le_bytes());
        cmd[0x08..0x0c].copy_from_slice(&len.to_le_bytes());

        // Calculate the 8-bit checksum
        cmd[0x01] = cmd[0x02..0x0c].iter().sum();

        // Write the command to the serial device
        self.port.write_all(&cmd)?;

        // Assert that the reponse is OK
        let _ = self.read_reply()?;

        // Read the sha256 data length
        let mut len_buf = [0u8; 2];
        self.port.read_exact(&mut len_buf)?;
        let length = u16::from_le_bytes([len_buf[0], len_buf[1]]);

        assert_eq!(length, 32);

        // Read the sha256 data
        let mut buf = [0u8; 32];
        self.port.read_exact(&mut buf)?;

        Ok(buf)
    }

    pub fn check_image(&mut self) -> Result<(), IspError> {
        let mut buf = [0u8; 4];

        buf[0] = 0x19;

        trace!("Sending check image command");

        self.port.write_all(&buf)?;
        self.read_reply()?;

        trace!("Successfully sent check image command");

        Ok(())
    }

    pub fn run_image(&mut self) -> Result<(), IspError> {
        let mut buf = [0u8; 4];

        buf[0] = 0x1a;

        trace!("Sending run image command");

        self.port.write_all(&buf)?;

        let _ = self.read_reply()?;

        trace!("Successfully sent run image command");

        Ok(())
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
                "Loading {:4} byte segment [{:02}/{:02}]",
                chunk.len(),
                idx + 1,
                num_chunks,
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
                let num_res = u16::from_le_bytes(res_buf);
                // Read num_res bytes
                let mut tmp = vec![0u8; num_res as usize];
                self.port.read_exact(&mut tmp)?;

                self.port.read_exact(&mut res_buf)?;

                // TODO: handle response
                //println!("res_buf {:?}", res_buf);
            }
        }

        Ok(())
    }

    /// Requests boot info from the BootROM
    pub fn get_boot_info(&mut self) -> Result<BootInfo, IspError> {
        trace!("Requesting ROM boot info");

        self.send_command(GetBootInfo)?;

        let mut buf = [0u8; 24];
        let _ = self.port.read_exact(&mut buf)?;

        let rom_version = u32::from_le_bytes(buf[0x4..0x8].try_into().unwrap());
        let otp_info = buf[0x8..0x18].try_into().unwrap();

        trace!("Received ROM boot info");

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
        assert_eq!(&buf[..4], &[0x11, 0x00, 0xb0, 0x00]);
        assert_eq!(&buf[4..], &bl::EFLASH_LOADER_NONE_BIN[0..176]);
    }
}
