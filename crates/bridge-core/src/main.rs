use std::sync::mpsc::channel;
use std::time::Duration;

use bridge_core::{
    ControllerEvent, ControllerSnapshot, FakeSteamInput, SteamPoller, to_dinput_report,
    to_xinput_report,
};
use bridge_protocol::{Button, ControllerId, ControllerState};

fn main() {
    let frames = demo_frames();
    let poller =
        SteamPoller::new(FakeSteamInput::new(frames)).with_tick_rate(Duration::from_millis(16));
    let (tx, rx) = channel();
    let handle = poller.spawn(tx);

    let mut state_events = 0usize;
    for event in rx {
        match event {
            ControllerEvent::Connected { id, label } => {
                println!("[connect] id={:?} label={label}", id);
            }
            ControllerEvent::State(snapshot) => {
                let xinput = to_xinput_report(snapshot.state);
                let dinput = to_dinput_report(snapshot.state);
                println!(
                    "[state] id={:?} label={} xinput={xinput:?} dinput={dinput:?}",
                    snapshot.id, snapshot.label
                );
                state_events += 1;
                if state_events >= 6 {
                    break;
                }
            }
            ControllerEvent::Disconnected { id } => {
                println!("[disconnect] id={id:?}");
            }
        }
    }

    let _ = handle.join();
}

fn demo_frames() -> Vec<Vec<ControllerSnapshot>> {
    let mut neutral = ControllerState::default();

    let mut jump = ControllerState {
        left_stick_x: 8_000,
        left_stick_y: -4_000,
        left_trigger: 24,
        ..ControllerState::default()
    };
    jump.set_pressed(Button::A, true);

    let mut shoulder = ControllerState {
        right_stick_x: 12_000,
        right_stick_y: -10_000,
        right_trigger: 200,
        ..ControllerState::default()
    };
    shoulder.set_pressed(Button::RightBumper, true);
    shoulder.set_pressed(Button::RightTrigger, true);
    shoulder.set_pressed(Button::DPadRight, true);

    neutral.set_pressed(Button::Guide, true);

    vec![
        vec![ControllerSnapshot {
            id: ControllerId(1),
            label: "Mock Stadia".to_string(),
            state: jump,
        }],
        vec![ControllerSnapshot {
            id: ControllerId(1),
            label: "Mock Stadia".to_string(),
            state: shoulder,
        }],
        vec![
            ControllerSnapshot {
                id: ControllerId(1),
                label: "Mock Stadia".to_string(),
                state: neutral,
            },
            ControllerSnapshot {
                id: ControllerId(2),
                label: "Mock DualSense".to_string(),
                state: jump,
            },
        ],
        vec![ControllerSnapshot {
            id: ControllerId(2),
            label: "Mock DualSense".to_string(),
            state: shoulder,
        }],
        vec![],
    ]
}
