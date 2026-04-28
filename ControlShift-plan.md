# Universal Gamepad Bridge — Full Project Plan

## Overview

A Windows desktop application (Steam-distributed) that intercepts any physical controller through the Steam Input API and re-emits it as a virtual gamepad device (XInput or DirectInput/HID) that non-Steam games recognize natively. Supports **unlimited simultaneous controllers** (8-player local co-op and beyond) via a fully custom virtual HID bus driver written in Rust.

**Core design principles:**
- Steam handles all button mapping — we only translate, never remap
- Per-controller output type: user picks XInput or DirectInput per device
- Controllers can be excluded entirely if they already work fine natively
- Virtual devices are hidden from Steam to prevent duplicate detection
- Original physical devices are selectively hidden from non-Steam programs per user choice

---

## Why Not ViGEmBus

ViGEmBus is a KMDF (kernel-mode) bus driver that emulates two device types:
- **Xbox 360 (XInput)** — hard capped at 4 by the XInput API (`XInputGetState` only accepts indices 0–3)
- **DualShock 4 (HID)** — technically unlimited, but games expecting XInput won't accept it

The XInput cap is not a ViGEmBus limitation — it is a **Windows API design constraint**. No matter how many virtual XInput devices you create, `xinput1_4.dll` will only ever enumerate 4 slots. To truly support 8+ controllers in XInput games, the game itself must use DirectInput or raw HID.

### The Real Strategy for 8+ Players

| Game API | Max Controllers | Strategy |
|---|---|---|
| XInput | 4 (API limit) | Virtual XInput devices, user assigns which controllers get XInput slots |
| DirectInput | Unlimited | Custom virtual HID devices, no ceiling |
| Raw HID | Unlimited | Custom virtual HID devices, no ceiling |
| Steam Input | Unlimited | Source — normalized input comes from here |

The plan is to build a **custom UMDF (User Mode Driver Framework) virtual HID bus driver** that creates virtual gamepad HID devices. Unlike ViGEmBus (KMDF, kernel-mode), UMDF runs in user space, has a much simpler signing story for distribution, and can create as many devices as system resources allow.

For XInput: virtual HID devices created with the Xbox 360 VID/PID (`045E:028E`) cause Windows to load `xusb22.sys` automatically, exposing them via the XInput API. The user controls which (up to 4) controllers use this path. All others are DirectInput HID devices.

---

## Design Decision: Steam Owns The Mapping

**This project does not remap buttons or axes.** That is Steam's job.

Steam Input already provides a full per-controller, per-game remapping UI that users already know. Our role is purely translation:

```
Steam Input abstract state  →  Standard XInput or DirectInput HID report
```

Steam Input outputs a normalized gamepad state regardless of the physical controller. Our translation is a fixed, deterministic conversion of that normalized state into a standard HID report byte layout. There is no configuration involved in this step — it is a pure technical translation of one standard format into another.

**If a user's button mapping is wrong → they fix it in Steam's controller settings, not ours.**

This simplifies the project significantly: no profile system, no mapping editor, no per-button config. The only per-controller configuration is:
- Is this controller excluded from bridging?
- Should it output as XInput or DirectInput?

---

## Architecture

