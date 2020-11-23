use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Command {
    /// Get and print the bootrom info
    Info,
}

#[derive(StructOpt, Debug)]
pub struct Opts {
    #[structopt(subcommand)]
    pub command: Command,

    /// The serial device to connect to
    #[structopt(
        env = "SERIAL_PORT",
        short = "d",
        long = "device",
        default_value = "/dev/ttyUSB0"
    )]
    pub serial_port: String,
}
