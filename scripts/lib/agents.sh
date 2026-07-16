#!/usr/bin/env bash
# Shared bits for the agent-fleet keys, so numpad 3 and numpad 8 cannot disagree
# about which session they mean.

SESSION="${KEYCHRON_TMUX_SESSION:-agents}"

require_tmux() {
    if ! command -v tmux >/dev/null 2>&1; then
        notify-send "keychron-micro" "tmux is not installed" 2>/dev/null || true
        echo "tmux not found" >&2
        exit 1
    fi
}

agents_exist() {
    tmux has-session -t "$SESSION" 2>/dev/null
}

# tmux is client/server, so the pad drives it by issuing commands rather than by
# synthesizing keystrokes into the window. That means no tmux key bindings to
# collide with what an agent wants its own keys for, nothing to intercept in the
# root table, and it works whether or not the tmux window even has focus.
agents_do() {
    if ! agents_exist; then
        notify-send "keychron-micro" "No agents running" 2>/dev/null || true
        exit 0
    fi
    tmux "$@"
}