```
┌───────────────────────────────────────────────────────────────┐
│                     Physical Controllers                       │
│      (Stadia, PS5, Switch Pro, 8BitDo, generic HID...)        │
│                                                               │
│  [Excluded controllers] ──────────────────────► pass through  │
│  [Bridged controllers]  ──────────────────────► continue ↓    │
└───────────────────────────┬───────────────────────────────────┘
                            │ raw HID / USB / BT
                            ▼
┌───────────────────────────────────────────────────────────────┐
│                   Steam Input API Layer                        │
│  • Normalizes all controllers to a standard gamepad state     │
│  • Steam handles all button/axis mapping per user settings    │
│  • Steamworks SDK (steamworks crate)                          │
└───────────────────────────┬───────────────────────────────────┘
                            │ normalized gamepad state
                            ▼
┌───────────────────────────────────────────────────────────────┐
│                  Standard Format Translator                    │
│  • Fixed, deterministic conversion — no user config here      │
│  • Steam state → XInput report layout (buttons, sticks, LT/RT)│
│  • Steam state → DirectInput HID report layout                │
└───────────────────────────┬───────────────────────────────────┘
                            │ typed XInput or HID report struct
                            ▼
┌───────────────────────────────────────────────────────────────┐
│                   Virtual Device Router                        │
│  • Per-controller config: XInput or DirectInput               │
│  • Enforces ≤ 4 XInput slots at all times                     │
│  • Assigns/releases slots on connect/disconnect               │
└──────────────┬────────────────────────────┬───────────────────┘
               │ XInput path                │ DirectInput path
               ▼                            ▼
┌──────────────────────┐      ┌─────────────────────────────────┐
│  Xbox VID/PID Device │      │   Custom VID/PID HID Device     │
│  (045E:028E)         │      │   (unknown vendor, safe VID)    │
│  xusb22.sys loads    │      │   Steam ignores unknown VID     │
│  → XInput slot       │      │   DirectInput sees via HID      │
│                      │      │   usage page                    │
└──────────┬───────────┘      └──────────────┬──────────────────┘
           │                                  │
           └──────────────┬───────────────────┘
                          │ IPC (named pipe)
                          ▼
┌───────────────────────────────────────────────────────────────┐
│            Custom Virtual HID Bus Driver (UMDF)               │
│  • Written in Rust (windows-drivers-rs + WDK)                 │
│  • Creates virtual HID devices on demand                      │
│  • No 4-controller ceiling for DirectInput devices            │
│  • UMDF = user-mode, simpler signing, no BSOD on crash        │
└───────────────────────────┬───────────────────────────────────┘
                            │ virtual HID reports
                            ▼
┌───────────────────────────────────────────────────────────────┐
│                     Visibility Layer (HidHide)                 │
│                                                               │
│  Purpose A: Hide virtual XInput devices from steam.exe        │
│             → Steam doesn't double-detect our virtual Xbox    │
│                                                               │
│  Purpose B: Hide physical devices from non-Steam programs     │
│             → Games see only the virtual device, not both     │
│             → Per-controller, opt-in by user                  │
│             → Our bridge process stays in allowlist           │
└───────────────────────────┬───────────────────────────────────┘
                            │
                            ▼
┌───────────────────────────────────────────────────────────────┐
│                      Non-Steam Games                          │
│  See clean standard Xbox / HID gamepad — no duplicates        │
└───────────────────────────────────────────────────────────────┘
```

---

## Repository Structure

```
gamepad-bridge/
├── Cargo.toml                   # workspace root
├── crates/
│   ├── bridge-core/             # main userspace process
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── steam_input/     # Steamworks SDK wrapper + poller
│   │   │   ├── translator/      # fixed Steam state → XInput/HID report
│   │   │   ├── router/          # per-controller slot + output type mgmt
│   │   │   ├── hidhide/         # HidHide integration (both directions)
│   │   │   ├── config/          # per-controller settings (excluded, type)
│   │   │   └── ipc/             # named pipe to vhid-driver
│   │   └── Cargo.toml
│   ├── vhid-driver/             # UMDF virtual HID bus driver
│   │   ├── src/
│   │   │   ├── lib.rs           # driver entry point
│   │   │   ├── bus.rs           # virtual bus device
│   │   │   ├── device.rs        # per-controller HID child device
│   │   │   └── hid_report.rs    # HID report descriptor (Xbox + generic)
│   │   ├── build.rs             # WDK linking
│   │   └── Cargo.toml
│   └── bridge-ui/               # system tray + config UI (Tauri)
│       ├── src-tauri/
│       └── src/                 # web frontend
├── installer/                   # WiX installer
│   ├── main.wxs
│   └── driver_install.ps1
└── docs/
    ├── architecture.md
    ├── hidhide_strategy.md
    └── driver_dev.md
```

---

## Component Deep Dives

### 1. Steam Input Layer (`bridge-core/src/steam_input/`)

Steam Input normalizes all controllers into a standard gamepad state regardless of physical hardware. The user configures their button layout inside Steam's own controller settings UI — we never touch that.

