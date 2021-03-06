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
        size: Option<u32>,
    },
    /// Erase flash contents
    Erase {
        /// The offset in flash to start erasing from, starting from 0
        #[structopt(required = true)]
        offset: u32,
        /// The number of bytes to erase
        #[structopt(required = true)]
        size: u32,
    },
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

    /// The serial baud rate to use when communicating with the Boot ROM
    #[structopt(
        env = "BAUD_RATE",
        short = "b",
        long = "baud-rate",
        default_value = "500000"
    )]
    pub baud_rate: usize,

    #[structopt(long = "programming-baud-rate", default_value = "500000")]
    pub programming_baud_rate: usize,
}
