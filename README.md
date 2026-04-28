# ControlShift

ControlShift is a Windows controller-bridging project for people who want non-Steam games to work cleanly with a wide range of modern controllers.

The idea is simple:

`physical controller -> Steam Input normalized state -> ControlShift translation -> virtual gamepad device`

Steam handles controller mapping. ControlShift does not try to build a second remapping system. Its job is to take Steam's normalized controller state and expose it to non-Steam games as standard virtual controllers that games already understand.

## What problem this solves

Many controllers work well inside Steam but behave poorly outside it. Common problems include:

- Non-Steam games not recognizing a controller at all
- Games only supporting XInput cleanly
- Duplicate input when both physical and virtual devices are visible
- Local multiplayer setups hitting the 4-controller XInput limit
- Mixed-controller households needing one consistent path into games

ControlShift is meant to bridge that gap. A user should be able to use Steam Input as the compatibility layer for the physical controller, then let ControlShift present that controller to non-Steam games in a form they can consume.

## Project goals

- Support many physical controllers through one normalized input pipeline
- Let Steam remain the source of truth for mappings and per-controller setup
- Output either XInput or DirectInput/HID per controller
- Enforce the real XInput limit of 4 while allowing additional DirectInput devices
- Avoid duplicate detection by controlling device visibility
- Eventually support 8-player and larger local multiplayer setups on Windows
- Keep the system testable in phases with strong automated coverage before hardware-heavy integration

## What ControlShift is not

- Not a button remapper
- Not a replacement for Steam Input
- Not a general-purpose driver toolkit
- Not finished end-to-end yet

If a button layout is wrong, the expected fix is in Steam controller settings, not inside ControlShift.

## Planned architecture

The long-term design has these major parts:

1. Steam Input polling layer
2. Fixed translation layer for standard controller reports
3. Router that decides XInput vs DirectInput per controller
4. Virtual device backend
5. Custom UMDF virtual HID driver for Windows
6. HidHide integration to avoid duplicate input
7. Desktop UI for controller and game configuration

The full design notes and phased project breakdown are in [ControlShift-plan.md](./ControlShift-plan.md).

## Phase progress

- [x] Phase 0 - Rust Foundations
- [x] Phase 1 - Input Pipeline Prototype
- [ ] Phase 2 - ViGEmBus Scaffold
- [ ] Phase 3 - Custom UMDF Driver
- [ ] Phase 4 - HidHide Integration
- [ ] Phase 5 - Tauri UI
- [ ] Phase 6 - Installer and Distribution

## Current stage

The project is currently in **Phase 1: input pipeline prototype**.

Implemented today:

- Cargo workspace setup
- `bridge-protocol` crate for shared controller/report/IPC types
- `bridge-core` crate for routing, translation, and input pipeline abstractions
- XInput 4-slot rule enforcement in the router layer
- Dedicated poller thread shape with exclusion filtering and connect/disconnect events
- Fixed `to_xinput_report` and `to_dinput_report` translation functions
- Console prototype that prints translated reports from a fake input provider
- Unit tests covering protocol, translation, routing, and poller behavior

Not implemented yet:

- Real Steamworks / Steam Input binding
- Virtual controller driver/backend
- HidHide integration
- UI and installer

## Current repository shape

`crates/bridge-protocol`

- Shared types such as `ControllerId`, `ControllerState`, `OutputType`, report structures, and driver command messages

`crates/bridge-core`

- Router/config behavior, input polling prototype, translator logic, and the virtual-device backend trait used to keep later phases testable

## Development approach

This project is being built phase by phase so each layer can be tested and debugged before adding the next one. The current strategy is:

- build pure Rust core logic first
- keep hardware-dependent pieces behind interfaces
- use fake backends and automated tests early
- only add Steam, driver, and HidHide dependencies after the core behavior is stable

That approach reduces risk in the most complex parts of the system, especially the Windows driver and visibility-management work.

## Status summary

- Repository maturity: early prototype with Phase 1 pipeline in place
- Platform target: Windows 10/11
- Main technical direction: Steam Input -> virtual XInput or DirectInput/HID
- Testing status: automated tests are in place for protocol, router, translator, and poller logic

## License

This repository is licensed under the terms in [LICENSE](./LICENSE).
