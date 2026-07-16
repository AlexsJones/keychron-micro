#!/usr/bin/env bash
# Numpad 5: what is this key again?
#
# Reads the bindings from the running daemon rather than a list kept here, so it
# cannot drift out of date: what it shows is what the pad will actually do,
# including anything changed in the web UI a minute ago.
#
# A GTK dialog is as modal as Wayland gets. It cannot float above everything or
# grab input the way an X11 override-redirect window could, and nothing can: a
# client places and stacks nothing. GNOME centres it and gives it focus, which in
# practice is what was wanted.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

url="${KEYCHRON_WEB_URL:-http://127.0.0.1:7373}"

if ! state=$(curl -fsS --max-time 2 "$url/api/state" 2>/dev/null); then
    notify-send "keychron-micro" "Cannot reach the daemon at $url" 2>/dev/null || true
    exit 1
fi

# Sorted the way the pad is laid out, not the way evdev names happen to sort.
# KEY_F13 next to KEY_INSERT next to KEY_KP0 is alphabetical order, which is no
# order at all when you are looking down at a numpad trying to find a key.
order='["KEY_PAGEUP","KEY_PAGEDOWN","KEY_INSERT",
        "KEY_F13","KEY_F14","KEY_F15","KEY_F16",
        "KEY_ESC","KEY_DELETE","KEY_TAB","KEY_BACKSPACE",
        "KEY_NUMLOCK","KEY_KPSLASH","KEY_KPASTERISK","KEY_KPMINUS",
        "KEY_KP7","KEY_KP8","KEY_KP9","KEY_KPPLUS",
        "KEY_KP4","KEY_KP5","KEY_KP6",
        "KEY_KP1","KEY_KP2","KEY_KP3","KEY_KPENTER",
        "KEY_KP0","KEY_KPDOT"]'

# The label is written "<key as Launcher shows it> -- <what the keycap shows>",
# so split it: the first half names the key you press, the second describes the
# cap you are looking at. Both beat KEY_KP0 for finding a key with your eyes.
rows=$(printf '%s' "$state" | jq -r --argjson order "$order" '
    def esc: gsub("&";"&amp;") | gsub("<";"&lt;") | gsub(">";"&gt;");
    def pad($n): . + (" " * ([$n - length, 0] | max));

    [ .binds[]
      # Grab the name first: piping $order into index() would make the `.` inside
      # it the array, and .key on an array is an error, not a lookup.
      | .key as $k
      | ($order | index($k)) as $i
      | . + { sort: (if $i == null then 99 else $i end) }
    ]
    | sort_by(.sort)
    | map(
        (.label // "") as $l
        | ($l | split(" -- ")) as $p
        | {
            key: (if ($p | length) > 1 then $p[0] else (.key | ltrimstr("KEY_") | ascii_downcase) end),
            cap: (if ($p | length) > 1 then ($p[1:] | join(" -- ")) else $l end),
            does: (.does // .run)
          }
      )
    | (map(.key | length) | max // 8) as $kw
    | (map(.cap | length) | max // 8) as $cw
    | .[]
    | "  <b>\(.key | pad($kw) | esc)</b>   \(.cap | pad($cw) | esc)   <span alpha=\"55%\">\(.does | esc)</span>"
')

harness=$(printf '%s' "$state" | jq -r '.harness | split(" ")[0]')
pad=$(printf '%s' "$state" | jq -r '.pad | split(" [")[0]')

# Pango markup, so the key you are hunting for is bold and the plumbing is not.
body="<tt>$rows

  <span alpha=\"55%\">$pad   ·   agent: $harness   ·   $url</span></tt>"

if command -v zenity >/dev/null 2>&1; then
    # Detached: zenity blocks until dismissed and the daemon waits on us to paint
    # the pad green. Nobody wants a yellow board until they click OK.
    setsid zenity --info \
        --title="keychron-micro" \
        --width=720 --height=460 \
        --text="$body" \
        >/dev/null 2>&1 &
    exit 0
fi

# No zenity: a terminal says the same thing and is always there. Strip the markup
# rather than print it.
plain=$(printf '%s' "$body" | sed -E 's/<[^>]+>//g; s/&amp;/\&/g; s/&lt;/</g; s/&gt;/>/g')
exec "$here/lib/term.sh" bash -c "printf '%s\n' ${plain@Q}"
