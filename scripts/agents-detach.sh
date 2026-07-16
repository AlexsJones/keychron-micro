#!/usr/bin/env bash
# Rotary encoder press: detach from the agent fleet.
#
# Detach, not kill: the agents carry on running with no terminal attached, and
# numpad 8 gets you back to them. That is the whole reason they live in tmux.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/agents.sh
. "$here/lib/agents.sh"

require_tmux

# Nothing attached is not a failure -- pressing this twice, or after closing the
# window by hand, is an ordinary thing to do. tmux says "no current client" and
# exits 1; the pad should not go red for that.
agents_do detach-client -s "$SESSION" 2>/dev/null || true
