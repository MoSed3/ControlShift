# Driver Development

ControlShift's Phase 3 driver target is a custom UMDF virtual HID bus driver for Windows.

## Current Status

The repository currently contains the `crates/vhid-driver` scaffold. It does not yet build or install a real UMDF driver.

The scaffold records the driver service name, IPC pipe name, XInput-compatible VID/PID, DirectInput custom VID/PID, and the planned driver phases.

## Phase 3a Goal

Phase 3a is complete only when a minimal UMDF driver package can be built, installed in test mode, loaded by Windows, and seen in Device Manager.

That requires manual machine setup:

- Windows 10/11 development machine
- Visual Studio Build Tools or equivalent MSVC environment
- Windows SDK
- Windows Driver Kit matching the SDK
- Rust MSVC toolchain
- Windows test signing enabled
- Administrator PowerShell for driver install/load checks

## Manual Test Signing Setup

Do not run these commands from automated tests.

```powershell
bcdedit /set testsigning on
```

Reboot after enabling test signing.

To disable test signing later:

```powershell
bcdedit /set testsigning off
```

Reboot again after disabling it.

## Manual Validation

Phase 3 driver validation belongs in manual checklists, not `cargo test`.

Automated tests should cover pure Rust driver-facing constants, report routing, IPC message encoding, and fake-backend behavior. Device installation, driver loading, Device Manager inspection, `joy.cpl`, and USB/HID tools require a person and a Windows driver development environment.
