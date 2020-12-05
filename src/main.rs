use std::convert::TryInto;
use std::fs::File;
use std::io::Cursor;
use std::path::Path;
use std::thread;
use std::time::Duration;

use anyhow::Context;
use log::{debug, warn};
use structopt::StructOpt;

mod bl;
mod bl60x;
mod cli;
mod elf_parser;
mod error;

use bl::Firmware;
pub use error::Error;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct VirtAddr(u32);

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

fn load_flasher(port: &str) -> Result<bl60x::Bl60xSerialPort, anyhow::Error> {
    // Open a serial port to the blx602 device
    let mut port = bl60x::Bl60xSerialPort::open(port)?;

    // Put the BootROM into UART mode
    port.enter_uart_mode()?;

    // Wait for 20ms
    thread::sleep(Duration::from_millis(20));

    // Get the boot info
    let boot_info = port.get_boot_info()?;

    // Parse the eflash_loader firmware
    let fw = Firmware::from_reader(Cursor::new(&bl::EFLASH_LOADER_38P4M_BIN))?;

    // Write the boot header into our buffer
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    fw.write_to(&mut buf)?;

    // Send the boot header
    match port.load_boot_header(buf.try_into().unwrap()) {
        Ok(res) => {
            // println!("res: {:?}", res);
        }
        Err(err) => {
            warn!(
                "Error when trying to read response from load boot header: {}",
                err
            );
        }
    };

    // Load the firmware segments
    for segment in fw.segments {
        port.load_segment(&segment)?;
    }

    port.check_image()?;
    port.run_image()?;

    Ok(port)
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

fn flash_command(
    command: &cli::FlashCommand,
    global_opts: &cli::Opts,
) -> Result<(), anyhow::Error> {
    use cli::FlashCommand;

    println!("Using serial device {:?}", &global_opts.serial_port);

    match command {
        FlashCommand::Read {
            address,
            size,
            filename,
            ..
        } => {
            println!(
                "Reading flash at {:#010x} of size {} and writing it to {}",
                address,
                size,
                filename.display()
            );

            load_flasher(&global_opts.serial_port)?;

            debug!("Reconfiguring serial port, this time in the eflasher context");
            let mut port = bl60x::Bl60xSerialPort::open(&global_opts.serial_port)?;

            // Open a serial port to the blx602 device
            //port.set_baud_rate(serial::BaudRate::BaudOther(500_000))?;

            thread::sleep(Duration::from_millis(4000));

            // Put the BootROM into UART mode
            port.enter_uart_mode()?;

            println!("Entered UART mode");

            // Wait for 20ms
            thread::sleep(Duration::from_millis(200));

            // Get the boot info
            let boot_info = port.get_boot_info()?;

            println!("BootROM version: {}", boot_info.rom_version);
        }
        _ => {}
    }

    Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    use cli::{Command, Elf2ImageOpts};

    // Create a logger with a timestamp that logs everything at Info level or above
    pretty_env_logger::init_timed();

    // Parse the command-line arguments
    let opts = cli::Opts::from_args();

    match &opts.command {
        Command::Info => {
            let serial_port = opts.serial_port;

            get_boot_info(&serial_port)?;
        }
        Command::Flash(ref cmd) => flash_command(cmd, &opts)?,
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
