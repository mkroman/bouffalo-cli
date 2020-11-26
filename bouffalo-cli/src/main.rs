use std::fs::File;
use std::path::Path;
use std::thread;
use std::time::Duration;

use anyhow::Context;
use structopt::StructOpt;

mod bl;
mod bl60x;
mod cli;
mod elf_parser;
mod error;

use bl::Firmware;
pub use error::Error;

fn get_boot_info(port: &str) -> Result<(), anyhow::Error> {
    println!("Using serial device {:?}", port);

    // Open a serial port to the blx602 device
    let mut port = bl60x::Bl60xSerialPort::open(port)?;

    // Put the BootROM into UART mode
    port.enter_uart_mode()?;

    // Wait for 20ms
    thread::sleep(Duration::from_millis(20));

    // Send get_boot_info command
    let boot_info = port.get_boot_info()?;

    println!("BootROM version: {}", boot_info.rom_version);
    println!("OTP flags:");

    // Print the individual bits of the OTP flags over multiple lines
    let otp_bit_strs: Vec<String> = boot_info
        .otp_info
        .iter()
        .map(|x| format!("{:08b}", x))
        .collect();

    for row in 0..otp_bit_strs.len() / 4 {
        println!(
            "  {} {} {} {}",
            otp_bit_strs[row * 4],
            otp_bit_strs[1 + row * 4],
            otp_bit_strs[2 + row * 4],
            otp_bit_strs[3 + row * 4]
        );
    }

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
    use cli::{Command, Elf2ImageOpts, FlashCommand, FlashReadOpts};

    // Create a logger with a timestamp that logs everything at Info level or above
    pretty_env_logger::init_timed();

    // Parse the command-line arguments
    let opts = cli::Opts::from_args();

    match &opts.command {
        Command::Info => {
            let serial_port = opts.serial_port;

            get_boot_info(&serial_port)?;
        }
        Command::Flash(FlashCommand::Read(FlashReadOpts {
            address,
            size,
            filename,
        })) => {
            println!(
                "Reading flash at {:#010x} of size {} to file {}",
                address,
                size,
                filename.as_path().display()
            );
        }
        Command::Elf2Image(Elf2ImageOpts { filename }) => {
            println!(
                "Converting elf image {} to firmware",
                filename.as_path().display()
            );

            elf2image(filename)?;
        }
        _ => {}
    }

    Ok(())
}
