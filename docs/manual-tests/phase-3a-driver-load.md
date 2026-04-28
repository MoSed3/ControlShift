# Phase 3a Manual Test - Minimal UMDF Driver Load

This checklist is for the first real driver milestone. It is intentionally manual and must not run from `cargo test`.

## Purpose

Confirm that the ControlShift virtual HID driver can be built, installed in Windows test-signing mode, loaded by the OS, and inspected in Device Manager.

## Preconditions

- Windows 10/11 development machine
- Windows SDK and matching WDK installed
- Rust MSVC toolchain installed
- Test signing enabled
- Machine rebooted after enabling test signing
- Administrator PowerShell available
- Minimal UMDF driver implementation exists in `crates/vhid-driver`

The current repository has only the Phase 3a scaffold. Treat this checklist as pending until the real UMDF entry point and driver package files exist.

## Manual Steps

1. Confirm test signing is enabled:

```powershell
bcdedit
```

2. Build the driver package using the project driver build command.

3. Install the driver package from Administrator PowerShell.

4. Open Device Manager.

5. Confirm `ControlShift Virtual HID Bus` appears.

6. Confirm the device has no warning icon.

7. Uninstall the driver package.

8. Confirm Device Manager no longer shows the ControlShift virtual HID bus.

## Pass Criteria

- Driver package builds without WDK version errors.
- Driver installs only in test-signing mode.
- Device Manager shows the ControlShift virtual HID bus.
- Driver unload/uninstall works without reboot.
- No system crash, hang, or stuck device entry.

## Notes

- Do not continue to child virtual devices until this passes.
- Keep `cargo test` independent from WDK and Device Manager.
- Record WDK version, Windows version, and Rust toolchain in the manual test result.
