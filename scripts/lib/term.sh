#!/usr/bin/env bash
# Run a command in a new terminal window, using whatever terminal the machine
# actually has. Ordered so a stock Fedora install works with nothing installed
# and nothing configured: Ptyxis on Workstation, Konsole on the KDE spin, xterm
# as the floor. Set $TERMINAL to override.
set -euo pipefail

hold_open=1
fullscreen=0
while true; do
    case "${1:-}" in
        --no-hold)
            # For a shell, or a tmux attach: the window closing when you exit is
            # the point, and "press enter to close" would only be in the way.
            hold_open=0; shift ;;
        --fullscreen)
            fullscreen=1; shift ;;
        *) break ;;
    esac
done

if [ $# -eq 0 ]; then
    echo "usage: term.sh [--no-hold] [--fullscreen] <command> [args...]" >&2
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

# Every terminal spells fullscreen differently, and one that does not know the
# flag would refuse to start -- so ask each one in its own dialect, and open a
# normal window rather than none where there is no answer.
fs() { [ "$fullscreen" = 1 ] && printf '%s' "$1"; }

# An explicit choice wins over anything guessed below. Assumes -e, which every
# terminal here except the GTK ones accepts.
if [ -n "${TERMINAL:-}" ] && have "$TERMINAL"; then
    launch "$TERMINAL" -e bash -lc "$hold"
fi

# The freedesktop way to ask for "the user's terminal" -- respects their actual
# preference rather than our ordering, so it goes first. No fullscreen flag
# exists in the spec, so a fullscreen request skips it rather than lose it.
if have xdg-terminal-exec && [ "$fullscreen" = 0 ]; then
    launch xdg-terminal-exec bash -lc "$hold"
fi

# Fedora Workstation's default since 41, when it replaced gnome-terminal.
# Takes the command after --, not -e.
if have ptyxis; then
    launch ptyxis $(fs --fullscreen) -- bash -lc "$hold"
fi

# Older Fedora/GNOME, and the KDE spin's default.
if have gnome-terminal; then
    launch gnome-terminal $(fs --full-screen) -- bash -lc "$hold"
fi
if have konsole; then
    launch konsole $(fs --fullscreen) -e bash -lc "$hold"
fi

# Common third-party choices, if the user installed one on PATH.
if have ghostty; then
    launch ghostty $(fs --fullscreen=true) -e bash -lc "$hold"
fi
if have kitty; then
    launch kitty $(fs --start-as=fullscreen) -e bash -lc "$hold"
fi
if have wezterm; then
    launch wezterm $(fs --position=main) start -- bash -lc "$hold"
fi
if have alacritty; then
    launch alacritty $(fs -o=window.startup_mode="Fullscreen") -e bash -lc "$hold"
fi
if have foot; then
    launch foot $(fs --fullscreen) -e bash -lc "$hold"
fi

# Ugly, but it is in every install and never not there.
if have xterm; then
    launch xterm -e bash -lc "$hold"
fi

echo "term.sh: no terminal emulator found" >&2
exit 1
