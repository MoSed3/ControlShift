use bridge_protocol::{Button, ControllerState, DINPUT_HAT_CENTERED, DInputReport, XInputReport};

pub fn to_xinput_report(state: ControllerState) -> XInputReport {
    state.into()
}

pub fn to_dinput_report(state: ControllerState) -> DInputReport {
    DInputReport {
        buttons: dinput_button_bits(&state),
        hat: dinput_hat(&state),
        left_stick_x: state.left_stick_x,
        left_stick_y: state.left_stick_y,
        right_stick_x: state.right_stick_x,
        right_stick_y: state.right_stick_y,
        left_trigger: state.left_trigger,
        right_trigger: state.right_trigger,
    }
}

fn dinput_button_bits(state: &ControllerState) -> u32 {
    const DINPUT_BUTTONS: [Button; 14] = [
        Button::A,
        Button::B,
        Button::X,
        Button::Y,
        Button::LeftBumper,
        Button::RightBumper,
        Button::LeftTrigger,
        Button::RightTrigger,
        Button::Back,
        Button::Start,
        Button::Guide,
        Button::Capture,
        Button::LeftStick,
        Button::RightStick,
    ];

    let mut bits = 0_u32;
    for (index, button) in DINPUT_BUTTONS.into_iter().enumerate() {
        if state.is_pressed(button) {
            bits |= 1 << index;
        }
    }
    bits
}

fn dinput_hat(state: &ControllerState) -> u8 {
    let up = state.is_pressed(Button::DPadUp);
    let down = state.is_pressed(Button::DPadDown);
    let left = state.is_pressed(Button::DPadLeft);
    let right = state.is_pressed(Button::DPadRight);

    match (up, down, left, right) {
        (true, false, false, false) => 0,
        (true, false, false, true) => 1,
        (false, false, false, true) => 2,
        (false, true, false, true) => 3,
        (false, true, false, false) => 4,
        (false, true, true, false) => 5,
        (false, false, true, false) => 6,
        (true, false, true, false) => 7,
        _ => DINPUT_HAT_CENTERED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xinput_translation_keeps_only_supported_button_surface() {
        let mut state = ControllerState {
            left_trigger: 20,
            right_trigger: 90,
            ..ControllerState::default()
        };
        state.set_pressed(Button::A, true);
        state.set_pressed(Button::Guide, true);
        state.set_pressed(Button::Capture, true);

        let report = to_xinput_report(state);

        assert_eq!(report.buttons, 0x1000);
        assert_eq!(report.left_trigger, 20);
        assert_eq!(report.right_trigger, 90);
    }

    #[test]
    fn dinput_translation_preserves_extra_buttons() {
        let mut state = ControllerState::default();
        state.set_pressed(Button::Guide, true);
        state.set_pressed(Button::Capture, true);
        state.set_pressed(Button::LeftTrigger, true);

        let report = to_dinput_report(state);

        assert_eq!(report.buttons, (1 << 6) | (1 << 10) | (1 << 11));
    }

    #[test]
    fn dinput_translation_uses_hat_for_dpad() {
        let mut state = ControllerState::default();
        state.set_pressed(Button::DPadUp, true);
        state.set_pressed(Button::DPadRight, true);

        let report = to_dinput_report(state);

        assert_eq!(report.hat, 1);
    }

    #[test]
    fn dinput_translation_centers_hat_for_conflicting_dpad() {
        let mut state = ControllerState::default();
        state.set_pressed(Button::DPadUp, true);
        state.set_pressed(Button::DPadDown, true);

        let report = to_dinput_report(state);

        assert_eq!(report.hat, DINPUT_HAT_CENTERED);
    }
}
