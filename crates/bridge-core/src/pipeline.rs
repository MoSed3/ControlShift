use bridge_protocol::{ControllerId, DeviceType, OutputType};
use thiserror::Error;

use crate::router::{ControllerConfig, Router, RouterError};
use crate::steam_input::ControllerEvent;
use crate::translator::{to_dinput_report, to_xinput_report};
use crate::virtual_device::{BackendError, VirtualDeviceBackend};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PipelineError {
    #[error(transparent)]
    Router(#[from] RouterError),
    #[error(transparent)]
    Backend(#[from] BackendError),
}

#[derive(Debug)]
pub struct InputPipeline<B> {
    router: Router,
    backend: B,
}

impl<B> InputPipeline<B> {
    pub fn new(router: Router, backend: B) -> Self {
        Self { router, backend }
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

impl<B> InputPipeline<B>
where
    B: VirtualDeviceBackend,
{
    pub fn set_controller_config(
        &mut self,
        id: ControllerId,
        config: ControllerConfig,
    ) -> Result<(), PipelineError> {
        let previous = self.router.unroute_controller(id)?;
        self.router.set_config(id, config)?;

        if let Some(previous) = previous {
            self.backend.plug_out(previous.slot)?;

            if let Some(current) = self.router.route_controller(id)? {
                self.backend.plug_in(current.slot, current.device_type)?;
            }
        }

        Ok(())
    }

    pub fn handle_event(&mut self, event: ControllerEvent) -> Result<(), PipelineError> {
        match event {
            ControllerEvent::Connected { id, .. } => {
                if self.router.routed(id).is_none()
                    && let Some(routed) = self.router.route_controller(id)?
                {
                    self.backend.plug_in(routed.slot, routed.device_type)?;
                }
            }
            ControllerEvent::State(snapshot) => {
                if let Some(routed) = self.router.routed(snapshot.id) {
                    let bytes = match routed.device_type {
                        DeviceType::XInput => {
                            to_xinput_report(snapshot.state).to_le_bytes().to_vec()
                        }
                        DeviceType::DirectInput => {
                            to_dinput_report(snapshot.state).to_le_bytes().to_vec()
                        }
                    };
                    self.backend.send_report(routed.slot, &bytes)?;
                }
            }
            ControllerEvent::Disconnected { id } => {
                if let Some(routed) = self.router.unroute_controller(id)? {
                    self.backend.plug_out(routed.slot)?;
                }
            }
        }

        Ok(())
    }
}

impl Default for InputPipeline<crate::virtual_device::RecordingBackend> {
    fn default() -> Self {
        Self::new(
            Router::default(),
            crate::virtual_device::RecordingBackend::default(),
        )
    }
}

pub fn config_for_output(output_type: OutputType) -> ControllerConfig {
    ControllerConfig {
        output_type,
        ..ControllerConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bridge_protocol::{Button, ControllerId, ControllerState, DriverCommand};

    use crate::steam_input::ControllerSnapshot;
    use crate::virtual_device::RecordingBackend;

    fn state_with(button: Button) -> ControllerState {
        let mut state = ControllerState::default();
        state.set_pressed(button, true);
        state
    }

    fn snapshot(id: u64, state: ControllerState) -> ControllerSnapshot {
        ControllerSnapshot {
            id: ControllerId(id),
            label: format!("Controller {id}"),
            state,
        }
    }

    #[test]
    fn pipeline_plugs_reports_and_unplugs_dinput_controller() {
        let mut pipeline = InputPipeline::default();

        pipeline
            .handle_event(ControllerEvent::Connected {
                id: ControllerId(1),
                label: "Controller 1".to_string(),
            })
            .unwrap();
        pipeline
            .handle_event(ControllerEvent::State(snapshot(
                1,
                state_with(Button::Capture),
            )))
            .unwrap();
        pipeline
            .handle_event(ControllerEvent::Disconnected {
                id: ControllerId(1),
            })
            .unwrap();

        assert_eq!(
            pipeline.backend().commands,
            vec![
                DriverCommand::PlugIn {
                    slot: 0,
                    device_type: DeviceType::DirectInput,
                },
                DriverCommand::Report {
                    slot: 0,
                    data: to_dinput_report(state_with(Button::Capture))
                        .to_le_bytes()
                        .to_vec(),
                },
                DriverCommand::PlugOut { slot: 0 },
            ]
        );
    }

    #[test]
    fn pipeline_uses_xinput_backend_reports_for_xinput_config() {
        let mut pipeline = InputPipeline::new(Router::default(), RecordingBackend::default());
        pipeline
            .set_controller_config(ControllerId(1), config_for_output(OutputType::XInput))
            .unwrap();

        pipeline
            .handle_event(ControllerEvent::Connected {
                id: ControllerId(1),
                label: "Controller 1".to_string(),
            })
            .unwrap();
        pipeline
            .handle_event(ControllerEvent::State(snapshot(1, state_with(Button::A))))
            .unwrap();

        assert_eq!(
            pipeline.backend().commands,
            vec![
                DriverCommand::PlugIn {
                    slot: 0,
                    device_type: DeviceType::XInput,
                },
                DriverCommand::Report {
                    slot: 0,
                    data: to_xinput_report(state_with(Button::A))
                        .to_le_bytes()
                        .to_vec(),
                },
            ]
        );
    }

    #[test]
    fn pipeline_ignores_excluded_controller_events() {
        let mut pipeline = InputPipeline::default();
        pipeline
            .set_controller_config(
                ControllerId(1),
                ControllerConfig {
                    excluded: true,
                    ..ControllerConfig::default()
                },
            )
            .unwrap();

        pipeline
            .handle_event(ControllerEvent::Connected {
                id: ControllerId(1),
                label: "Controller 1".to_string(),
            })
            .unwrap();
        pipeline
            .handle_event(ControllerEvent::State(snapshot(1, state_with(Button::A))))
            .unwrap();
        pipeline
            .handle_event(ControllerEvent::Disconnected {
                id: ControllerId(1),
            })
            .unwrap();

        assert!(pipeline.backend().commands.is_empty());
    }

    #[test]
    fn pipeline_ignores_duplicate_connect_events() {
        let mut pipeline = InputPipeline::default();
        let connect = ControllerEvent::Connected {
            id: ControllerId(1),
            label: "Controller 1".to_string(),
        };

        pipeline.handle_event(connect.clone()).unwrap();
        pipeline.handle_event(connect).unwrap();

        assert_eq!(
            pipeline.backend().commands,
            vec![DriverCommand::PlugIn {
                slot: 0,
                device_type: DeviceType::DirectInput,
            }]
        );
    }

    #[test]
    fn pipeline_replugs_connected_controller_when_output_type_changes() {
        let mut pipeline = InputPipeline::default();

        pipeline
            .handle_event(ControllerEvent::Connected {
                id: ControllerId(1),
                label: "Controller 1".to_string(),
            })
            .unwrap();
        pipeline
            .set_controller_config(ControllerId(1), config_for_output(OutputType::XInput))
            .unwrap();

        assert_eq!(
            pipeline.backend().commands,
            vec![
                DriverCommand::PlugIn {
                    slot: 0,
                    device_type: DeviceType::DirectInput,
                },
                DriverCommand::PlugOut { slot: 0 },
                DriverCommand::PlugIn {
                    slot: 0,
                    device_type: DeviceType::XInput,
                },
            ]
        );
    }

    #[test]
    fn pipeline_supports_four_xinput_and_extra_dinput_controllers() {
        let mut pipeline = InputPipeline::default();

        for id in 0..4 {
            pipeline
                .set_controller_config(ControllerId(id), config_for_output(OutputType::XInput))
                .unwrap();
        }

        for id in 0..8 {
            pipeline
                .handle_event(ControllerEvent::Connected {
                    id: ControllerId(id),
                    label: format!("Controller {id}"),
                })
                .unwrap();
        }

        let plug_ins = pipeline
            .backend()
            .commands
            .iter()
            .filter_map(|command| match command {
                DriverCommand::PlugIn { slot, device_type } => Some((*slot, *device_type)),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            plug_ins,
            vec![
                (0, DeviceType::XInput),
                (1, DeviceType::XInput),
                (2, DeviceType::XInput),
                (3, DeviceType::XInput),
                (0, DeviceType::DirectInput),
                (1, DeviceType::DirectInput),
                (2, DeviceType::DirectInput),
                (3, DeviceType::DirectInput),
            ]
        );
    }
}
