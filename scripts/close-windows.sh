#!/usr/bin/env bash
# Numpad 2: close the terminal windows this pad opened.
#
# Deliberately NOT every terminal on the desktop. Your editor, your shell, the
# session you are reading this in -- all of those are terminals too, and a macro
# key that killed them would be a foot-gun you press by accident once and regret.
# term.sh marks the windows it opens with KEYCHRON_PAD_WINDOW in their command
# line; only those are matched here. Anything you opened yourself is untouched.
#
# Whatever is running inside does get killed -- an agent mid-turn included. That
# is what closing the window means.
set -euo pipefail

marker='KEYCHRON_PAD_WINDOW=1'

# pgrep excludes itself, and the shells we started are the ones carrying the
# marker; killing them is what makes the window go away.
mapfile -t pids < <(pgrep -u "$USER" -f "$marker" 2>/dev/null || true)

if [ ${#pids[@]} -eq 0 ]; then
    notify-send "keychron-micro" "No pad-opened windows" 2>/dev/null || true
    exit 0
fi

# TERM first so shells can tidy up; anything still there after a moment is not
# going to leave politely.
kill -TERM "${pids[@]}" 2>/dev/null || true
sleep 0.5
for p in "${pids[@]}"; do
    kill -0 "$p" 2>/dev/null && kill -KILL "$p" 2>/dev/null || true
done

notify-send "keychron-micro" "Closed ${#pids[@]} pad window(s)" 2>/dev/null || true
