#!/usr/bin/env bash
# Numpad 5: the demo key. Opens a window proving the whole chain works --
# your keypress reached the daemon, the daemon ran this script, and the script
# opened a terminal on your desktop.
#
# Bound in presets/default.toml, so a fresh clone does something on the first
# press with nothing configured.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(dirname "$here")"

url="${KEYCHRON_WEB_URL:-http://127.0.0.1:7373}"

# Everything below runs inside the new window, so it is readable there rather
# than vanishing into the daemon's journal.
read -r -d '' script <<EOF || true
printf '\n  keychron-micro is working.\n\n'
printf '  You pressed a key. The daemon caught it, ran %s,\n' 'scripts/demo.sh'
printf '  and that opened this window. That is the whole idea.\n\n'

printf '  Your pad:\n'
'$repo/target/release/keychron-micro' probe 2>&1 | sed 's/^/    /'

printf '\n  The pad is grabbed exclusively, so its keys reach this daemon and\n'
printf '  nothing else. That means the keycodes do not matter -- a key that\n'
printf '  reports numpad 5 is just a name. Nothing else on the machine sees it.\n\n'

printf '  Next:\n'
printf '    1. Open %s\n' '$url'
printf '    2. Press any key on the pad -- its name appears on the page.\n'
printf '    3. Paste that name into config.toml, point it at a script, save.\n\n'
printf '  No restart needed. The daemon rebinds as soon as it validates.\n\n'
EOF

exec "$here/lib/term.sh" bash -c "$script"
