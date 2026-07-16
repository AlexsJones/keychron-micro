#!/usr/bin/env bash
# Red "i" key: a coding-agent session with permission prompts turned off.
#
# Which agent, and the flags it gets, come from [harness] in config.toml, handed
# down by the daemon. This script names no agent of its own, so moving from
# claude to codex is one edit there rather than a change here. The defaults below
# apply only when it is run by hand, outside the daemon.
#
# The agent runs unattended with its safety rail down, so this is deliberately a
# singleton: a macro key is easy to lean on, and each press would otherwise be
# another unsupervised agent.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

harness="${KEYCHRON_HARNESS:-claude}"
hargs="${KEYCHRON_HARNESS_ARGS:-}"

# Where the session starts. Override per-machine without editing this script.
dir="${AGENT_PAD_DIR:-$HOME}"

# Named per-harness, so switching agent does not inherit the other's lock and
# refuse to start.
lock="${XDG_RUNTIME_DIR:-/tmp}/keychron-micro/$(basename "$harness").lock"
mkdir -p "$(dirname "$lock")"

# Resolve exactly as the launch will. term.sh runs the command in a login shell,
# which sources your profile and so has your real PATH; this script does not --
# under systemd it inherits a bare PATH without ~/.local/bin. Checking with a
# plain `command -v` here would report "not found" for an agent that runs fine.
if ! bash -lc "command -v ${harness@Q}" >/dev/null 2>&1; then
    notify-send "keychron-micro" "$harness is not on PATH" 2>/dev/null || true
    echo "harness '$harness' not found in a login shell -- check [harness] in config.toml" >&2
    exit 1
fi

# The session below holds this lock for as long as it runs, so a second press
# lands here and does nothing.
if ! flock -n "$lock" true 2>/dev/null; then
    notify-send "$harness" "Session already running" 2>/dev/null || true
    exit 0
fi

# flock inside the window is the real guard -- the check above only saves us
# opening a window we would immediately close.
#
# term.sh already wraps this in a login shell, so plain `bash -c` here: a second
# -l would source the profile twice for nothing. cd explicitly rather than
# relying on inherited cwd, which a single-instance terminal does not carry over.
# $hargs arrives already shell-quoted from the daemon, so it goes in unquoted.
exec "$here/lib/term.sh" bash -c \
    "cd ${dir@Q} && exec flock -n ${lock@Q} ${harness@Q} $hargs"
