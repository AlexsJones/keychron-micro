#!/usr/bin/env bash
# Numpad 3: a tmux session of four agents, tiled, one window.
#
# For setting up a batch of tasks side by side. Press again to re-attach to the
# same session rather than start four more -- the agents keep running whether a
# terminal is attached or not, which is the point of putting them in tmux.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

harness="${KEYCHRON_HARNESS:-claude}"
hargs="${KEYCHRON_HARNESS_ARGS:-}"
dir="${AGENT_PAD_DIR:-$HOME}"
session="${KEYCHRON_TMUX_SESSION:-agents}"
panes="${KEYCHRON_TMUX_PANES:-4}"

if ! command -v tmux >/dev/null 2>&1; then
    notify-send "keychron-micro" "tmux is not installed" 2>/dev/null || true
    echo "tmux not found" >&2
    exit 1
fi

# Each pane runs a login shell: tmux inherits the daemon's bare PATH, which has
# no ~/.local/bin, so `claude` would not resolve without one.
launch="$harness $hargs"

if ! tmux has-session -t "$session" 2>/dev/null; then
    tmux new-session -d -s "$session" -c "$dir" -n agents bash -lc "$launch"
    for _ in $(seq 2 "$panes"); do
        # Re-tile as we go: split-window fails once panes get too short to halve,
        # and tiling after each one keeps them all viable.
        tmux split-window -t "$session" -c "$dir" bash -lc "$launch"
        tmux select-layout -t "$session" tiled >/dev/null
    done
    tmux select-layout -t "$session" tiled >/dev/null
fi

exec "$here/lib/term.sh" --no-hold tmux attach -t "$session"
