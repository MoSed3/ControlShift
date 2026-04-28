use std::collections::HashMap;

use bridge_protocol::{ControllerId, DeviceType, OutputType};
use thiserror::Error;

pub const MAX_XINPUT_SLOTS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControllerConfig {
    pub excluded: bool,
    pub output_type: OutputType,
    pub hide_original_from_nonsteam: bool,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            excluded: false,
            output_type: OutputType::DirectInput,
            hide_original_from_nonsteam: false,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RouterError {
    #[error("all XInput slots are already assigned")]
    XInputSlotsFull,
    #[error("controller is not known: {0:?}")]
    UnknownController(ControllerId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutedController {
    pub slot: u8,
    pub device_type: DeviceType,
}

#[derive(Debug, Default)]
pub struct Router {
    configs: HashMap<ControllerId, ControllerConfig>,
    routed: HashMap<ControllerId, RoutedController>,
    next_dinput_slot: u8,
}

impl Router {
    pub fn set_config(
        &mut self,
        id: ControllerId,
        config: ControllerConfig,
    ) -> Result<(), RouterError> {
        if config.output_type == OutputType::XInput && !config.excluded {
            let already_xinput = self.configs.get(&id).is_some_and(|existing| {
                !existing.excluded && existing.output_type == OutputType::XInput
            });

            if !already_xinput && self.xinput_assigned_count() >= MAX_XINPUT_SLOTS {
                return Err(RouterError::XInputSlotsFull);
            }
        }

        self.configs.insert(id, config);
        Ok(())
    }

    pub fn config(&self, id: ControllerId) -> ControllerConfig {
        self.configs.get(&id).copied().unwrap_or_default()
    }

    pub fn xinput_assigned_count(&self) -> usize {
        self.configs
            .values()
            .filter(|config| !config.excluded && config.output_type == OutputType::XInput)
            .count()
    }

    pub fn route_controller(
        &mut self,
        id: ControllerId,
    ) -> Result<Option<RoutedController>, RouterError> {
        let config = self.config(id);

        if config.excluded {
            self.routed.remove(&id);
            return Ok(None);
        }

        let routed = match config.output_type {
            OutputType::XInput => RoutedController {
                slot: self.lowest_available_xinput_slot()?,
                device_type: DeviceType::XInput,
            },
            OutputType::DirectInput => {
                let slot = self.next_dinput_slot;
                self.next_dinput_slot = self.next_dinput_slot.saturating_add(1);
                RoutedController {
                    slot,
                    device_type: DeviceType::DirectInput,
                }
            }
        };

        self.routed.insert(id, routed);
        Ok(Some(routed))
    }

    pub fn unroute_controller(
        &mut self,
        id: ControllerId,
    ) -> Result<Option<RoutedController>, RouterError> {
        Ok(self.routed.remove(&id))
    }

    pub fn routed(&self, id: ControllerId) -> Option<RoutedController> {
        self.routed.get(&id).copied()
    }

    fn lowest_available_xinput_slot(&self) -> Result<u8, RouterError> {
        for slot in 0..MAX_XINPUT_SLOTS as u8 {
            let slot_used = self
                .routed
                .values()
                .any(|routed| routed.device_type == DeviceType::XInput && routed.slot == slot);

            if !slot_used {
                return Ok(slot);
            }
        }

        Err(RouterError::XInputSlotsFull)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xinput_config() -> ControllerConfig {
        ControllerConfig {
            output_type: OutputType::XInput,
            ..ControllerConfig::default()
        }
    }

    #[test]
    fn controller_config_defaults_to_dinput_not_hidden_not_excluded() {
        let config = ControllerConfig::default();

        assert!(!config.excluded);
        assert_eq!(config.output_type, OutputType::DirectInput);
        assert!(!config.hide_original_from_nonsteam);
    }

    #[test]
    fn rejects_fifth_xinput_assignment() {
        let mut router = Router::default();

        for id in 0..4 {
            router
                .set_config(ControllerId(id), xinput_config())
                .expect("first four XInput assignments should succeed");
        }

        let err = router
            .set_config(ControllerId(4), xinput_config())
            .expect_err("fifth XInput assignment should fail");

        assert_eq!(err, RouterError::XInputSlotsFull);
    }

    #[test]
    fn excluded_xinput_controller_does_not_count_against_cap() {
        let mut router = Router::default();

        router
            .set_config(
                ControllerId(99),
                ControllerConfig {
                    excluded: true,
                    output_type: OutputType::XInput,
                    hide_original_from_nonsteam: false,
                },
            )
            .unwrap();

        for id in 0..4 {
            router
                .set_config(ControllerId(id), xinput_config())
                .expect("excluded XInput assignment should not consume a slot");
        }

        assert_eq!(router.xinput_assigned_count(), 4);
    }

    #[test]
    fn routes_xinput_to_lowest_available_slot() {
        let mut router = Router::default();
        let first = ControllerId(1);
        let second = ControllerId(2);

        router.set_config(first, xinput_config()).unwrap();
        router.set_config(second, xinput_config()).unwrap();

        assert_eq!(router.route_controller(first).unwrap().unwrap().slot, 0);
        assert_eq!(router.route_controller(second).unwrap().unwrap().slot, 1);

        router.unroute_controller(first).unwrap();

        let third = ControllerId(3);
        router.set_config(third, xinput_config()).unwrap();
        assert_eq!(router.route_controller(third).unwrap().unwrap().slot, 0);
    }

    #[test]
    fn excluded_controller_is_not_routed() {
        let mut router = Router::default();
        let id = ControllerId(10);

        router
            .set_config(
                id,
                ControllerConfig {
                    excluded: true,
                    ..ControllerConfig::default()
                },
            )
            .unwrap();

        assert_eq!(router.route_controller(id).unwrap(), None);
        assert_eq!(router.routed(id), None);
    }
}
