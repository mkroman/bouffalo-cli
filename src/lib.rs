pub mod bl;
pub mod bl60x;
mod error;
pub mod isp;

use std::cell::RefCell;
use std::ffi::OsStr;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant};

pub use error::Error;

pub use isp::Bootloader;

use log::trace;
pub use serialport;
use serialport::prelude::*;

/// Provides static typing for a virtual address on the target platform.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct VirtAddr(u32);

/// Serial connection with an open serial port.
pub struct SerialPort {
    inner_port: Box<dyn serialport::SerialPort>,
}

impl fmt::Debug for SerialPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SerialPort")
            .field("name", &self.name())
            .field("settings", &self.settings())
            .finish()
    }
}

impl Deref for SerialPort {
    type Target = Box<dyn serialport::SerialPort>;

    fn deref(&self) -> &Self::Target {
        &self.inner_port
    }
}

impl DerefMut for SerialPort {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner_port
    }
}

impl SerialPort {
    /// Opens the given `port` as a `SerialPort` with the given `baud_rate`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use bouffalo::SerialPort;
    ///
    /// let port = SerialPort::open("/dev/ttyUSB0", 500_000)?;
    ///
    /// # Ok::<(), bouffalo::Error>(())
    /// ```
    pub fn open<S: AsRef<OsStr>>(port: S, baud_rate: u32) -> Result<SerialPort, Error> {
        let settings = SerialPortSettings {
            baud_rate,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_millis(2000),
        };

        let serial_port = serialport::open_with_settings(port.as_ref(), &settings)
            .map_err(Error::SerialOpenError)?;

        Ok(SerialPort {
            inner_port: serial_port,
        })
    }

    /// Consumes `self` and returns the inner serial port.
    pub fn into_port(self) -> Box<dyn serialport::SerialPort> {
        self.inner_port
    }

    /// Attempt to put the device into the UART bootloader mode and return `Bootloader` on success.
    ///
    /// On failure, this will return either a serial timeout error, or `Error::HandshakeFailed` if
    /// the device did not respond as we expected when sending a handshake.
    pub fn enter_bootloader(mut self) -> Result<Bootloader, Error> {
        let mut buf = [0u8; 2];

        // Calculate the number of bytes to send in order to to keep the UART busy for 5ms
        // bauds * 3s / (8 data bits + 1 start bit + 1 stop bit) / 1000 ms
        let num_bytes = self
            .inner_port
            .baud_rate()
            .expect("Could not get baud rate for serial port")
            .saturating_mul(3)
            / 10
            / 1000;

        trace!("Trying to put device in UART mode");
        trace!("Sending {} x 0x55 bytes", num_bytes);

        self.inner_port
            .write_all(&vec![0x55u8; num_bytes as usize])?;
        self.inner_port.read_exact(&mut buf)?;

        if &buf != b"OK" {
            return Err(Error::HandshakeFailed);
        }

        trace!("Device successfully entered UART mode");

        Ok(Bootloader {
            last_interaction: Instant::now(),
            port: RefCell::new(self),
        })
    }
}
