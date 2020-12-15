use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    /// An error ocurred after sending a command to the ROM bootloader
    #[error("Bootloader error: {0}")]
    BootloaderError(u16),

    /// An I/O error occurred
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Error)]
pub enum SerialError {
    #[error("Error when opening serial port: {}", _0)]
    OpenError(String, serialport::Error),
    #[error("Error when trying to set serial port baud rate to {}: {}", _0, _1)]
    BaudError(usize, serialport::Error),
    #[error("Error when setting serial timeout to {}: {}", _0, _1)]
    TimeoutError(u64, serialport::Error),
}
