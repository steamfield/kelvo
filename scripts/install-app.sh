#!/usr/bin/env bash
# Shared install/update step for local builds and the curl installer.
set -euo pipefail

APP="${1:?usage: install-app.sh path/to/StatGlass.app}"
DEST="$HOME/Applications/StatGlass.app"

# Finish copying and verify the replacement before interrupting the running app.
mkdir -p "$HOME/Applications"
STAGING="$(mktemp -d "$HOME/Applications/.statglass-install.XXXXXX")"
trap 'rm -rf "$STAGING"' EXIT
ditto "$APP" "$STAGING/StatGlass.app"
codesign --verify --deep --strict "$STAGING/StatGlass.app"

# Only stop this user's StatGlass, and allow time for it to exit before replacing files.
if pgrep -u "$UID" -x statglass >/dev/null; then
  echo "statglass: quitting the running StatGlass."
  pkill -u "$UID" -x statglass || true # It may have exited since pgrep.
  for ((attempt = 0; attempt < 50; attempt++)); do
    if ! pgrep -u "$UID" -x statglass >/dev/null; then break; fi
    sleep 0.2
  done
  if pgrep -u "$UID" -x statglass >/dev/null; then
    echo "statglass: StatGlass did not quit within 10 seconds; leaving the installed app in place." >&2
    exit 1
  fi
fi

rm -rf "$DEST"
mv "$STAGING/StatGlass.app" "$DEST"
echo "statglass: installed $DEST."

# Link the command-line tool onto PATH (D-126). The link points into the bundle, so
# updates keep it current. Only a link to a StatGlass CLI is replaced; any other link,
# file or directory at the name is left alone. Never fatal: the app is installed.
link_cli() {
  local bin_dir="$HOME/.local/bin"
  local link="$bin_dir/statglass"
  local target="$DEST/Contents/MacOS/statglass-cli"
  local manual="To use this build: ln -sf \"$target\" \"$link\""
  [ -x "$target" ] || return 0
  if [ -L "$link" ]; then
    local current
    current="$(readlink "$link")" || current=""
    case "$current" in
      *.app/Contents/MacOS/statglass-cli) ;;
      *)
        echo "statglass: $link points to $current, not StatGlass; left it alone. $manual" >&2
        return 0
        ;;
    esac
  elif [ -e "$link" ]; then
    echo "statglass: $link exists and is not a link; left it alone. $manual" >&2
    return 0
  fi
  if ! mkdir -p "$bin_dir" 2>/dev/null; then
    echo "statglass: could not create $bin_dir; the command-line tool is not linked." >&2
    return 0
  fi
  if ! command -v python3 >/dev/null; then
    echo "statglass: python3 not found, so the command-line tool is not linked. $manual" >&2
    return 0
  fi
  # A temporary link renamed over the name, so it is never absent or half-made. Not
  # `mv`: BSD mv moves into a directory that a symlink at the destination points to.
  local tmp="$bin_dir/.statglass.$$.tmp"
  rm -f "$tmp"
  if ! ln -s "$target" "$tmp" 2>/dev/null ||
    ! python3 -c 'import os, sys; os.replace(sys.argv[1], sys.argv[2])' "$tmp" "$link"; then
    rm -f "$tmp"
    echo "statglass: could not link $link; the command-line tool is not linked. $manual" >&2
    return 0
  fi
  echo "statglass: linked $link to the bundled command-line tool."
  case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) echo "statglass: $bin_dir is not on PATH; add it to your shell profile to run statglass." >&2 ;;
  esac
}
link_cli || echo "statglass: linking the command-line tool failed; continuing." >&2
open "$DEST"
