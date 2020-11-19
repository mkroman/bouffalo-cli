use std::convert::TryInto;
use std::io::{self, BufReader, Read, Seek, SeekFrom};

use thiserror::Error;

/// This is a simple ELF64 file parser that makes it easy to extract sections
#[derive(Debug)]
pub struct ElfParser<R> {
    reader: BufReader<R>,
}

/// This is an ELF32 header
#[derive(Debug)]
pub struct Header {
    /// This byte is set to either 1 or 2 to signify 32- or 64-bit format, respectively
    class: Class,
    /// The endianness of the file
    endianness: Endianness,
    /// The ELF file version
    version: u8,
    /// The target OS ABI
    os_abi: u8,
    /// The target OS ABI version
    os_abi_version: u8,
    /// The object file type
    file_type: u16,
    /// The program entry address
    entry_addr: u32,
    /// The program header offset
    ph_offset: u32,
    /// The section header offset
    sh_offset: u32,
    /// The size of a program header entry
    ph_entry_size: u16,
    /// The number of program header entries
    ph_entry_num: u16,
    /// The size of a section header entry
    sh_entry_size: u16,
    /// The number of section header entries
    sh_entry_num: u16,
    /// The index of the section header that contains the names for the sections
    sh_str_idx: u16,
}

/// ELF32 Program Header
#[derive(Debug)]
pub struct ProgramHeader {
    /// The type of the program header segment
    typ: u32,
    /// The offset to the segment in the image file
    offset: u32,
    /// The virtual address to map the segment to
    virt_addr: u32,
    /// The physical address to map the segment to, when relevant
    phys_addr: u32,
    /// Size of the segment in the file image, in bytes
    file_size: u32,
    /// Size of the segment in memory, in bytes
    mem_size: u32,
    /// Segment-dependent flags
    flags: u32,
    /// How to align the section
    ///
    /// 0 and 1 specify no alignment
    ///
    /// Otherwise should be a positive, integral power of 2, with `virt_addr` equating `offset`
    /// modulus `alignment`
    alignment: u32,
}

/// ELF32 Section Header
#[derive(Debug)]
pub struct SectionHeader {
    /// Offset to a string in the .shstrtab section with the name of this section
    name_offset: u32,
    /// The type of this section
    typ: u32,
    /// The attributes of this section
    flags: u32,
    /// Virtual address for this section, if it's to be loaded into memory
    virt_addr: u32,
    /// Offset to the section in the file image
    offset: u32,
    /// The size of the section in the file image, in bytes
    size: u32,
    /// Contains the index of an associated section, which might be used depending on the type
    link: u32,
    /// Contains information about the section
    info: u32,
    /// The required alignment of the section
    addr_align: u32,
    /// The size of each entry, in bytes, if this is a section with fixed sized data
    entry_size: u32,
}

#[derive(Debug)]
pub enum Class {
    Elf32,
}

#[derive(Debug)]
pub enum Endianness {
    Little,
}

/// Errors that indicate what went wrong during parsing
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Missing ELF header")]
    MissingHeader,
    #[error("Input does not contain ELF magic header")]
    InvalidMagicHeader,
    #[error("Input ELF is 64-bit, only 32-bit is supported")]
    ElfIs64Bit,
    #[error("Input has an unsupported ELF version, expected 1")]
    InvalidElfVersion,
    #[error("Input endianness is unsupported, only little endian is supported")]
    UnsupportedEndianness,
    #[error("Input ABI is unsupported, only System V is supported")]
    UnsupportedAbi,
    #[error("Input has an unsupported machine type, only RISC-V is supported")]
    UnsupportedMachineType,
    #[error("Input is an unsupported file type, only executable files are supported")]
    UnsupportedFileType,
    #[error("There was an internal error when converting fields")]
    ConversionError,
    #[error("I/O error: {}", _0)]
    IoError(#[from] io::Error),
}

impl<R: Read + Seek> ElfParser<R> {
    pub fn new(reader: R) -> ElfParser<R> {
        let reader = BufReader::new(reader);

        ElfParser { reader }
    }

    pub fn parse_header(&mut self) -> Result<Header, ParseError> {
        // Seek to the beginning of the file
        self.reader.seek(SeekFrom::Start(0))?;

        let mut header = [0u8; 64];

        // Read the first 64 bytes of the input into the `header` buffer
        self.reader
            .read_exact(&mut header)
            .map_err(|_| ParseError::MissingHeader)?;

        // Ensure that the header starts with the magic value
        if header[0x0..0x4] != [0x7f, 0x45, 0x4c, 0x46] {
            return Err(ParseError::InvalidMagicHeader);
        }

        // Read the target class, either 32-bit or 64-bit
        let class = match header[0x4] {
            1 => Class::Elf32,
            _ => return Err(ParseError::ElfIs64Bit),
        };

        // Read the ELF endianness
        let endianness = match header[0x5] {
            1 => Endianness::Little,
            _ => return Err(ParseError::UnsupportedEndianness),
        };

        // Read the ELF version and assert that it is 1
        let version = header[0x6];

        if version != 1 {
            return Err(ParseError::InvalidElfVersion);
        }

        // Read the OS ABI and assert that it is 0 (UNIX - System V)
        let os_abi = header[0x7];

        if os_abi != 0 {
            return Err(ParseError::UnsupportedAbi);
        }

        // Read the OS ABI version
        let os_abi_version = header[0x8];

        // Read the object file type
        let file_type = u16::from_le_bytes(
            header[0x10..0x12]
                .try_into()
                .map_err(|_| ParseError::ConversionError)?,
        );

        // Assert that the file type is an executable file
        if file_type != 0x02 {
            return Err(ParseError::UnsupportedFileType);
        }

        // Read the machine type
        let machine_type = u16::from_le_bytes(header[0x12..0x14].try_into().unwrap());

        // Assert that the machine type is RISC-V
        if machine_type != 0xF3 {
            return Err(ParseError::UnsupportedMachineType);
        }

        // Read the entry address
        let entry_addr = u32::from_le_bytes(header[0x18..0x1c].try_into().unwrap());

        // Read the program header offset
        let ph_offset = u32::from_le_bytes(header[0x1c..0x20].try_into().unwrap());

        // Read the section header offset
        let sh_offset = u32::from_le_bytes(header[0x20..0x24].try_into().unwrap());

        // Read the size of the program header entries
        let ph_entry_size = u16::from_le_bytes(header[0x2a..0x2c].try_into().unwrap());

        // Read the number of program header entries
        let ph_entry_num = u16::from_le_bytes(header[0x2c..0x2e].try_into().unwrap());

        // Read the size of the section header entries
        let sh_entry_size = u16::from_le_bytes(header[0x2e..0x30].try_into().unwrap());

        // Read the number of section header entries
        let sh_entry_num = u16::from_le_bytes(header[0x30..0x32].try_into().unwrap());

        // Read the index of the section header that contains the name of the sections
        let sh_str_idx = u16::from_le_bytes(header[0x32..0x34].try_into().unwrap());

        let header = Header {
            class,
            endianness,
            version,
            os_abi,
            os_abi_version,
            file_type,
            entry_addr,
            ph_offset,
            sh_offset,
            ph_entry_size,
            ph_entry_num,
            sh_entry_size,
            sh_entry_num,
            sh_str_idx,
        };

        Ok(header)
    }
}
