#!/usr/bin/env bash
# Run a command in a new terminal window, using whatever terminal the machine
# actually has. Ordered so a stock Fedora install works with nothing installed
# and nothing configured: Ptyxis on Workstation, Konsole on the KDE spin, xterm
# as the floor. Set $TERMINAL to override.
set -euo pipefail

hold_open=1
if [ "${1:-}" = "--no-hold" ]; then
    # For a shell, or a tmux attach: the window closing when you exit is the
    # point, and "press enter to close" would only be in the way.
    hold_open=0
    shift
fi

if [ $# -eq 0 ]; then
    echo "usage: term.sh [--no-hold] <command> [args...]" >&2
    exit 2
fi

# KEYCHRON_PAD_WINDOW marks the window as one of ours in the process list --
# that is how close-windows.sh finds pad-opened windows and leaves the terminal
# you are sitting in alone. It must be in the command line, not merely exported
# to the environment, because that is all pgrep can match on.
inner=$(printf '%q ' "$@")
if [ "$hold_open" = 1 ]; then
    # Hold the window open so output is readable after the command exits. Both
    # the format and $? have to survive to the shell we hand this to, hence the
    # quoting: the printf written here is not the printf that runs.
    hold="export KEYCHRON_PAD_WINDOW=1; ${inner}; printf '\n[exit %s] press enter to close' \$?; read -r"
else
    hold="export KEYCHRON_PAD_WINDOW=1; exec ${inner}"
fi

# Detach rather than exec: the caller is the keychron-micro daemon, which paints
# the pad green once we exit. Exec'ing would keep it "running" for as long as the
# window stayed open, so launch the terminal in its own session and return now.
# The exit status therefore means "the window opened", not what ran inside it --
# that is on show in the window itself.
launch() {
    setsid "$@" >/dev/null 2>&1 &
    exit 0
}

have() { command -v "$1" >/dev/null 2>&1; }

# An explicit choice wins over anything guessed below. Assumes -e, which every
# terminal here except the GTK ones accepts.
if [ -n "${TERMINAL:-}" ] && have "$TERMINAL"; then
    launch "$TERMINAL" -e bash -lc "$hold"
fi

# The freedesktop way to ask for "the user's terminal" -- respects their actual
# preference rather than our ordering, so it goes first.
if have xdg-terminal-exec; then
    launch xdg-terminal-exec bash -lc "$hold"
fi

# Fedora Workstation's default since 41, when it replaced gnome-terminal.
# Takes the command after --, not -e.
if have ptyxis; then
    launch ptyxis -- bash -lc "$hold"
fi

# Older Fedora/GNOME, and the KDE spin's default.
if have gnome-terminal; then
    launch gnome-terminal -- bash -lc "$hold"
fi
if have konsole; then
    launch konsole -e bash -lc "$hold"
fi

# Common third-party choices, if the user installed one on PATH.
for t in ghostty kitty wezterm alacritty foot; do
    if have "$t"; then
        launch "$t" -e bash -lc "$hold"
    fi
done

# Ugly, but it is in every install and never not there.
if have xterm; then
    launch xterm -e bash -lc "$hold"
fi

echo "term.sh: no terminal emulator found" >&2
exit 1