**Key responsibilities:**
- Call `SteamAPI_Init()` on startup; gracefully degrade if Steam is not running
- Call `SteamInput()->RunFrame()` on a dedicated high-frequency thread — this is mandatory polling, not event-driven
- Enumerate connected controllers with `GetConnectedControllers()`
- Skip controllers that are in the user's exclusion list
- Read analog axes via `GetAnalogActionData()` and digital buttons via `GetDigitalActionData()`
- Emit a `ControllerState` struct per controller per tick to the translator

**The Action Set approach:**
Define one Action Set called `Gamepad` with a fixed set of standard gamepad actions: `AxisLX`, `AxisLY`, `AxisRX`, `AxisRY`, `TriggerL`, `TriggerR`, `BtnA`, `BtnB`, `BtnX`, `BtnY`, `BtnLB`, `BtnRB`, `BtnStart`, `BtnBack`, `BtnGuide`, `BtnL3`, `BtnR3`, `DPadUp`, `DPadDown`, `DPadLeft`, `DPadRight`. Steam maps the physical controller's actual buttons to these standard names — that mapping lives in Steam and is edited by the user in Steam's UI.

```rust
// steam_input/poller.rs — conceptual structure
pub struct SteamPoller {
    tx: mpsc::Sender<(ControllerId, ControllerState)>,
    excluded: Arc<RwLock<HashSet<ControllerId>>>,
}

impl SteamPoller {
    pub fn run(self) {
        std::thread::spawn(move || {
            loop {
                unsafe { steam_input().run_frame() };
                let controllers = unsafe { steam_input().get_connected_controllers() };
                for id in controllers {
                    if self.excluded.read().unwrap().contains(&id) {
                        continue; // excluded — don't touch this controller
                    }
                    let state = self.read_state(id);
                    let _ = self.tx.try_send((id, state));
                }
                std::thread::sleep(Duration::from_micros(500)); // ~2000Hz
            }
        });
    }
}
```

**Critical:** `RunFrame()` runs on a **dedicated OS thread**, never in tokio. The sleep is fixed — no jitter from async scheduler.

**Steamworks SDK setup:**
- Download SDK from the Valve partner portal
- Link via `build.rs` using the `steamworks` crate or raw `bindgen`
- Ship `steam_api64.dll` alongside your binary
- Use Spacewar AppID (`480`) during development; apply for your own AppID early

---

### 2. Standard Format Translator (`bridge-core/src/translator/`)

This is the simplest component by design. It takes the normalized `ControllerState` from Steam Input and converts it into one of two standard byte layouts. There is no user configuration here.

```rust
// translator/mod.rs

pub struct ControllerState {
    pub left_x:    f32,  // -1.0 to 1.0
    pub left_y:    f32,
    pub right_x:   f32,
    pub right_y:   f32,
    pub trigger_l: f32,  // 0.0 to 1.0
    pub trigger_r: f32,
    pub buttons:   ButtonSet, // bitfield
    pub dpad:      DPadState,
}

/// Translate to standard XInput-equivalent HID report (Xbox 360 layout).
/// Fixed conversion — no configuration.
pub fn to_xinput_report(state: &ControllerState) -> XInputHidReport {
    XInputHidReport {
        left_x:    (state.left_x  * 32767.0) as i16,
        left_y:    (state.left_y  * 32767.0) as i16,
        right_x:   (state.right_x * 32767.0) as i16,
        right_y:   (state.right_y * 32767.0) as i16,
        trigger_l: (state.trigger_l * 255.0) as u8,
        trigger_r: (state.trigger_r * 255.0) as u8,
        buttons:   state.buttons.to_xinput_bits(),
        dpad:      state.dpad.to_hat_switch(),
    }
}

/// Translate to generic DirectInput HID report (standard gamepad HID layout).
/// Fixed conversion — no configuration.
pub fn to_dinput_report(state: &ControllerState) -> DInputHidReport {
    // same math, same layout — just a different HID descriptor on the driver side
}
```

