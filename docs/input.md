# Input: joysticks, mice and keyboards through AmigaInput

The `input` module wraps **AmigaInput.library**, the AmigaOS 4 library for
game controllers and other input devices (USB joysticks, gamepads, mice,
keyboards). AmigaInput finds the devices through its drivers and lets a
program read their axes, buttons and hats.

- An **axis** is a value that moves along a range, such as a stick's left and
  right.
- A **button** is on or off (digital) or pressed by a degree (analogue).
- A **hat** is a direction pad.

## What it is for

Use it in a game or tool that reads a joystick or gamepad directly. The
module works by **polling**: you ask for the device's current values when you
want them, for example once per frame.

For ordinary keyboard and mouse input in a window, use the window's events
(see the `intuition` module) instead.

## Cargo features

```toml
[dependencies]
amigaos4 = { path = "...", features = ["amigainput"] }
```

| Feature | Gives | Pulls in |
|---|---|---|
| `amigainput` | the `input` module: `Context`, `Device`, `DeviceInfo`, `State`, `AinError`, `error_string`, the `q` query numbers, `AINDT_*` | nothing else |

The feature is called `amigainput`; the module is `amigaos4::input`.

## Build modes

| Mode | Status |
|---|---|
| no-C-runtime program (`#![no_std]`) | Creating a context, listing devices, error texts and refusing an unknown device are tested by the crate's test programs. **Obtaining and reading a device are untested**: the test machine had no AmigaInput devices. |
| std program (`fn main` on clib4) | Compiles; not exercised by the crate's test programs. |

You need no interface global of your own. If `IAmigaInput` is NULL the crate
opens AmigaInput.library itself.

## `Context`

A `Context` is your connection to AmigaInput (API version 1, polling, no
message port and no window). Dropping it deletes the context.

```rust
use amigaos4::input::{Context, AINDT_JOYSTICK, AINDT_MOUSE};

let ctx = Context::new()?;
for d in ctx.devices() {
    let kind = match d.kind {
        AINDT_MOUSE => "mouse",
        AINDT_JOYSTICK => "joystick",
        _ => "other",
    };
    // d.id, d.name, d.axes, d.buttons, d.hats
}
```

| Method | What it does |
|---|---|
| `Context::new() -> Result<Context>` | Creates a polling context. |
| `devices(&self) -> Vec<DeviceInfo>` | Copies every device AmigaInput offers (at most 256). |
| `query(&self, id: u32, param: u32, arg: u32) -> Result<u32>` | Asks one 32-bit question about device `id`. `param` is one of the `q` constants; `arg` is an axis, button or hat number where the question takes one, else 0. |
| `obtain(&self, id: u32) -> Result<Device<'_>>` | Takes device `id` for this context. |
| `last_error(&self) -> AinError` | The error code and text of the last call that failed. |

`DeviceInfo` has `id`, `name`, `kind` (`AINDT_MOUSE`, `AINDT_KEYBOARD` or
`AINDT_JOYSTICK`), `subtype`, `buttons`, `axes` and `hats`.

The `q` module holds the query numbers: `NUMAXES`, `NUMBUTTONS`, `NUMHATS`,
`POLLED`, `AXIS_SYMMETRIC`, `SUBCLASS`, `DRIVERVERSION`, `AXIS_CLASS`,
`LAYOUT_CLASS`, `CONNECTED` and `UNITS`.

## `Device` and `State`

`obtain` gives a `Device`. It borrows the context, so it is always released
before the context is deleted. Dropping it releases the device.

```rust
use amigaos4::input::{q, Context, AINDT_JOYSTICK};

let ctx = Context::new()?;
if let Some(info) = ctx.devices().into_iter().find(|d| d.kind == AINDT_JOYSTICK) {
    let axes = ctx.query(info.id, q::NUMAXES, 0)?;
    let pad = ctx.obtain(info.id)?;

    // once per frame:
    let st = pad.read()?;
    let x = st.axes.first().copied().unwrap_or(0);
    let fire = st.buttons.first().copied().unwrap_or(0) != 0;
}
```

| Method | What it does |
|---|---|
| `info(&self) -> &DeviceInfo` | The device as listed. |
| `read(&self) -> Result<State>` | The current values, copied. |
| `set_active(&self, on: bool) -> Result<()>` | Whether this context receives the device's input. |

A `State` holds three vectors, one value per axis, button and hat, in the
device's order:

| Field | Range |
|---|---|
| `axes` | -32768 to 32767 |
| `buttons` | 0 or 1 (digital), 0 to 32767 (analogue) |
| `hats` | -32768 to 32767 |

The axis, button and hat positions are asked for once, when the device is
obtained.

## Errors

A failed call gives `AmigaError::IoError(code)` with AmigaInput's error code,
or `AmigaError::Device` when the library gave no code. `obtain` with an id
that `devices()` does not list gives `AmigaError::InvalidArgument`.
`ctx.last_error()`
returns an `AinError` with `code` and `text`, and `error_string(code)` gives
the text for any code. For example, obtaining the same device twice in one
context is error 11 (AINERR_DOUBLEOBTAIN).

```rust
use amigaos4::input;

let e = ctx.last_error();
let text = input::error_string(e.code);
```

## Limitations

- Obtaining and reading a device are untested (see Build modes).
- `set_active` is not exercised by the crate's test programs.
- Not wrapped: event mode (a message port and AINDP_EVENT), contexts tied to
  a window, and the device requester (which is undocumented).

## See also

- [console.md](console.md): turning raw key codes into characters.
