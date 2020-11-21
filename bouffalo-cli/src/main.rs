use std::fs::File;
use std::path::Path;

use anyhow::Context;

mod elf_parser;
mod error;

pub use error::Error;

fn elf2image<P: AsRef<Path>>(input_path: P) -> Result<(), anyhow::Error> {
    let file = File::open(&input_path)?;
    let mut parser = elf_parser::ElfParser::new(file);
    let header = parser.parse_header().with_context(|| {
        format!(
            "Failed to parse header of ELF file '{}'",
            input_path.as_ref().display()
        )
    })?;

    println!("ELF header: {:?}", header);

    // Read the program headers
    println!("Program headers:");

    for n in 0..header.ph_entry_num {
        let off = header.ph_offset as u64 + (0x20 * n as u64);

        let program_header = parser.parse_program_header(off)?;

        println!("{:#x} {:?}", off, program_header);
    }

    println!("Section headers:");

    for n in 0..header.sh_entry_num {
        let off = header.sh_offset as u64 + (0x28 * n as u64);
        let section_header = parser.parse_section_header(off)?;

        println!("{:#x} {:?}", off, section_header);
    }

    Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = std::env::args().collect();

    match args[1].as_str() {
        "elf2image" => {
            println!("elf2image {}", args[2]);

            let image = elf2image(&args[2])?;
        }
        _ => println!("Usage: {} elf2image <file.elf>", args[0]),
    }

    Ok(())
}
