use std::io::{Cursor, Read, Write};

use byteorder::{LittleEndian, ReadBytesExt};

use crate::Error;

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

/// Status response that implements `Response<Rom>`.
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

/// ROM bootloader command to retrieve the bootloader version and OTP bit flags.
struct GetBootInfo;

impl Command<Rom> for GetBootInfo {
    fn to_writer<W: Write>(&self, mut writer: W) -> Result<(), Error> {
        writer.write_all(&[0x10, 0x00, 0x00, 0x00])?;

        Ok(())
    }
}

/// The return type of the GetBootInfo command when successful.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct BootInfo {
    /// The version of the bootloader in ROM
    pub version: u32,
    /// OTP flags
    pub otp_info: [u8; 16],
}

impl Response<Rom> for BootInfo {
    type T = Self;

    fn from_reader<R: Read + ReadBytesExt>(mut reader: R) -> Result<Self, Error> {
        Status::from_reader(&mut reader)?;

        let mut buf = [0u8; 22];
        reader.read_exact(&mut buf)?;

        let mut cursor = Cursor::new(buf);
        let length = cursor.read_u16::<LittleEndian>()?;

        // The length should be exactly 20 bytes
        assert_eq!(length, 20);

        // Read the bootloader version
        let version = cursor.read_u32::<LittleEndian>()?;

        // Read the OTP flags
        let mut otp_info = [0u8; 16];
        cursor.read_exact(&mut otp_info)?;

        Ok(BootInfo { version, otp_info })
    }
}

#[cfg(test)]
mod tests {
    use assert_hex::*;
    use hex_literal::hex;

    use super::*;

    /// Writes the given `command` to a Vec<u8> and returns it
    fn rom_command_to_vec<C: Command<Rom>>(command: C) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();

        command.to_writer(&mut buf).unwrap();

        buf
    }

    #[test]
    fn it_should_serialize_get_boot_info_cmd_on_rom() {
        let buf = rom_command_to_vec(GetBootInfo);

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
}
