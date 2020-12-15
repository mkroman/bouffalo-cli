use std::convert::TryInto;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Context};
use log::{debug, error};
use sha2::{Digest, Sha256};
use structopt::StructOpt;

mod bl;
mod bl60x;
mod cli;
mod elf_parser;
mod error;

use bl::Firmware;
use bl60x::Bl60xSerialPort;
pub use error::SerialError;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct VirtAddr(u32);

fn get_boot_info(global_opts: &cli::Opts) -> Result<(), anyhow::Error> {
    let serial_port = &global_opts.serial_port;
    let baud_rate = global_opts.baud_rate;

    println!("Using serial device {:?}", &serial_port);

    // Open a serial port to the blx602 device
    let mut port = bl60x::Bl60xSerialPort::open_with_baud_rate(serial_port, baud_rate)
        .with_context(|| "Could not open serial port")?;

    // Put the BootROM into UART mode
    port.enter_uart_mode()
        .with_context(|| "Could not enter BootROM UART mode")?;

    // Wait for 20ms
    thread::sleep(Duration::from_millis(20));

    // Send get_boot_info command
    let boot_info = port
        .get_boot_info()
        .with_context(|| "Could not get boot info")?;

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

///
fn load_flasher(port: &mut Bl60xSerialPort) -> Result<(), anyhow::Error> {
    // Put the BootROM into UART mode
    port.enter_uart_mode()?;

    // Wait for 20ms
    thread::sleep(Duration::from_millis(20));

    // Parse the eflash_loader firmware
    let fw = Firmware::from_reader(Cursor::new(&bl::EFLASH_LOADER_40M_BIN))?;

    // Write the boot header into our buffer
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    fw.write_to(&mut buf)?;

    // Send the boot header
    match port.load_boot_header(buf.try_into().unwrap()) {
        Ok(_) => {}
        Err(err) => {
            error!(
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

fn flash_command(
    command: &cli::FlashCommand,
    global_opts: &cli::Opts,
) -> Result<(), anyhow::Error> {
    use cli::FlashCommand;

    println!("Using serial device {:?}", &global_opts.serial_port);

    // Open the serial port
    let mut port = bl60x::Bl60xSerialPort::open_with_baud_rate(
        &global_opts.serial_port,
        global_opts.baud_rate,
    )?;

    // Load fhe eflash firmware into RAM and run it
    load_flasher(&mut port)?;

    // Wait for 100 ms
    std::thread::sleep(Duration::from_millis(100));

    // Change the baud rate
    port.set_baud_rate(serial::BaudRate::from_speed(
        global_opts.programming_baud_rate,
    ))?;

    // Put the BootROM into UART mode
    port.enter_uart_mode()?;

    // Wait for 20ms
    thread::sleep(Duration::from_millis(20));

    match command {
        FlashCommand::Read {
            address,
            size,
            filename,
            ..
        } => {
            println!(
                "Reading {} bytes from flash at {:#010x} and writing it to file {}",
                size,
                address,
                filename.display()
            );

            const READ_SIZE: usize = 8192;

            let mut buf = vec![0u8; READ_SIZE];
            let mut file = File::create(filename)?;
            let mut hasher = Sha256::new();

            let rem = *size as usize % READ_SIZE;
            let div = *size as usize / READ_SIZE;
            let div = if rem > 0 { div + 1 } else { div };
            let mut remaining = *size as usize;

            for n in 0..div {
                let s = std::cmp::min(remaining, READ_SIZE);
                let start = n * READ_SIZE;
                let mut b = &mut buf[0..s];

                port.read_flash_exact(start as u32, &mut b)?;

                hasher.update(&b);
                file.write_all(&b)?;
                remaining -= s;
            }

            // Have the device calculate the sha256 hash for the flash regions we requested
            let flash_hash = port.flash_sha256(*address, *size)?;
            // Calculate the final sha256 hash for the data we just read
            let read_hash = hasher.finalize();

            // Compare and ensure that the data we just read matches what's on the flash
            if flash_hash[..] != read_hash[..] {
                error!("SHA256 hash mismatch between the flash data and the data we just read");
            } else {
                debug!("SHA256 hash between flash and the data we just read matches");
            }
        }
        FlashCommand::Write {
            filename,
            address,
            size,
        } => {
            let file = File::open(filename)
                .with_context(|| "Could not open the file we wanted to write to flash")?;
            let file_size = file
                .metadata()
                .with_context(|| {
                    "Could not read metadata for the file we wanted to write to flash"
                })?
                .len();
            let size = size.unwrap_or_else(|| file_size.try_into().unwrap());

            assert!(size as u64 <= file_size);

            println!(
                "Writing {} bytes to flash at {:#010x} from the file {}",
                size,
                address,
                filename.display()
            );

            // Read the contents of the file into memory
            let mut buf = vec![0u8; size as usize];
            let mut reader = BufReader::new(file);

            reader.read_exact(&mut buf)?;

            port.set_timeout(Duration::from_secs(60))?;
            port.write_flash(*address, &buf)?;
            port.set_timeout(Duration::from_secs(2))?;
        }
        FlashCommand::Erase { offset, size } => {
            if size % 4096 > 0 {
                return Err(anyhow!("The erase size must be a multiple of 4096, since data is erased in entire sections"));
            }

            println!("Erasing {} bytes from flash at 0x{:08x}", size, offset);

            // Increase the timeout duration since this can take a while
            port.set_timeout(Duration::from_secs(60))?;
            port.erase_flash(*offset, *size)?;
            // Restore the timeout duration
            port.set_timeout(Duration::from_secs(2))?;
        }
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
        Command::Info => get_boot_info(&opts)?,
        Command::Flash(ref cmd) => flash_command(cmd, &opts)?,
        Command::Elf2Image(Elf2ImageOpts { filename }) => {
            println!(
                "Converting elf image {} to firmware",
                filename.as_path().display()
            );

            elf2image(filename)?;
        }
    }

    Ok(())
}