**Why this is intentionally simple:** If a user's `BtnA` fires when they press `BtnB`, that is a Steam controller settings issue. They open Steam → Controller Settings → their controller → fix the binding. This project has no opinion about that.

---

### 3. Per-Controller Config (`bridge-core/src/config/`)

The only user configuration in this project. Stored as TOML, one entry per controller (identified by a stable device identifier derived from Steam's InputHandle or USB VID/PID + serial).

```toml
# config.toml — managed by the UI, not hand-edited by users

[controllers."stadia_1234abcd"]
label              = "Mo's Stadia Controller"
excluded           = false       # if true, bridge ignores this controller entirely
output_type        = "xinput"    # "xinput" or "dinput"
hide_from_nonsteam = true        # hide original device from non-Steam programs

[controllers."ds5_5678efgh"]
label              = "Player 2 DualSense"
excluded           = false
output_type        = "dinput"
hide_from_nonsteam = true

[controllers."xbox_aabbccdd"]
label              = "Office Xbox Controller"
excluded           = true        # works fine natively, skip entirely
# no other fields matter when excluded = true
```

**Exclusion behavior:**
When `excluded = true`:
- Steam Input still reads the controller (we can't stop that at the API level)
- We skip it in our poller — no virtual device created
- We do not add it to HidHide's blocklist
- It behaves exactly as if this program doesn't exist for that controller

**XInput slot enforcement:**
The config layer enforces: no more than 4 controllers may have `output_type = "xinput"` at any time. The UI prevents setting a 5th. If an XInput controller disconnects, its slot is freed and another controller (if waiting) can claim it.

```rust
// config/manager.rs
pub fn set_output_type(
    &mut self,
    id: ControllerId,
    output_type: OutputType,
) -> Result<(), ConfigError> {
    if output_type == OutputType::XInput {
        let current_xinput_count = self.count_xinput_assigned();
        if current_xinput_count >= 4 {
            return Err(ConfigError::XInputSlotsFull);
        }
    }
    self.entries.get_mut(&id).unwrap().output_type = output_type;
    self.save()?;
    Ok(())
}
```

---

### 4. Virtual Device Router (`bridge-core/src/router/`)

Manages the lifecycle of virtual devices. On controller connect: check config → create virtual device via IPC to the driver → register HidHide rules. On disconnect: destroy virtual device → release HidHide rules.

**XInput slot assignment:**
XInput identifies controllers by physical slot index 0–3. When a controller is assigned XInput output, the router picks the lowest free XInput slot and creates the virtual device with Xbox VID/PID (`045E:028E`). Windows loads `xusb22.sys` on top of it and the XInput API exposes it at that slot index.

**DirectInput device creation:**
Created with a custom vendor VID (e.g. `0x33A5` / `0x0001`). Steam Input does not auto-detect unknown VID/PIDs, so Steam will not try to handle it as a controller. Games using DirectInput or raw HID enumerate by HID Usage Page `0x01` / Usage `0x05` (Gamepad) and will find it correctly.

**Runtime slot table:**
```
XInput slots:   [Controller_A, Controller_B, empty, empty]  (hard max: 4)
DInput slots:   [Controller_C, Controller_D, Controller_E, ...]  (unlimited)
Excluded:       [Controller_F]  (not tracked, completely untouched)
```

---

### 5. Custom UMDF Virtual HID Driver (`vhid-driver/`)

The driver creates virtual HID bus children on demand. Each child appears to Windows as a freshly plugged-in physical USB gamepad.

#### What UMDF Is

UMDF (User Mode Driver Framework v2) runs in user space, not kernel ring 0:
- No BSOD if the driver crashes — restarts automatically
- Standard debugger works (no kernel debugger needed for most issues)
- Still needs EV cert + Microsoft attestation signing for distribution
- `microsoft/windows-drivers-rs` (official Microsoft Rust WDK crate) supports UMDF

#### Two Device Descriptors

The driver maintains two HID report descriptors:

**XInput descriptor** — mimics Xbox 360 controller exactly (VID `045E`, PID `028E`). This causes Windows to automatically load `xusb22.sys`, which wraps the device in the XInput API. Must be byte-for-byte identical to the real Xbox 360 descriptor — verify against a real device with USB Device Tree Viewer.

**DirectInput descriptor** — standard generic gamepad HID descriptor with your custom VID/PID. Uses HID Usage Page `0x01` / Usage `0x05`. Steam ignores this because it doesn't recognize the VID. DirectInput games find it by usage page, not VID.

#### IPC Between App and Driver

Named pipe at `\\.\pipe\gamepad-bridge-vhid`:

```rust
enum DriverCommand {
    PlugIn  { slot: u8, device_type: DeviceType },
    PlugOut { slot: u8 },
    Report  { slot: u8, data: [u8; 28] },
}

enum DeviceType {
    XInput,  // Xbox VID/PID → triggers xusb22.sys → XInput API
    DInput,  // Custom VID/PID → Steam-safe, DirectInput visible
}
```

For each `PlugIn`: the bus driver creates a new child PDO. For each `Report`: forward bytes as a HID input report. Windows handles enumeration, driver loading, and application exposure.

#### Development Setup

- Install **Windows Driver Kit (WDK)** matching your Windows SDK version
- Enable test signing on dev machine: `bcdedit /set testsigning on` (reboot required)
- Use `microsoft/windows-drivers-rs` for WDK FFI — official Microsoft Rust WDK project
- Debug with **WinDbg** + UMDF host tracing (WPP or TraceLogging)
- Inspect virtual devices with **USB Device Tree Viewer**; compare against real Xbox 360

#### Signing for Distribution

An **EV Code Signing Certificate** is required (~$300–500/yr from DigiCert or Sectigo). Submit driver package (`.inf` + UMDF `.dll`) to **Microsoft Hardware Dev Center** for attestation signing. Mandatory for Windows 11. Start this process early — approval can take weeks.

---

### 6. HidHide Integration (`bridge-core/src/hidhide/`)

HidHide is a kernel filter driver that hides HID devices from all processes except a configurable allowlist. In this project it serves two completely separate purposes.

#### Purpose A — Hide Virtual XInput Devices From Steam

When we create a virtual Xbox VID/PID device, Steam detects it and treats it as a controller — causing duplicate input (Steam sees the original physical via Steam Input AND the virtual via its own detection).

**Solution:** Add virtual XInput device instance IDs to HidHide's blocklist. Do NOT add `steam.exe` to the allowlist for these. Non-Steam game executables that the user adds are put in the allowlist, so they can see the virtual XInput device. Steam is left blind.

For DirectInput virtual devices this is not needed — the unknown VID/PID means Steam ignores them automatically without HidHide.

**XInput virtual device visibility:**
```
steam.exe         → HidHide blocks it    → cannot see virtual Xbox device ✓
game.exe          → in allowlist         → can see virtual Xbox device ✓
bridge-core.exe   → in allowlist always  → can see everything ✓
```

#### Purpose B — Hide Physical Devices From Non-Steam Programs

Per-controller, opt-in via `hide_from_nonsteam = true`. When enabled:

1. Physical device instance ID added to HidHide blocklist
2. `bridge-core.exe` is in the allowlist — Steam Input can still read the physical device
3. `steam.exe` is in the allowlist — Steam overlay and other Steam features still work
4. Non-Steam game executables are NOT in the allowlist — they see only the virtual device

**Physical device visibility when `hide_from_nonsteam = true`:**
```
steam.exe         → in allowlist         → sees physical controller ✓
bridge-core.exe   → in allowlist         → sees physical controller ✓
game.exe          → HidHide blocks it    → sees only virtual device ✓
```

**When `excluded = true` (any controller):**
```
steam.exe         → sees controller normally ✓
bridge-core.exe   → skips this controller in poller
game.exe          → sees controller directly, zero HidHide involvement ✓
```

#### Cleanup Is Non-Negotiable

If the bridge crashes without restoring HidHide state, physical controllers become invisible to all programs. Controllers are effectively bricked until reboot.

Four-layer cleanup — all must be implemented:
1. `Drop` impl on `HidHideManager` — runs on clean shutdown
2. `std::panic::set_hook` — runs on Rust panics
3. `SetConsoleCtrlHandler` Win32 call — catches Ctrl+C and system shutdown
4. Windows Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` — last resort if process is killed externally

```rust
pub struct HidHideManager {
    handle: HANDLE,
    hidden_physical: Vec<String>,  // physical instance IDs we hid
    hidden_virtual:  Vec<String>,  // virtual XInput IDs hidden from Steam
}

impl Drop for HidHideManager {
    fn drop(&mut self) {
        // restore in order: virtual first, then physical
        for id in &self.hidden_virtual  { let _ = self.unhide_device(id); }
        for id in &self.hidden_physical { let _ = self.unhide_device(id); }
        unsafe { CloseHandle(self.handle) };
    }
}
```

#### Game Executable Allowlist

Since HidHide's allowlist is process-based, users need a way to add game executables. The UI provides:
- "Add Game" button → file picker → selects `.exe` → added to HidHide allowlist
- List of allowed executables with remove button
- `bridge-core.exe` and `steam.exe` are permanent, cannot be removed

---

### 7. UI (`bridge-ui/`)

Built with **Tauri** — Rust backend communicates with bridge-core via IPC, web frontend handles all rendering.

**System tray:**
- Green = all bridged controllers active, no issues
- Yellow = Steam not running / HidHide not installed / degraded mode
- Red = UMDF driver not loaded or critical error
- Right-click: Open Window, Quit

**Main window — Controllers tab:**
- List of all detected controllers (physical)
- Per-controller row:
  - Controller name and type icon
  - Toggle: **Excluded** — if on, bridge completely ignores this controller
  - Toggle: **XInput / DirectInput** — output type (XInput grayed out when 4 slots already assigned; shows "4/4 XInput slots used" tooltip)
  - Toggle: **Hide original from non-Steam programs** — enables HidHide Purpose B
  - Status badge: Virtual slot number, actively bridging / idle
- Live input visualizer: stick positions, button highlights, trigger fill

**Main window — Game Executables tab:**
- List of `.exe` files currently in HidHide allowlist
- Add / Remove buttons
- Explanatory note: "Games listed here can see your virtual XInput controllers. Add the game's main executable."

**Main window — Status tab:**
- UMDF driver: loaded / not loaded
- HidHide: installed / version / not installed (with install button)
- Steam: running / not running / AppID
- XInput slots used: 0–4 / 4 max
- DirectInput virtual devices: count
- Log viewer with "Copy for bug report" button

---

## Development Phases

### Phase 0 — Rust Foundations (Week 1–2)
- Read Rust Book: ownership, error handling, traits, FFI, unsafe
- Set up cargo workspace with the crate structure above
- Write "hello from Steamworks" — init Steam, print controller count, read one axis value
- Understand the dedicated thread requirement for `RunFrame()`

**Goal:** FFI story is clear, workspace compiles, development environment ready.

---

### Phase 1 — Input Pipeline Prototype (Week 3–4)
- Implement `steam_input/` poller on dedicated thread with exclusion check
- Implement `translator/` — both fixed conversion functions (`to_xinput_report`, `to_dinput_report`)
- Print translated report structs to console at 60Hz per connected controller
- Handle connect/disconnect events

**Goal:** Steam Input read + fixed translation working cleanly. Don't move on until stable.

---

### Phase 2 — ViGEmBus Scaffold (Week 5–6)
- Temporarily use `vigem-client` crate for virtual XInput output
- Connect Phase 1 output → ViGEmBus → confirm in `joy.cpl`
- Move a Stadia stick → see Xbox controller respond in Windows Game Controllers
- Implement per-controller config (excluded flag, output type, XInput slot cap enforcement)
- Implement router slot assignment and release

**Goal:** Full pipeline end-to-end. Exclusion and output type selection working. XInput cap enforced. Validates the entire approach before driver work.

---

### Phase 3 — Custom UMDF Driver (Week 7–12)

- **3a:** Set up WDK + `windows-drivers-rs`, enable test signing. Minimal UMDF bus driver that loads cleanly. Confirm in Device Manager.
- **3b:** Add child device creation with hardcoded XInput HID descriptor. Confirm in `joy.cpl`.
- **3c:** Implement named pipe IPC. Send test HID report from CLI tool. Confirm state changes in `joy.cpl`.
- **3d:** Add DirectInput descriptor (custom VID/PID). Confirm Steam does NOT detect it. Confirm DirectInput-based tool does detect it.
- **3e:** Replace ViGEmBus in Phase 2 pipeline with the new driver. Full regression test.
- **3f:** Test 8 simultaneous controllers. Stress test connect/disconnect cycles.

**Goal:** ViGEmBus fully replaced. 4 XInput + 4 DirectInput simultaneously confirmed working.

---

### Phase 4 — HidHide Integration (Week 13–14)
- Implement `hidhide/` module — both Purpose A (hide virtual from Steam) and Purpose B (hide physical from games)
- Startup check: HidHide installed? If not, prompt with bundled installer
- Implement all four cleanup paths (Drop, panic hook, SetConsoleCtrlHandler, Job Object)
- Implement game executable allowlist management in config layer
- Test exact scenario: physical hidden → game sees only virtual → Steam sees only physical → no duplicates

**Goal:** Zero duplicate input. Zero bricked controllers on crash. Steam sees no virtual devices.

---

### Phase 5 — Tauri UI (Week 15–17)
- System tray with status icon logic
- Controllers tab with live input visualizer
- XInput slot cap enforced in UI (toggle grayed out at 4/4)
- Game Executables allowlist tab
- Status / diagnostics tab with log viewer

**Goal:** Non-technical users can configure everything from the UI. No file editing needed.

---

### Phase 6 — Installer & Distribution (Week 18–19)
- WiX installer: bundles HidHide installer (runs silently if not present), registers UMDF driver service, sets up autostart
- Obtain EV certificate
- Submit driver to Microsoft Hardware Dev Center for attestation signing
- Steam depot + store page setup

**Goal:** One-click install. User opens Steam, downloads, launches — everything works.

---

## Key Technical Risks & Mitigations

| Risk | Severity | Mitigation |
|---|---|---|
| UMDF driver complexity | High | ViGEmBus scaffold in Phase 2 de-risks the pipeline before driver work |
| XInput path requires exact Xbox HID descriptor | High | Byte-for-byte compare against real device with USB Device Tree Viewer |
| EV signing cost and timeline | Medium | Budget $400+; start during Phase 3 — approval takes weeks |
| Steamworks AppID required for production | Medium | Use Spacewar AppID (480) for dev; apply for real AppID early |
| Steam detecting virtual XInput device | Medium | HidHide blocks steam.exe from Xbox VID/PID virtual devices |
| Steam detecting virtual DirectInput device | None | Custom VID/PID → Steam ignores automatically, no HidHide needed |
| HidHide not installed on user machine | Low | Bundle installer, verify on startup, guide user |
| Crash leaving controllers hidden | High | 4-layer cleanup: Drop + panic hook + Win32 signal handler + Job Object |
| WDK version mismatches on user machines | Medium | Pin WDK version; test on Win10 21H2 and Win11 23H2 minimum |
| HidHide allowlist UX confusion | Medium | Clear in-app explanation; file picker for adding games |
| XInput slot confusion for 5+ player setups | Low | UI shows "4/4 XInput slots used" clearly; DirectInput is offered as alternative |

---

## Windows Version Support

| Windows | Support Level | Notes |
|---|---|---|
| Windows 10 21H2+ | Full | Primary target |
| Windows 11 22H2+ | Full | Primary target |
| Windows 10 before 21H2 | Best effort | UMDF v2 minimum is Win10 RS1 (1607) |
| Windows 7/8 | Not supported | Steamworks SDK itself has dropped support |

---

## Critical Points Summary

1. **Steam owns the mapping — always.** If a user asks "how do I remap my buttons," the answer is "open Steam → Controller Settings." This project has no mapping feature and should never grow one.

2. **XInput cap is 4 — enforce it in the UI, not just at runtime.** The XInput toggle must be grayed out when 4 slots are full, with a clear tooltip. Do not let users hit a runtime error they don't understand.

3. **Excluded controllers are completely untouched.** No HidHide, no virtual device, no poller entry. From every program's perspective, the bridge does not exist for that controller.

4. **Steam must not see virtual XInput devices.** HidHide blocks `steam.exe` from the Xbox VID/PID virtual device. Test explicitly: open Steam Big Picture → verify virtual Xbox does NOT appear there, while the physical one does.

5. **Custom VID/PID is your Steam shield for DirectInput devices.** Steam only auto-detects known VID/PIDs. An unknown vendor ID means Steam silently ignores the device. DirectInput and raw HID games find it by HID usage page, not VID. No HidHide needed for DirectInput virtual devices.

6. **`bridge-core.exe` and `steam.exe` must always be in the HidHide allowlist.** If you accidentally remove them, Steam Input loses access to the physical controller and the entire bridge breaks. Make them permanent and non-removable in the UI.

7. **The input polling loop is sacred.** Dedicated OS thread, fixed sleep, never touched by the async scheduler. Any jitter here is felt by the player immediately.

8. **Cleanup on all exit paths — no exceptions.** Drop + panic hook + SetConsoleCtrlHandler + Job Object. A user whose physical controller is invisible after a crash will uninstall the app immediately.

9. **HID report descriptor must be exact for the XInput path.** One wrong byte and `xusb22.sys` refuses to load. Compare byte-for-byte against a real Xbox 360 controller using USB Device Tree Viewer.

10. **Named pipe throughput.** HID reports at 1000Hz × 8 controllers = 8000 writes/second. Use a ring buffer and coalesce writes. Measure pipe latency under load — it should add < 1ms.

---

## Dependencies Summary

```toml
# bridge-core/Cargo.toml
[dependencies]
steamworks         = "0.11"
windows            = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_System_IO",
    "Win32_Devices_HumanInterfaceDevice",
    "Win32_Storage_FileSystem",
    "Win32_System_Threading",
    "Win32_System_JobObjects",
] }
tokio              = { version = "1", features = ["full"] }
serde              = { version = "1", features = ["derive"] }
toml               = "0.8"
arc-swap           = "1"       # lock-free config swap between poller thread and main
notify             = "6"       # watch config file for external changes
tracing            = "0.1"
tracing-subscriber = "0.3"
thiserror          = "1"
anyhow             = "1"

