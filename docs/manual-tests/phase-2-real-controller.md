# Phase 2 Manual Test - Real Controller Detection

These checks require a person, Steam, and at least one physical controller. Do not wire these into `cargo test`; they are manual acceptance checks for hardware behavior.

## Purpose

Confirm that the Phase 2 pipeline can be validated with real hardware when a human is available to run the check:

- physical controller is detected once
- connect and disconnect events are visible
- button, trigger, d-pad, and stick changes appear in translated reports
- excluded controllers do not produce virtual backend commands
- XInput and DirectInput routing select the expected report format

## Preconditions

- Windows 10/11 machine
- Steam running
- controller connected through USB or Bluetooth
- Steam Input enabled for the controller
- project builds with `cargo test`
- real Steam Input adapter or temporary hardware input adapter selected for the manual run

The automated test suite uses `FakeSteamInput` and `RecordingBackend`. Keep this document as a manual acceptance checklist; do not convert it into a default Rust test.

## Manual Steps

1. Start Steam and open Steam controller settings.
2. Connect one physical controller.
3. Run the ControlShift manual/dev binary that uses the real Steam Input source.
4. Confirm one connect event appears with a stable controller id and readable label.
5. Press `A/B/X/Y`, d-pad directions, `L1/R1`, `L2/R2`, `L3/R3`, `Back`, `Start`, `Home`, and `Capture/Share` where the controller supports it.
6. Move both sticks through all directions and sweep both triggers from released to fully pressed.
7. Confirm translated XInput reports contain supported XInput fields and omit unsupported buttons such as `Capture/Share`.
8. Confirm translated DirectInput reports preserve the richer button surface, including `Capture/Share`.
9. Disconnect the controller and confirm one disconnect event appears.
10. Mark the controller as excluded and repeat connect/input checks.
11. Confirm excluded controller input produces no virtual backend plug-in or report commands.

## Pass Criteria

- No duplicate connect events for a single physical controller.
- Disconnect is emitted exactly once when the controller is removed.
- Sticks, triggers, d-pad, and standard buttons change the expected report fields.
- `Capture/Share` is preserved for DirectInput/HID and omitted for XInput.
- Excluded controllers are ignored by the pipeline.
- No panic or stuck polling thread during connect/disconnect.

## Notes

- This is intentionally not an automated Rust test.
- Use automated fake-backend tests for CI and regression coverage.
- Keep manual results in the issue or release checklist for the phase being validated.
- If no real Steamworks-backed adapter exists yet, record the manual hardware check as pending instead of blocking `cargo test`.
