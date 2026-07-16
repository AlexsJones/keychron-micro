#!/usr/bin/env bash
# Numpad 1: bring the terminal running the agent to the front.
#
# CAVEAT, and it is not a small one: on GNOME Wayland nothing can focus a
# specific window from a script. The three ways you would try are all shut:
#
#   org.gnome.Shell.Introspect.GetWindows   -> "GetWindows is not allowed"
#   org.gnome.Shell.Eval                    -> returns (false, '') since GNOME 41
#   wmctrl / xdotool                        -> X11 only, no-ops under Wayland
#
# That leaves org.freedesktop.Application.Activate, which raises an *app*, not a
# window: GNOME brings its most-recently-used window forward. With one terminal
# window that is exactly right. With several, it raises whichever you touched
# last, which may not be the agent's.
#
# For real per-window focus, install a shell extension that exposes it over
# D-Bus (Window Calls, github.com/ickyicky/window-calls) and this can target the
# agent by title instead.
set -euo pipefail

# The terminal term.sh would have opened the agent in -- ask the same question
# in the same order, so we activate the app that is actually holding it.
app=""
for candidate in \
    "org.gnome.Ptyxis" \
    "org.gnome.Terminal" \
    "org.kde.konsole" \
    "com.mitchellh.ghostty" \
    "dev.kitty.kitty"
do
    if gdbus introspect --session --dest "$candidate" \
        --object-path "/${candidate//./\/}" >/dev/null 2>&1; then
        app="$candidate"
        break
    fi
done

if [ -z "$app" ]; then
    notify-send "keychron-micro" "No terminal found on the session bus to raise" 2>/dev/null || true
    echo "no known terminal owns a bus name -- nothing to activate" >&2
    exit 1
fi

exec gdbus call --session --dest "$app" --object-path "/${app//./\/}" \
    --method org.freedesktop.Application.Activate "{}" >/dev/null
