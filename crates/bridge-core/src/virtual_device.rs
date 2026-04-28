use bridge_protocol::{DeviceType, DriverCommand};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BackendError {
    #[error("virtual device backend is unavailable")]
    Unavailable,
    #[error("virtual device backend rejected command: {0}")]
    Rejected(String),
}

pub trait VirtualDeviceBackend {
    fn plug_in(&mut self, slot: u8, device_type: DeviceType) -> Result<(), BackendError>;
    fn plug_out(&mut self, slot: u8) -> Result<(), BackendError>;
    fn send_report(&mut self, slot: u8, data: &[u8]) -> Result<(), BackendError>;
}

#[derive(Debug, Default)]
pub struct RecordingBackend {
    pub commands: Vec<DriverCommand>,
}

impl VirtualDeviceBackend for RecordingBackend {
    fn plug_in(&mut self, slot: u8, device_type: DeviceType) -> Result<(), BackendError> {
        self.commands
            .push(DriverCommand::PlugIn { slot, device_type });
        Ok(())
    }

    fn plug_out(&mut self, slot: u8) -> Result<(), BackendError> {
        self.commands.push(DriverCommand::PlugOut { slot });
        Ok(())
    }

    fn send_report(&mut self, slot: u8, data: &[u8]) -> Result<(), BackendError> {
        self.commands.push(DriverCommand::Report {
            slot,
            data: data.to_vec(),
        });
        Ok(())
    }
}
