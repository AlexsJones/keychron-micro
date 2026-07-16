#!/usr/bin/env bash
# Numpad 4: a plain terminal, centred.
#
# The centring is GNOME's, not ours: under Wayland a client cannot place its own
# window, and nothing outside the compositor can place it either. What GNOME does
# offer is `org.gnome.mutter center-new-windows`, which centres every new window
# on the active monitor. It is already on here, so this just opens a terminal and
# lets mutter do the rest. On a machine where it is off, this key opens a window
# wherever the compositor feels like -- `make install` does not change your
# desktop settings to fix that, and should not.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# $SHELL is not set for a systemd user unit, so fall back to the passwd entry
# rather than assuming bash.
shell="${SHELL:-}"
[ -n "$shell" ] || shell="$(getent passwd "$(id -un)" | cut -d: -f7)"
[ -n "$shell" ] || shell=/bin/bash

# --no-hold: exiting the shell should just close the window.
exec "$here/lib/term.sh" --no-hold "$shell" -l
