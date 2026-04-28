# vhid-driver

Phase 3 starts here.

This crate is the home for the future ControlShift virtual HID bus driver. It currently contains the driver-facing constants and device specifications needed by the app side while the real UMDF implementation is still being built.

The real driver work must follow the Phase 3 plan:

- 3a: WDK and minimal UMDF driver load
- 3b: child device creation with XInput-compatible descriptor
- 3c: named pipe IPC
- 3d: DirectInput descriptor
- 3e: replace fake backend with driver backend
- 3f: stress test 8 simultaneous controllers

Normal `cargo test` must keep working without WDK installed. WDK and Device Manager checks are manual.
