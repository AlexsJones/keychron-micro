#!/usr/bin/env bash
# Numpad 3: a fullscreen tmux session of four agents, tiled.
#
# For setting up a batch of tasks side by side. Press again and it re-attaches to
# the same session rather than starting four more. The agents keep running
# whether a terminal is attached or not, which is the point of putting them in
# tmux: close the window and they carry on.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/agents.sh
. "$here/lib/agents.sh"

harness="${KEYCHRON_HARNESS:-claude}"
hargs="${KEYCHRON_HARNESS_ARGS:-}"
dir="${AGENT_PAD_DIR:-$HOME}"
panes="${KEYCHRON_TMUX_PANES:-4}"

require_tmux

# Each pane runs a login shell: tmux inherits the daemon's bare PATH, which has
# no ~/.local/bin, so `claude` would not resolve without one.
launch="$harness $hargs"

if ! agents_exist; then
    tmux new-session -d -s "$SESSION" -c "$dir" -n agents bash -lc "$launch"
    for _ in $(seq 2 "$panes"); do
        # Re-tile as we go: split-window fails once panes get too short to halve,
        # and tiling after each one keeps them all viable.
        tmux split-window -t "$SESSION" -c "$dir" bash -lc "$launch"
        tmux select-layout -t "$SESSION" tiled >/dev/null
    done
    tmux select-layout -t "$SESSION" tiled >/dev/null
fi

exec "$here/lib/term.sh" --no-hold --fullscreen tmux attach -t "$SESSION"
