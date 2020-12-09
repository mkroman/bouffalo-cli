use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Command {
    /// Get and print the bootrom info
    Info,
    /// Operate on the external flash
    Flash(FlashCommand),
    /// Convert an elf image to a firmware image
    #[structopt(name = "elf2image")]
    Elf2Image(Elf2ImageOpts),
}

#[derive(StructOpt, Debug)]
pub struct Elf2ImageOpts {
    /// The elf filename
    pub filename: PathBuf,
}

#[derive(StructOpt, Debug)]
pub enum FlashCommand {
    /// Read external flash contents
    Read {
        /// Address offset of the flash medium
        #[structopt(required = true)]
        address: u32,
        /// Size of the region to read
        #[structopt(required = true)]
        size: u32,
        /// The name of the file to save the contents to
        #[structopt(required = true, default_value = "flash.bin")]
        filename: PathBuf,
    },
    /// Write external flash contents
    Write {
        /// The name of the file to read from
        #[structopt(required = true)]
        filename: PathBuf,
        /// Address offset of the flash medium
        #[structopt(required = true)]
        address: u32,
        /// Size of the region to write
        #[structopt(required = true)]
        size: u32,
    },
    /// Erase flash contents
    Erase,
}

#[derive(StructOpt, Debug)]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: Command,

    /// The serial device to connect to
    #[structopt(
        env = "SERIAL_PORT",
        short = "p",
        long = "port",
        default_value = "/dev/ttyUSB0"
    )]
    pub serial_port: String,
    /// The serial baud rate to use for the Boot ROM
    #[structopt(
        env = "BAUD_RATE",
        short = "b",
        long = "baud-rate",
        default_value = "500000"
    )]
    pub baud_rate: usize,
}
