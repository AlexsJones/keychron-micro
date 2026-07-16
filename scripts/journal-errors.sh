#!/usr/bin/env bash
# Medical cross key: this boot's error-level journal entries.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$here/lib/term.sh" journalctl -p err -b --no-pager
