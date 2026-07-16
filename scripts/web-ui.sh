#!/usr/bin/env bash
# N.Ent (crosshair): open this daemon's config page in the default browser.
set -euo pipefail

# The daemon passes the port it actually bound. The fallback is only for running
# this by hand -- if you changed [web] port, the daemon's value is the true one.
url="${KEYCHRON_WEB_URL:-http://127.0.0.1:7373}"

# Detached: xdg-open can block for as long as the browser lives, and the daemon
# waits on us to paint the pad green.
setsid xdg-open "$url" >/dev/null 2>&1 &
