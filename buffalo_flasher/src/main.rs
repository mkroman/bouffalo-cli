// This initial flashing tool was implemented by reading the source of bl60x-flash:
// https://github.com/stschake/bl60x-flash/blob/065770004629c3e5bf98057677e7a6ca566e9c4a/bl60x_flash/main.py

use anyhow::{anyhow, Context, Result};
use serial::{SerialPort, SystemPort};

use std::ffi::OsStr;
use std::time::Duration;
use std::{env, fmt, thread};

/// The serial settings expected by the BootROM on the bl602
const BL602_SERIAL_SETTINGS: serial::PortSettings = serial::PortSettings {
    baud_rate: serial::BaudOther(500_000),
    char_size: serial::Bits8,
    parity: serial::ParityNone,
    stop_bits: serial::Stop1,
    flow_control: serial::FlowNone,
};

/// Opens the given path as a serial port
fn open_serial_port<P: AsRef<OsStr> + ?Sized + fmt::Display>(port: &P) -> Result<SystemPort> {
    serial::open(port).with_context(|| format!("Could not open serial port {}", port))
}

/// Configures the given `port` to a baud rate of 500_000 and sets the port timeout to 2 seconds
fn configure_serial_port<T: SerialPort>(port: &mut T) -> Result<()> {
    port.configure(&BL602_SERIAL_SETTINGS)?;
    port.set_timeout(Duration::from_millis(2000))?;

    Ok(())
}

/// Resets the the chip
fn reset<T: SerialPort>(port: &mut T) -> Result<()> {
    port.set_rts(false)?;
    thread::sleep(Duration::from_millis(200));

    for _ in 0..2 {
        port.set_rts(true)?;
        thread::sleep(Duration::from_millis(5));
        port.set_rts(false)?;
        thread::sleep(Duration::from_millis(100));
        port.set_rts(true)?;
        thread::sleep(Duration::from_millis(5));
        port.set_rts(false)?;
        thread::sleep(Duration::from_millis(5));
    }

    Ok(())
}

/// Toggle the RTS and DTR pins to as a preamble to let the chip know we want to communicate
fn send_handshake<T: SerialPort>(port: &mut T) -> Result<()> {
    port.set_rts(true)?;
    thread::sleep(Duration::from_millis(200));
    port.set_rts(false)?;
    thread::sleep(Duration::from_millis(50));
    port.set_rts(true)?;
    port.set_dtr(true)?;
    thread::sleep(Duration::from_millis(100));
    port.set_dtr(false)?;
    thread::sleep(Duration::from_millis(100));

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <port>", args[0]);
        return Err(anyhow!("Missing port"));
    }

    let port_path = &args[1];
    let mut port = open_serial_port(&port_path)?;

    configure_serial_port(&mut port)
        .with_context(|| format!("Could not configure serial port {}", port_path))?;
    send_handshake(&mut port).with_context(|| "Could not send handshake")?;
    reset(&mut port).with_context(|| "Could not reset device")?;

    println!("port: {:?}", port.timeout());

    Ok(())
}