# vhid-driver/Cargo.toml
[dependencies]
windows            = { version = "0.58", features = [
    "Wdk_Foundation",
    "Wdk_Devices_HumanInterfaceDevice",
    "Wdk_System_SystemServices",
] }
# Build requires cargo-wdk toolchain from microsoft/windows-drivers-rs
```

---

## Suggested Name Ideas

- **Nexin** — Next-gen Input
- **UniBridge** — Universal Controller Bridge
- **InputForge** — forge the input signal into whatever shape the game needs
- **ControlShift** — shifting control signals between formats
- **PassPad** — passes your controller through, transformed

---

## Resources

- [Steamworks SDK Documentation](https://partner.steamgames.com/doc/sdk)
- [Steam Input Overview](https://partner.steamgames.com/doc/features/steam_controller)
- [Windows UMDF Driver Development](https://learn.microsoft.com/en-us/windows-hardware/drivers/wdf/umdf-driver-host-process)
- [microsoft/windows-drivers-rs](https://github.com/microsoft/windows-drivers-rs) — official Microsoft Rust WDK driver crate
- [HID Report Descriptor Tutorial — USB.org](https://www.usb.org/hid)
- [ViGEmBus Source (reference)](https://github.com/nefarius/ViGEmBus) — study the XUSB bus interface
- [HidHide Source (reference)](https://github.com/nefarius/HidHide) — study IOCTL interface for integration
- [DS4Windows](https://github.com/ds4windows/DS4Windows) — reference for HidHide allowlist patterns
- [windows-rs (microsoft)](https://github.com/microsoft/windows-rs)
- [USB Device Tree Viewer](https://www.uwe-sieber.de/usbtreeview_e.html) — essential for HID descriptor debugging
- [HidViz](https://github.com/hidviz/hidviz) — HID report descriptor visualizer
