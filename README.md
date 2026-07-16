<p align="center">
  <img src="icon.svg" width="96" alt="keychron-micro">
</p>

# keychron-micro

Turn a Keychron Q0 Max into a scriptable macropad on **Linux**. Every key runs a
script from this repo; a small web UI shows what is bound and edits it live.

> **Linux only, and not by accident.** This works by grabbing the pad at the
> evdev layer and runs as a systemd user service, so it needs a Linux desktop.
> It is built and used on Fedora Workstation with GNOME on Wayland. Other distros
> and compositors should work, with the caveats in
> [what Wayland will not let you do](#what-wayland-will-not-let-you-do); a couple
> of keys are GNOME-specific and say so. There is no macOS or Windows port and
> there will not be one, since none of this exists there.

**The board lights up with what your scripts are doing.** The whole pad goes
yellow while a script runs, green when it exits cleanly, red when it fails, and
magenta when something wants you. It is driven over VIA raw HID and lives in RAM,
so your saved VIA config is never touched. Any process can colour it by writing
one word to a fifo, which is how a Claude Code session running on the pad reports
itself: yellow while it thinks, green when it hands back.
[How it works](#status-colours)

Clone, `make install`, press **numpad 5**. That is the demo key, and it works on
a pad you have never reprogrammed.

---

## Compared to Codex Micro

OpenAI and Work Louder sell [Codex Micro](https://openai.com/supply/co-lab/work-louder/):
a **$230** macropad, limited run, that shows agent status on lit keys. It is a
nice piece of hardware and the idea is a good one. This is the same idea, as free
software, on a pad you may already own.

|  | Codex Micro | keychron-micro |
|---|---|---|
| Cost | **$230**, while supplies last | **free**, on a [Q0 Max](https://www.keychron.com/products/keychron-q0-max-qmk-custom-number-pad) (~$110), or nothing if you have one |
| Source | closed | open, ~900 lines you can read in a sitting |
| Runs on | **Mac / Windows only** | Linux / Wayland |
| Tied to | the ChatGPT desktop app | any agent. Set `command = "codex"` and it is codex |
| Keys do | what the app exposes | whatever you can write in a shell script |
| Lights mean | agent status | whatever you write to a fifo |
| Also | | still a numpad |

The real difference is the last two rows. A key here runs an executable, so it
can do anything your machine can: open a tmux fleet, toggle the Activities
Overview, restart PipeWire. And the lights are a one-word write to a fifo from
*any* process, not a status feed from one vendor's app.

Not a fair fight in every direction, mind. Codex Micro is a designed object with
an analog stick, a dial, push-to-talk and a warranty, and it works out of the box
on the two platforms it targets. This is a daemon you `make install` on Fedora,
and the pad it needs is a numpad that does not know what an agent is. If you want
the hardware, buy the hardware; it looks lovely. But on Linux, where Codex Micro
does not go at all, this is the whole thing for the price of a numpad.

---

## Programming a key

Three things, in order.

**1. Find out what the key sends.** Open <http://127.0.0.1:7373> and press it.
The name appears on the page, `KEY_KP8` say. It reports unbound keys too, which
is exactly when you need it. (`make learn` does the same in a terminal, but has
to stop the daemon first: both want an exclusive grab.)

**2. Point it at a script.** In `config.toml`, or in the web UI's editor:

```toml
[[bind]]
key = "KEY_KP8"
label = "green 4-square grid"   # what the keycap shows
run = "scripts/my-script.sh"
```

**3. Save.** It applies at once, with no restart and no dropped keypress. A
config that would not load is rejected and never written, so a typo costs you a
red message rather than a dead pad on the next reboot.

### The three fields

- **`key`** is the evdev name from step 1.
- **`label`** is documentation only, and worth writing. The legends and the
  keycodes have nothing to do with each other: a key with a medical cross on it
  may send `KEY_KP1`. In six months you will not remember which.
- **`run`** is relative to this repo, so a clone reproduces your setup as-is.

### The keycodes do not matter

The daemon grabs the pad **exclusively**: its keys reach this daemon and nothing
else on the machine. So a key reporting `KEY_KP8` is just a name. It will never
collide with the `8` on your real keyboard, and there is no reason to reprogram
the pad at all. Leave it on Keychron's factory map and bind the numpad codes.

Two things that *are* worth doing in
[Keychron Launcher](https://launcher.keychron.com):

- **`M1` to `M4`** (the left column) fire *macros*. An empty macro sends nothing
  at all, so those keys are dead until you give them a keycode. Any unused one
  will do, since nothing escapes the grab.
- **`MO(1)`** switches layer inside the firmware and sends no keycode ever, so it
  cannot be bound. Layer 1 is a free second bank if you want one.

### What a script gets

Scripts are ordinary executables. The daemon passes:

| variable | what |
|---|---|
| `KEYCHRON_HARNESS` | the agent from `[harness]`, e.g. `claude` |
| `KEYCHRON_HARNESS_ARGS` | its args, pre-quoted for a shell |
| `KEYCHRON_WEB_URL` | where the web UI is listening |

**Careful: the daemon runs with a bare `PATH`.** A systemd user unit gets no
`~/.local/bin`, because nothing sources your shell profile. Anything going
through `scripts/lib/term.sh` is fine, since that runs a login shell, but a
script calling a tool from `~/.local/bin` directly will not find it. Use
`bash -lc`, as the scripts here do.

Two helpers worth using:

- **`lib/term.sh [--no-hold] <cmd>`** opens a window with whatever terminal the
  machine has: Ptyxis on Fedora Workstation, Konsole on the KDE spin, `xterm` as
  the floor. `$TERMINAL` overrides.
- **`lib/status.sh <state>`** colours the board (below). It no-ops when the
  daemon is not running, so a macro key never fails for want of lights.

---

## Presets

`config.toml` is yours: gitignored, and rewritten every time the web UI saves.
`presets/` is the tracked half, holding snapshots you chose to share.

```
make presets                       list them
make preset-use NAME=alexsjones    adopt one
make preset-save NAME=mine         publish your config.toml as one
make install PRESET=alexsjones     start from one
```

- **`default`** binds the demo key only. Nothing runs an agent until you say so.
- **`alexsjones`** is a full pad: agent sessions, tmux fleet, overview, terminals.

Because the two are separate files, your local tweaks never dirty the tree and a
`git pull` never treads on your bindings.

---

## Status colours

`run` opens a fifo at `$XDG_RUNTIME_DIR/keychron-micro/status`. Write a state to
it and the board takes that colour:

```sh
printf 'thinking\n' > "$XDG_RUNTIME_DIR/keychron-micro/status"
```

| state | colour | meaning |
|---|---|---|
| `idle` | white | nothing happening |
| `thinking` | yellow | working |
| `complete` | green | finished cleanly |
| `needs_input` | magenta | waiting for you |
| `error` | red | failed |

Key presses drive this with no help from the script: yellow while it runs, then
green or red by exit status, then back to white.

`claude-pad-settings.json` maps Claude Code's hook lifecycle onto those states,
and `scripts/agent-yolo.sh` passes it via `claude --settings`, so it applies to
the session the pad launched and to nothing else. Your other sessions are
untouched and `~/.claude/settings.json` is never written to.

---

## Changing the agent

```toml
[harness]
command = "claude"
args = ["--settings", "{repo}/claude-pad-settings.json", "--dangerously-skip-permissions"]
```

One edit, and every script follows, because they read the harness from the daemon
rather than naming an agent. `{repo}` expands to your clone, so the config stays
portable. For codex:

```toml
command = "codex"
args = ["--dangerously-bypass-approvals-and-sandbox"]
```

The colours are Claude's, though: they come from its hook lifecycle. Another
agent needs its own equivalent. Dropping `--settings` just means no colour from
the agent, and the keys still work.

---

## What Wayland will not let you do

Worth knowing before you write a key that cannot exist.

- **Focus a named window.** No. `Shell.Eval` is disabled since GNOME 41,
  `GetWindows` and `FocusSearch` answer *"is not allowed"*, and `wmctrl` and
  `xdotool` are X11-only no-ops. `focus-agent.sh` raises the *app* via
  `org.freedesktop.Application.Activate`, which lands on its most recent window:
  right with one window open, a guess with several. A shell extension such as
  [Window Calls](https://github.com/ickyicky/window-calls) is the way out.
- **Place a window.** No. `terminal.sh` relies on
  `org.gnome.mutter center-new-windows`, which is GNOME's own setting.
- **Open the Activities Overview.** Yes, as it happens.
  `org.gnome.Shell.OverviewActive` is `readwrite`, so `overview.sh` toggles it
  without an extension or a synthetic keypress.

---

## Why not GNOME's custom shortcuts

GNOME binds *key combinations*, not *devices*. It cannot tell that `Q` came from
the pad rather than your keyboard, so every binding fires from both. It is also
dconf-only, GNOME-only, and per-machine.

## Why not keyd

[keyd](https://github.com/rvaiya/keyd) is the obvious tool, but it is not
packaged for Fedora, and its `command()` runs as the user running keyd, which is
root. That is both a security smell and awkward for launching GUI apps that need
your Wayland session. This daemon runs as you.

---

## Install

```
make install      udev rule (sudo, once), service, and starts it
make update       rebuild and restart after editing scripts or config
make uninstall    remove the service and the udev rule; keeps config.toml
make logs         follow it
make probe        what the pad exposes, and whether we can reach it
```

The repo *is* the installation: the binary resolves `config.toml` and `scripts/`
relative to where it was built, so the directory has to stay put. Moving it means
`make update` afterwards.

## What it touches

- **The udev rule** grants access to VID:PID `3434:0800` only. That is this pad,
  never your real keyboard. Fedora deliberately withholds `uaccess` from
  keyboards, since read access to a keyboard is read access to everything you
  type. Joining the `input` group would work too, and would grant access to
  *every* input device.
- **The web UI** binds to `127.0.0.1` and only ever will. It rewrites a config in
  which every `run` is a script this daemon executes on a keypress; reachable
  from the network, that is remote code execution. `port = 0` disables it.
- **Lighting** is set over VIA raw HID (usage page `0xFF60`) and lives in RAM. It
  never sends `id_custom_save`, so nothing survives a replug and your saved VIA
  config cannot be clobbered.
- **The service** is a systemd *user* unit tied to `graphical-session.target`.
  The scripts open windows, so it lives and dies with the session that has a
  display to open them into.

## Requirements

Fedora Workstation (GNOME/Wayland), a Keychron Q0 Max, and a Rust toolchain.
`tmux` is needed only for `agents-tmux.sh`. Nothing else: the daemon is `evdev`,
`libc`, `serde` and a hand-rolled HTTP server.
