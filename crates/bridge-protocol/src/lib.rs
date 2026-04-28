use serde::{Deserialize, Serialize};

pub const XINPUT_REPORT_LEN: usize = 12;
pub const DINPUT_REPORT_LEN: usize = 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControllerId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputType {
    XInput,
    DirectInput,
}

impl Default for OutputType {
    fn default() -> Self {
        Self::DirectInput
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    XInput,
    DirectInput,
}

impl From<OutputType> for DeviceType {
    fn from(value: OutputType) -> Self {
        match value {
            OutputType::XInput => Self::XInput,
            OutputType::DirectInput => Self::DirectInput,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Button {
    A,
    B,
    X,
    Y,
    LeftBumper,
    RightBumper,
    Back,
    Start,
    Guide,
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    LeftTrigger,
    RightTrigger,
    Capture,
}

impl Button {
    pub const COUNT: usize = 18;

    pub const fn bit(self) -> u32 {
        1 << (self as u32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerState {
    pub buttons: u32,
    pub left_stick_x: i16,
    pub left_stick_y: i16,
    pub right_stick_x: i16,
    pub right_stick_y: i16,
    pub left_trigger: u8,
    pub right_trigger: u8,
}

impl ControllerState {
    pub fn is_pressed(&self, button: Button) -> bool {
        self.buttons & button.bit() != 0
    }

    pub fn set_pressed(&mut self, button: Button, pressed: bool) {
        if pressed {
            self.buttons |= button.bit();
        } else {
            self.buttons &= !button.bit();
        }
    }

    pub fn xinput_button_bits(&self) -> u16 {
        const XINPUT_SUPPORTED: [Button; 15] = [
            Button::A,
            Button::B,
            Button::X,
            Button::Y,
            Button::LeftBumper,
            Button::RightBumper,
            Button::Back,
            Button::Start,
            Button::Guide,
            Button::LeftStick,
            Button::RightStick,
            Button::DPadUp,
            Button::DPadDown,
            Button::DPadLeft,
            Button::DPadRight,
        ];

        let mut bits = 0_u16;
        for button in XINPUT_SUPPORTED {
            if self.is_pressed(button) {
                bits |= 1 << (button as u16);
            }
        }
        bits
    }
}

impl Default for ControllerState {
    fn default() -> Self {
        Self {
            buttons: 0,
            left_stick_x: 0,
            left_stick_y: 0,
            right_stick_x: 0,
            right_stick_y: 0,
            left_trigger: 0,
            right_trigger: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct XInputReport {
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub left_stick_x: i16,
    pub left_stick_y: i16,
    pub right_stick_x: i16,
    pub right_stick_y: i16,
}

impl XInputReport {
    pub fn to_le_bytes(self) -> [u8; XINPUT_REPORT_LEN] {
        let mut bytes = [0; XINPUT_REPORT_LEN];
        bytes[0..2].copy_from_slice(&self.buttons.to_le_bytes());
        bytes[2] = self.left_trigger;
        bytes[3] = self.right_trigger;
        bytes[4..6].copy_from_slice(&self.left_stick_x.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.left_stick_y.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.right_stick_x.to_le_bytes());
        bytes[10..12].copy_from_slice(&self.right_stick_y.to_le_bytes());
        bytes
    }
}

impl Default for XInputReport {
    fn default() -> Self {
        ControllerState::default().into()
    }
}

impl From<ControllerState> for XInputReport {
    fn from(value: ControllerState) -> Self {
        Self {
            buttons: value.xinput_button_bits(),
            left_trigger: value.left_trigger,
            right_trigger: value.right_trigger,
            left_stick_x: value.left_stick_x,
            left_stick_y: value.left_stick_y,
            right_stick_x: value.right_stick_x,
            right_stick_y: value.right_stick_y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DInputReport {
    pub bytes: [u8; DINPUT_REPORT_LEN],
}

impl DInputReport {
    pub const fn zeroed() -> Self {
        Self {
            bytes: [0; DINPUT_REPORT_LEN],
        }
    }
}

impl Default for DInputReport {
    fn default() -> Self {
        Self::zeroed()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriverCommand {
    PlugIn { slot: u8, device_type: DeviceType },
    PlugOut { slot: u8 },
    Report { slot: u8, data: Vec<u8> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_type_defaults_to_direct_input() {
        assert_eq!(OutputType::default(), OutputType::DirectInput);
    }

    #[test]
    fn controller_state_defaults_to_neutral() {
        let state = ControllerState::default();

        assert_eq!(state.buttons, 0);
        assert_eq!(state.left_stick_x, 0);
        assert_eq!(state.left_stick_y, 0);
        assert_eq!(state.right_stick_x, 0);
        assert_eq!(state.right_stick_y, 0);
        assert_eq!(state.left_trigger, 0);
        assert_eq!(state.right_trigger, 0);
    }

    #[test]
    fn button_flags_are_set_and_cleared() {
        let mut state = ControllerState::default();

        state.set_pressed(Button::A, true);
        state.set_pressed(Button::RightBumper, true);
        state.set_pressed(Button::LeftTrigger, true);
        assert!(state.is_pressed(Button::A));
        assert!(state.is_pressed(Button::RightBumper));
        assert!(state.is_pressed(Button::LeftTrigger));
        assert!(!state.is_pressed(Button::B));

        state.set_pressed(Button::A, false);
        assert!(!state.is_pressed(Button::A));
        assert!(state.is_pressed(Button::RightBumper));
    }

    #[test]
    fn controller_state_can_carry_capture_and_trigger_press_separately() {
        let mut state = ControllerState {
            left_trigger: 32,
            right_trigger: 200,
            ..ControllerState::default()
        };

        state.set_pressed(Button::Capture, true);
        state.set_pressed(Button::LeftTrigger, true);

        assert!(state.is_pressed(Button::Capture));
        assert!(state.is_pressed(Button::LeftTrigger));
        assert_eq!(state.left_trigger, 32);
        assert_eq!(state.right_trigger, 200);
    }

    #[test]
    fn xinput_translation_drops_unsupported_buttons() {
        let mut state = ControllerState {
            left_trigger: 64,
            right_trigger: 128,
            ..ControllerState::default()
        };

        state.set_pressed(Button::A, true);
        state.set_pressed(Button::Capture, true);
        state.set_pressed(Button::LeftTrigger, true);
        state.set_pressed(Button::RightTrigger, true);

        let report = XInputReport::from(state);

        assert_eq!(report.buttons, Button::A.bit() as u16);
        assert_eq!(report.left_trigger, 64);
        assert_eq!(report.right_trigger, 128);
    }

    #[test]
    fn xinput_report_has_expected_wire_size() {
        let report = XInputReport {
            buttons: 0x1204,
            left_trigger: 10,
            right_trigger: 240,
            left_stick_x: -100,
            left_stick_y: 100,
            right_stick_x: i16::MIN,
            right_stick_y: i16::MAX,
        };

        let bytes = report.to_le_bytes();

        assert_eq!(bytes.len(), XINPUT_REPORT_LEN);
        assert_eq!(&bytes[0..2], &0x1204_u16.to_le_bytes());
        assert_eq!(bytes[2], 10);
        assert_eq!(bytes[3], 240);
        assert_eq!(&bytes[4..6], &(-100_i16).to_le_bytes());
    }

    #[test]
    fn dinput_report_has_expected_wire_size() {
        assert_eq!(DInputReport::default().bytes.len(), DINPUT_REPORT_LEN);
    }

    #[test]
    fn button_count_matches_superset_model() {
        assert_eq!(Button::COUNT, 18);
        assert!(Button::Capture.bit() > Button::DPadRight.bit());
    }

    #[test]
    fn protocol_types_serialize() {
        let command = DriverCommand::PlugIn {
            slot: 2,
            device_type: DeviceType::DirectInput,
        };

        let json = serde_json::to_string(&command).unwrap();

        assert!(json.contains("PlugIn"));
        assert!(json.contains("DirectInput"));
    }
}
