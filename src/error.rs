use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerialError {
    #[error("Error when opening serial port: {}", _0)]
    OpenError(String, serial::Error),
    #[error("Error when trying to set serial port baud rate to {}: {}", _0, _1)]
    BaudError(usize, serial::Error),
    #[error("Error when setting serial timeout to {}: {}", _0, _1)]
    TimeoutError(u64, serial::Error),
}
