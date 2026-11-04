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
# updates keep it current. Only an existing symlink is replaced, never a regular file.
link_cli() {
  local bin_dir="$HOME/.local/bin"
  local link="$bin_dir/statglass"
  local target="$DEST/Contents/MacOS/statglass-cli"
  [ -x "$target" ] || return 0
  mkdir -p "$bin_dir"
  if [ -e "$link" ] && [ ! -L "$link" ]; then
    echo "statglass: $link exists and is not a link; left it alone. To use this build: ln -sf \"$target\" \"$link\"" >&2
    return 0
  fi
  ln -sfn "$target" "$link"
  echo "statglass: linked $link to the bundled command-line tool."
  case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) echo "statglass: $bin_dir is not on PATH; add it to your shell profile to run statglass." >&2 ;;
  esac
}
link_cli
open "$DEST"
