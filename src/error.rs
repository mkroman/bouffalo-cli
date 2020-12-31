use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Bootloader error: {0}")]
    /// An error ocurred after sending a command to the ROM bootloader
    BootloaderError(u16),

    #[error("The device did not respond to our handshake in the way we expected")]
    /// The device did not respond to our attempt to enter UART bootloader mode
    HandshakeFailed,

    #[error("There was an error when trying to open the serial port: {0}")]
    SerialOpenError(#[from] serialport::Error),

    #[error("The bootloader has reset after 2000ms")]
    /// The bootloader has reset after a timeout
    ///
    /// Note that this *must* be handled when sending commands to the `Bootloader`, and the user is
    /// in charge of taking back the ownership of `port`
    BootloaderReset { port: crate::SerialPort },

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
