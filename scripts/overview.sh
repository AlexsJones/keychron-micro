#!/usr/bin/env bash
# Numpad 0: the Activities Overview -- what tapping Super does.
#
# GNOME shuts every obvious route to this: Shell.Eval is disabled since GNOME 41,
# and FocusSearch/GetWindows answer "is not allowed" to callers outside the
# shell's own whitelist. But org.gnome.Shell.OverviewActive is declared
# `readwrite`, and setting it is allowed -- so this needs no extension, no
# synthetic keypress, and no /dev/uinput.
#
# GNOME only. On KDE or sway this key does nothing; the toggle is mutter's.
set -euo pipefail

shell=(--session --dest org.gnome.Shell --object-path /org/gnome/Shell)

if ! state=$(gdbus call "${shell[@]}" --method org.freedesktop.DBus.Properties.Get \
        org.gnome.Shell OverviewActive 2>/dev/null); then
    notify-send "keychron-micro" "No GNOME Shell to talk to" 2>/dev/null || true
    echo "org.gnome.Shell is not on the session bus -- this key is GNOME-only" >&2
    exit 1
fi

# Read first, then invert: a blind `set true` would open an overview that is
# already open, where tapping Super would have closed it.
case "$state" in
    *true*) want=false ;;
    *)      want=true  ;;
esac

exec gdbus call "${shell[@]}" --method org.freedesktop.DBus.Properties.Set \
    org.gnome.Shell OverviewActive "<$want>" >/dev/null
