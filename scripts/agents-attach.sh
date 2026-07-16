#!/usr/bin/env bash
# Numpad 8: attach to the running agent fleet. Does nothing if there is not one.
#
# The counterpart to numpad 3. That key creates the fleet; this one only ever
# joins it, so pressing it after closing the window gets you back to the same
# four agents rather than starting four more by surprise.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/agents.sh
. "$here/lib/agents.sh"

require_tmux

if ! agents_exist; then
    # Not an error: "nothing to attach to" is a normal answer, and a red pad
    # would be a lie. Say so quietly and stop.
    notify-send "keychron-micro" "No agents running" 2>/dev/null || true
    echo "no '$SESSION' session -- numpad 3 starts one" >&2
    exit 0
fi

exec "$here/lib/term.sh" --no-hold --fullscreen tmux attach -t "$SESSION"
