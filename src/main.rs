use bouffalo::SerialPort;
use pretty_env_logger;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init_timed();

    let port = SerialPort::open("/dev/ttyUSB1", 500_000)?;
    println!("serial port: {:?}", port);
    let bootloader = port.enter_bootloader()?;
    let info = bootloader.get_boot_info()?;

    println!("boot info: {:?}", info);

    Ok(())
}
