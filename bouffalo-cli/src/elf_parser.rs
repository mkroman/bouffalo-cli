use std::convert::TryInto;
use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};

use thiserror::Error;

/// This is a simple ELF64 file parser that makes it easy to extract sections
#[derive(Debug)]
pub struct ElfParser<R> {
    reader: BufReader<R>,
    header: Header,
    program_headers: Vec<ProgramHeader>,
    section_headers: Vec<SectionHeader>,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum ProgType {
    Null = 0x0,
    Load,
    Dynamic,
    Interp,
    Note,
    ShLib,
    PHdr,
    Tls,
    GnuEhFrame = 0x6474e550,
    GnuStack = 0x6474e551,
    GnuRelRo = 0x6474e552,
}

impl From<u32> for ProgType {
    fn from(val: u32) -> ProgType {
        match val {
            0 => ProgType::Null,
            1 => ProgType::Load,
            2 => ProgType::Dynamic,
            3 => ProgType::Interp,
            4 => ProgType::Note,
            5 => ProgType::ShLib,
            6 => ProgType::PHdr,
            7 => ProgType::Tls,
            0x6474e550 => ProgType::GnuEhFrame,
            0x6474e551 => ProgType::GnuStack,
            0x6474e552 => ProgType::GnuRelRo,
            _ => ProgType::Null,
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum SectionType {
    Null = 0x0,
    ProgBits,
    SymTab,
    StrTab,
    RelA,
    Hash,
    Dynamic,
    Note,
    NoBits,
    Rel,
    ShLib,
    DynSym,
    InitArray = 0xe,
    FiniArray = 0xf,
    PreInitArray,
    Group,
    SymTabShNdx,
    Num,
    // Sometimes called ARM_ATTRIBUTES, other times RISCV_ATTRIBUTES
    CompatAttribute = 0x70000003,
}

impl From<u32> for SectionType {
    fn from(val: u32) -> SectionType {
        match val {
            0x00 => SectionType::Null,
            0x01 => SectionType::ProgBits,
            0x02 => SectionType::SymTab,
            0x03 => SectionType::StrTab,
            0x04 => SectionType::RelA,
            0x05 => SectionType::Hash,
            0x06 => SectionType::Dynamic,
            0x07 => SectionType::Note,
            0x08 => SectionType::NoBits,
            0x09 => SectionType::Rel,
            0x0a => SectionType::ShLib,
            0x0b => SectionType::DynSym,
            0x0e => SectionType::InitArray,
            0x0f => SectionType::FiniArray,
            0x10 => SectionType::PreInitArray,
            0x11 => SectionType::Group,
            0x12 => SectionType::SymTabShNdx,
            0x13 => SectionType::Num,
            0x70000003 => SectionType::CompatAttribute,
            _ => panic!("Unrecognized section type {:#x}", val),
        }
    }
}

impl fmt::Debug for SectionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SectionType::Null => "NULL",
            SectionType::ProgBits => "PROGBITS",
            SectionType::SymTab => "SYMTAB",
            SectionType::StrTab => "STRTAB",
            SectionType::RelA => "RELA",
            SectionType::Hash => "HASH",
            SectionType::Dynamic => "DYNAMIC",
            SectionType::Note => "NOTE",
            SectionType::NoBits => "NOBITS",
            SectionType::Rel => "REL",
            SectionType::ShLib => "SHLIB",
            SectionType::DynSym => "DYNSYM",
            SectionType::InitArray => "INIT_ARRAY",
            SectionType::FiniArray => "FINI_ARRAY",
            SectionType::PreInitArray => "PREINIT_ARRAY",
            SectionType::Group => "GROUP",
            SectionType::SymTabShNdx => "SYMTAB_SHNDX",
            SectionType::Num => "NUM",
            SectionType::CompatAttribute => "RISCV_ATTRIBUTE",
        };

        write!(f, "{}", s)
    }
}

/// This is an ELF32 header
#[derive(Debug)]
pub struct Header {
    /// This byte is set to either 1 or 2 to signify 32- or 64-bit format, respectively
    pub class: Class,
    /// The endianness of the file
    pub endianness: Endianness,
    /// The ELF file version
    pub version: u8,
    /// The target OS ABI
    pub os_abi: u8,
    /// The target OS ABI version
    pub os_abi_version: u8,
    /// The object file type
    pub file_type: u16,
    /// The program entry address
    pub entry_addr: u32,
    /// The program header offset
    pub ph_offset: u32,
    /// The section header offset
    pub sh_offset: u32,
    /// The size of a program header entry
    pub ph_entry_size: u16,
    /// The number of program header entries
    pub ph_entry_num: u16,
    /// The size of a section header entry
    pub sh_entry_size: u16,
    /// The number of section header entries
    pub sh_entry_num: u16,
    /// The index of the section header that contains the names for the sections
    pub sh_str_idx: u16,
}

/// ELF32 Program Header
#[derive(Debug)]
pub struct ProgramHeader {
    /// The type of the program header segment
    typ: ProgType,
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
    pub name_offset: u32,
    /// The type of this section
    pub typ: SectionType,
    /// The attributes of this section
    pub flags: u32,
    /// Virtual address for this section, if it's to be loaded into memory
    pub virt_addr: u32,
    /// Offset to the section in the file image
    pub offset: u32,
    /// The size of the section in the file image, in bytes
    pub size: u32,
    /// Contains the index of an associated section, which might be used depending on the type
    pub link: u32,
    /// Contains information about the section
    pub info: u32,
    /// The required alignment of the section
    pub addr_align: u32,
    /// The size of each entry, in bytes, if this is a section with fixed sized data
    pub entry_size: u32,
    /// The name of the section
    pub name: Option<String>,
}

/// The target machine class
#[derive(Debug)]
pub enum Class {
    Elf32,
}

/// Indicates the elf and target endianness
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
    #[error("There was an error when trying to parse the section name as utf-8")]
    SectionNameEncodingError(#[from] std::string::FromUtf8Error),
    #[error("I/O error: {}", _0)]
    IoError(#[from] io::Error),
}

impl<R: Read + Seek> ElfParser<R> {
    /// Parses ELF file-, program- and section headers from the given `reader` input and returns an
    /// `ElfParser` that retains ownership in order to read further section data
    pub fn parse(reader: R) -> Result<ElfParser<R>, ParseError> {
        let mut reader = BufReader::new(reader);
        let header = Self::parse_header(&mut reader)?;
        let mut program_headers = Vec::with_capacity(header.ph_entry_num as usize);
        let mut section_headers = Vec::with_capacity(header.sh_entry_num as usize);

        // Read the program headers
        for n in 0..header.ph_entry_num {
            let offset = header.ph_offset as u64 + (header.ph_entry_size as u64 * n as u64);
            let program_header = Self::parse_program_header(&mut reader, offset)?;

            program_headers.push(program_header);
        }

        // Read the section headers
        for n in 0..header.sh_entry_num {
            let offset = header.sh_offset as u64 + (header.sh_entry_size as u64 * n as u64);
            let section_header = Self::parse_section_header(&mut reader, offset)?;

            section_headers.push(section_header);
        }

        let mut strbuf: Vec<u8> = Vec::new();
        let str_table_offset = section_headers[header.sh_str_idx as usize].offset as u64;

        // Read the section names
        for sh in section_headers.iter_mut() {
            strbuf.clear();
            reader.seek(SeekFrom::Start(str_table_offset + sh.name_offset as u64))?;
            reader.read_until(0x00, &mut strbuf)?;

            sh.name = Some(String::from_utf8_lossy(&strbuf[..strbuf.len() - 1]).to_string());
        }

        Ok(ElfParser {
            reader,
            header,
            program_headers,
            section_headers,
        })
    }

    /// Parses and returns the Program Header at the given `offset` from the beginning of the input
    fn parse_program_header(
        reader: &mut BufReader<R>,
        offset: u64,
    ) -> Result<ProgramHeader, ParseError> {
        reader.seek(SeekFrom::Start(offset))?;

        let mut buffer = [0u8; 32];

        reader.read_exact(&mut buffer)?;

        let typ = u32::from_le_bytes(buffer[0x00..0x04].try_into().unwrap());
        let offset = u32::from_le_bytes(buffer[0x04..0x08].try_into().unwrap());
        let virt_addr = u32::from_le_bytes(buffer[0x08..0x0c].try_into().unwrap());
        let phys_addr = u32::from_le_bytes(buffer[0x0c..0x10].try_into().unwrap());
        let file_size = u32::from_le_bytes(buffer[0x10..0x14].try_into().unwrap());
        let mem_size = u32::from_le_bytes(buffer[0x14..0x18].try_into().unwrap());
        let flags = u32::from_le_bytes(buffer[0x18..0x1c].try_into().unwrap());
        let alignment = u32::from_le_bytes(buffer[0x1c..0x20].try_into().unwrap());

        Ok(ProgramHeader {
            typ: typ.into(),
            offset,
            virt_addr,
            phys_addr,
            file_size,
            mem_size,
            flags,
            alignment,
        })
    }

    /// Parses and returns the section header at `offset`
    pub fn parse_section_header(
        reader: &mut BufReader<R>,
        offset: u64,
    ) -> Result<SectionHeader, ParseError> {
        reader.seek(SeekFrom::Start(offset))?;

        let mut buffer = [0u8; 40];

        reader.read_exact(&mut buffer)?;

        let name_offset = u32::from_le_bytes(buffer[0x00..0x04].try_into().unwrap());
        let typ = u32::from_le_bytes(buffer[0x04..0x08].try_into().unwrap());
        let flags = u32::from_le_bytes(buffer[0x08..0x0c].try_into().unwrap());
        let virt_addr = u32::from_le_bytes(buffer[0x0c..0x10].try_into().unwrap());
        let offset = u32::from_le_bytes(buffer[0x10..0x14].try_into().unwrap());
        let size = u32::from_le_bytes(buffer[0x14..0x18].try_into().unwrap());
        let link = u32::from_le_bytes(buffer[0x18..0x1c].try_into().unwrap());
        let info = u32::from_le_bytes(buffer[0x1c..0x20].try_into().unwrap());
        let addr_align = u32::from_le_bytes(buffer[0x20..0x24].try_into().unwrap());
        let entry_size = u32::from_le_bytes(buffer[0x24..0x28].try_into().unwrap());

        Ok(SectionHeader {
            name_offset,
            typ: typ.into(),
            flags,
            virt_addr,
            offset,
            size,
            link,
            info,
            addr_align,
            entry_size,
            name: None,
        })
    }

    /// Parses and returns an ELF32 file header at the current position of the reader
    ///
    /// Note: It is up to the user to ensure that the reader is at the beginning of the input
    fn parse_header(reader: &mut BufReader<R>) -> Result<Header, ParseError> {
        // Read the first 64 bytes of the input into the `header` buffer
        let mut header = [0u8; 64];

        reader
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
        let file_type = u16::from_le_bytes(header[0x10..0x12].try_into().unwrap());

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
