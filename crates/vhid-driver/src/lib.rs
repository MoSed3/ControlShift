use bridge_protocol::DeviceType;

pub const DRIVER_SERVICE_NAME: &str = "ControlShiftVhid";
pub const DRIVER_DISPLAY_NAME: &str = "ControlShift Virtual HID Bus";
pub const IPC_PIPE_NAME: &str = r"\\.\pipe\controlshift-vhid";

pub const XINPUT_VENDOR_ID: u16 = 0x045e;
pub const XINPUT_PRODUCT_ID: u16 = 0x028e;
pub const DINPUT_VENDOR_ID: u16 = 0x33a5;
pub const DINPUT_PRODUCT_ID: u16 = 0x0001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HidDescriptorKind {
    XInputCompatible,
    GenericDirectInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualHidDeviceSpec {
    pub device_type: DeviceType,
    pub vendor_id: u16,
    pub product_id: u16,
    pub descriptor_kind: HidDescriptorKind,
}

impl VirtualHidDeviceSpec {
    pub const fn for_device_type(device_type: DeviceType) -> Self {
        match device_type {
            DeviceType::XInput => Self {
                device_type,
                vendor_id: XINPUT_VENDOR_ID,
                product_id: XINPUT_PRODUCT_ID,
                descriptor_kind: HidDescriptorKind::XInputCompatible,
            },
            DeviceType::DirectInput => Self {
                device_type,
                vendor_id: DINPUT_VENDOR_ID,
                product_id: DINPUT_PRODUCT_ID,
                descriptor_kind: HidDescriptorKind::GenericDirectInput,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverPhase {
    ScaffoldOnly,
    MinimalUmdfLoad,
    ChildDeviceCreation,
    NamedPipeIpc,
    DirectInputDescriptor,
    PipelineBackend,
    StressTested,
}

pub const CURRENT_DRIVER_PHASE: DriverPhase = DriverPhase::ScaffoldOnly;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xinput_spec_uses_xbox_360_vid_pid() {
        let spec = VirtualHidDeviceSpec::for_device_type(DeviceType::XInput);

        assert_eq!(spec.vendor_id, 0x045e);
        assert_eq!(spec.product_id, 0x028e);
        assert_eq!(spec.descriptor_kind, HidDescriptorKind::XInputCompatible);
    }

    #[test]
    fn dinput_spec_uses_custom_vid_pid() {
        let spec = VirtualHidDeviceSpec::for_device_type(DeviceType::DirectInput);

        assert_eq!(spec.vendor_id, 0x33a5);
        assert_eq!(spec.product_id, 0x0001);
        assert_eq!(spec.descriptor_kind, HidDescriptorKind::GenericDirectInput);
    }

    #[test]
    fn phase_3_starts_as_scaffold_only() {
        assert_eq!(CURRENT_DRIVER_PHASE, DriverPhase::ScaffoldOnly);
    }
}
