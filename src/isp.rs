use std::cell::RefCell;
use std::io::{Cursor, Read, Write};
use std::ops::DerefMut;
use std::time::{Duration, Instant};

use byteorder::{LittleEndian, ReadBytesExt};

use crate::{Error, VirtAddr};

/// The amount of time of inactivity until the bootloader resets.
const BOOTLOADER_TIMEOUT: Duration = Duration::from_millis(2000);

/// A trait marker to mark a type as a programming protocol.
pub trait Protocol {}

/// Protocol definition for the bootloader in the masked boot ROM.
pub struct Rom;

impl Protocol for Rom {}

/// Protocol definition for the eflash loader.
pub struct EFlashLoader;

impl Protocol for EFlashLoader {}

/// An interface for serializing commands to a writer in a binary protocol that the target
/// supports.
pub trait Command<P: Protocol> {
    fn to_writer<W: Write>(&self, writer: W) -> Result<(), Error>;
}

/// An interface for deserializing commands from a reader.
pub trait Response<P: Protocol>
where
    Self: Sized,
{
    /// The type to return on successful read from a reader.
    type T;

    fn from_reader<R>(reader: R) -> Result<Self::T, Error>
    where
        R: Read + ReadBytesExt;
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
/// Status response that indicates whether the recently sent ISP command was successful.
pub struct Status;

impl Response<Rom> for Status {
    type T = ();

    fn from_reader<R: Read + ReadBytesExt>(mut reader: R) -> Result<Self::T, Error> {
        let mut buf = [0u8; 2];

        reader.read_exact(&mut buf)?;

        if &buf == b"OK" {
            return Ok(());
        } else if &buf == b"FL" {
            // Read the error code
            reader.read_exact(&mut buf)?;

            let code = u16::from_le_bytes([buf[0], buf[1]]);

            return Err(Error::BootloaderError(code));
        }

        unreachable!(
            "expected response to be b\"OK\" or b\"FL\" but it was {:#x?}",
            buf
        )
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
/// ROM bootloader command to retrieve the bootloader version and OTP bit flags.
struct GetBootInfo;

impl Command<Rom> for GetBootInfo {
    fn to_writer<W: Write>(&self, mut writer: W) -> Result<(), Error> {
        writer.write_all(&[0x10, 0x00, 0x00, 0x00])?;

        Ok(())
    }
}

/// The return type of the GetBootInfo command when successful.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct BootInfo {
    /// The version of the bootloader in ROM
    pub version: u32,
    /// OTP flags
    pub otp_info: [u8; 16],
}

impl Response<Rom> for BootInfo {
    type T = Self;

    fn from_reader<R: Read + ReadBytesExt>(mut reader: R) -> Result<Self, Error> {
        // Read and assert the status response
        Status::from_reader(&mut reader)?;

        // Read the full boot info response into memory
        let mut buf = [0u8; 22];
        reader.read_exact(&mut buf)?;

        // Create a cursor over the memory buffer
        let mut cursor = Cursor::new(buf);

        // Read and assert that the length of the response message is exactly 20 bytes
        let length = cursor.read_u16::<LittleEndian>()?;
        assert_eq!(length, 20);

        // Read the bootloader version
        let version = cursor.read_u32::<LittleEndian>()?;

        // Read the OTP flags
        let mut otp_info = [0u8; 16];
        cursor.read_exact(&mut otp_info)?;

        Ok(BootInfo { version, otp_info })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
/// Command to load the given boot header into memory.
pub struct LoadBootHeader {
    /// The boot header, in binary, to load on the device
    boot_header: Vec<u8>,
}

impl Command<Rom> for LoadBootHeader {
    fn to_writer<W: Write>(&self, mut writer: W) -> Result<(), Error> {
        let mut tmp = [0u8; 180];

        // Write the command id
        tmp[0x00] = 0x11;

        // Write the bootheader data length
        tmp[0x02] = 176;

        // Write the bootheader data
        tmp[0x04..].copy_from_slice(&self.boot_header);

        writer.write_all(&tmp)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
/// Command to load the given segment header details into memory.
pub struct LoadSegmentHeader {
    /// The destination address for the segment
    pub dest_addr: VirtAddr,
    /// Reserved bytes - we need to store these because they're apparently used for something
    pub reserved: u32,
    /// The length of the segment
    pub size: u32,
}

impl Command<Rom> for LoadSegmentHeader {
    fn to_writer<W: Write>(&self, mut writer: W) -> Result<(), Error> {
        let mut tmp = [0u8; 20];

        // Write the command id
        tmp[0x00] = 0x17;

        // Write the command length
        tmp[0x02] = 16;

        let seg_header = &mut tmp[0x4..];

        // Write the segment destination address
        seg_header[0x00..0x04].copy_from_slice(&self.dest_addr.0.to_le_bytes());
        // Write the segment size
        seg_header[0x04..0x08].copy_from_slice(&self.size.to_le_bytes());
        // Write the reserved 32-bit value
        seg_header[0x08..0x0c].copy_from_slice(&self.reserved.to_le_bytes());
        // Write the command field checksum
        let crc = crc32(&seg_header[0x00..0x0c]);
        seg_header[0x0c..0x10].copy_from_slice(&crc.to_le_bytes());

        writer.write_all(&mut tmp)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
/// Command to load the givent segment `data` into memory at the destination given in a load
/// segment header command.
pub struct LoadSegment {
    /// The data to load into memory
    data: Vec<u8>,
}

impl Command<Rom> for LoadSegment {
    fn to_writer<W: Write>(&self, mut writer: W) -> Result<(), Error> {
        // The size of the command prefix
        const COMMAND_LEN: usize = 4;

        // Assert that the segment size is not larger than 2^16-1
        assert!(self.data.len() < std::u16::MAX as usize);

        // Create a buffer on the heap
        let mut tmp = vec![0u8; COMMAND_LEN + self.data.len()];

        // Write the command id
        tmp[0x00] = 0x18;

        // Write the command length
        tmp[0x02..0x04].copy_from_slice(&(self.data.len() as u16).to_le_bytes());

        // Write the segment data
        tmp[0x04..].copy_from_slice(&self.data);

        writer.write_all(&tmp)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Copy, Default)]
/// Command to have the bootloader check and verify the boot header and segments.
pub struct CheckImage;

impl Command<Rom> for CheckImage {
    fn to_writer<W: Write>(&self, mut writer: W) -> Result<(), Error> {
        writer.write_all(&[0x19, 0x00, 0x00, 0x00])?;

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Copy, Default)]
/// Command to have the bootloader jump to the entry point as defined in the boot header.
pub struct RunImage;

impl Command<Rom> for RunImage {
    fn to_writer<W: Write>(&self, mut writer: W) -> Result<(), Error> {
        writer.write_all(&[0x1a, 0x00, 0x00, 0x00])?;

        Ok(())
    }
}

/// Calculates the crc32 checksum for the given slice of `bytes`
///
/// The crc32 is implemented with the polynomial 0xEDB88320 and the initial value of 0xFFFFFFFF
pub fn crc32(bytes: &[u8]) -> u32 {
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

/// This is a type that takes ownership of a `SerialPort` once the end-device has entered its
/// masked ROM bootloader.
///
/// Every function that sends a command to the bootloader will return a Result.
///
/// If we try to send a command after more than 2000ms of inactivity, the function will fail
/// immediately and return an Err with the ownership of the inner `SerialPort`.
/// It is up to the user to handle this and take back ownership.
pub struct Bootloader {
    /// The time since we last interacted with the bootloader
    ///
    /// This is used to determine whether the bootloader has reset since our last command, in which
    /// case we'll have to return an error
    pub last_interaction: Instant,
    /// The serial port connected to the device
    pub port: RefCell<crate::SerialPort>,
}

impl Bootloader {
    pub fn get_boot_info(&mut self) -> Result<BootInfo, Error> {
        self.send_and_receive::<_, BootInfo>(GetBootInfo)
    }

    /// Reads the response type `T` from the bootloader.
    pub fn read_response<T: Response<Rom>>(&mut self) -> Result<T::T, Error> {
        let mut port = self.port.borrow_mut();
        let response = T::from_reader(port.deref_mut().deref_mut());

        self.last_interaction = Instant::now();

        response
    }

    /// Sends the given command `cmd` and then tries to read the response `R` from the bootloader,
    /// returning `R::T` on success, or`Err` otherwise.
    pub fn send_and_receive<C: Command<Rom>, R: Response<Rom>>(
        &mut self,
        cmd: C,
    ) -> Result<R::T, Error> {
        self.send_command(cmd)?;
        self.read_response::<R>()
    }

    /// Sends the given `command` to the bootloader.
    pub fn send_command<'a, T: Command<Rom>>(&mut self, cmd: T) -> Result<(), Error> {
        if self.last_interaction.elapsed() > BOOTLOADER_TIMEOUT {
            return Err(Error::BootloaderReset {
                port: self.port.into_inner(),
            });
        }

        let mut port = self.port.borrow_mut();

        let mut buf = Vec::new();
        cmd.to_writer(&mut buf)?;

        self.last_interaction = Instant::now();
        port.write_all(&buf)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use assert_hex::*;
    use hex_literal::hex;

    use super::*;

    /// Writes the given command to a `Vec<u8>` and returns the result.
    macro_rules! rom_command_to_vec {
        ($c:expr) => {{
            let mut buf: Vec<u8> = Vec::new();

            $c.to_writer(&mut buf).unwrap();

            buf
        }};
    }

    #[test]
    fn it_should_serialize_get_boot_info_cmd_on_rom() {
        let buf = rom_command_to_vec!(GetBootInfo);

        assert_eq_hex!(&buf, &[0x10, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn it_should_deserialize_boot_rom_info() {
        let input = hex!("4F 4B 14 00 01 00 00 00 00 00 00 00 03 00 00 00 58 9E 02 42 E8 B4 1D 00");
        let boot_info = BootInfo::from_reader(Cursor::new(&input)).unwrap();

        assert_eq!(boot_info.version, 1);
        assert_eq!(
            boot_info.otp_info,
            hex!("00 00 00 00 03 00 00 00 58 9E 02 42 E8 B4 1D 00")
        );
    }

    #[test]
    fn it_should_serialize_load_boot_header() {
        let buf = rom_command_to_vec!(LoadBootHeader {
            boot_header: vec![0x41u8; 176]
        });

        assert_eq_hex!(buf[0], 0x11); // command id
        assert_eq!(u16::from_le_bytes([buf[2], buf[3]]), 176); // command length
        assert_eq_hex!(&buf[0x04..], &[0x41u8; 176]);
    }

    #[test]
    fn it_should_serialize_load_segment_header() {
        let buf = rom_command_to_vec!(LoadSegmentHeader {
            dest_addr: VirtAddr(0x22010000),
            size: 21872,
            reserved: u32::from_le_bytes([0x8f, 0xc3, 0x9d, 0xe9])
        });

        assert_eq!(buf[0], 0x17); // command id
        assert_eq!(buf[0x02..0x04], 16u16.to_le_bytes()); // command length

        // segment dest addr
        assert_eq!(
            u32::from_le_bytes(buf[0x04..0x08].try_into().unwrap()),
            0x22010000
        );
        // segment length
        assert_eq!(buf[0x08..0x0c], 21872u32.to_le_bytes());
        // reserved bytes
        assert_eq!(buf[0x0c..0x10], [0x8f, 0xc3, 0x9d, 0xe9]);
        // crc32 checksum
        assert_eq!(buf[0x10..0x14], 3729223691u32.to_le_bytes());
    }

    #[test]
    fn it_should_serialize_load_segment() {
        let segment_data = &crate::bl::EFLASH_LOADER_40M_BIN[176 + 16..];

        let buf = rom_command_to_vec!(LoadSegment {
            data: segment_data.into()
        });

        assert_eq!(buf[0], 0x18);
        assert_eq!(u16::from_le_bytes([buf[2], buf[3]]), 21872);
        assert_eq!(&buf[0x04..], segment_data);
    }

    #[test]
    fn it_should_serialize_check_image() {
        let buf = rom_command_to_vec!(CheckImage);

        // The command id should be 0x19
        assert_eq!(buf[0], 0x19);
        // The command length should be 0
        assert_eq!(u16::from_le_bytes(buf[0x02..0x04].try_into().unwrap()), 0);
    }

    #[test]
    fn it_should_serialize_run_image() {
        let buf = rom_command_to_vec!(RunImage);

        // The command id should be 0x1a
        assert_eq!(buf[0], 0x1a);
        // The command length should be 0
        assert_eq!(u16::from_le_bytes(buf[0x02..0x04].try_into().unwrap()), 0);
    }
}
