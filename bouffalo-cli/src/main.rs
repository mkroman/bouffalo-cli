use std::fs::File;
use std::path::Path;
use std::thread;
use std::time::Duration;

use anyhow::Context;

mod bl;
mod bl60x;
mod elf_parser;
mod error;

use bl::Firmware;
pub use error::Error;

fn get_info(port: &str) -> Result<(), anyhow::Error> {
    // Open a serial port to the blx602 device
    let mut port = bl60x::Bl60xSerialPort::open(port)?;

    // Put the BootROM into UART mode
    port.enter_uart_mode()?;

    // Wait for 20ms
    thread::sleep(Duration::from_millis(20));

    // Send get_boot_info command
    let boot_info = port.get_boot_info()?;

    println!("Boot info: {:#?}", boot_info);

    Ok(())
}

fn elf2image<P: AsRef<Path>>(input_path: P) -> Result<(), anyhow::Error> {
    let file = File::open(&input_path)?;
    let parser = elf_parser::ElfParser::parse(file).with_context(|| {
        format!(
            "Failed to parse header of ELF file '{}'",
            input_path.as_ref().display()
        )
    })?;

    let fw = Firmware::builder()
        .entry_point(0x1337)
        .build()
        .with_context(|| "Failed to build firmware image")?;

    println!("ELF header: {:?}", parser);
    println!("Firmware: {:?}", fw);

    Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = std::env::args().collect();

    match args[1].as_str() {
        "elf2image" => {
            println!("elf2image {}", args[2]);

            let _image = elf2image(&args[2])?;
        }
        "info" => {
            let port = args.get(2).map(|s| s.as_str()).unwrap_or("/dev/ttyUSB0");

            get_info(port)?;
        }
        _ => println!("Usage: {} elf2image <file.elf>", args[0]),
    }

    Ok(())
}
