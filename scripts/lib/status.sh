#!/usr/bin/env bash
# Set the pad's colour: status.sh idle|thinking|complete|needs_input|error
#
# Writes to the running daemon's pipe. Safe to call when nothing is listening --
# a macro key should never fail because the lights are not up.
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "usage: status.sh idle|thinking|complete|needs_input|error" >&2
    exit 2
fi

pipe="${XDG_RUNTIME_DIR:-/tmp}/keychron-micro/status"
[ -p "$pipe" ] || exit 0

# Never block: if the daemon is wedged and the pipe is full, drop the update
# rather than hang the script that called us.
timeout 1 bash -c "printf '%s\n' \"\$1\" > \"\$2\"" _ "$1" "$pipe" || true
