#!/usr/bin/env bash
# Rotary encoder: cycle to the next agent pane.
#
# Issued as a tmux command, not a keystroke: it works whether or not the tmux
# window has focus, and cannot collide with a key an agent wants for itself.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/agents.sh
. "$here/lib/agents.sh"

require_tmux
agents_do select-pane -t "$SESSION:.+"
