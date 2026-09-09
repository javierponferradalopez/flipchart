#!/bin/bash
# Packs the box: the five files of ADR-0013 and ADR-0018 and nothing else, into
# an Info-ZIP zip, and writes the path of the zip to stdout.
#
#   package.sh <version-or-tag> <binary> <destination>
#
# Info-ZIP (`zip`) is part of the contract, not a convenience: it is the packer
# that writes `version made by == 3` with the Unix modes intact, and that is
# where the `100755` the binary reaches the user's machine with comes from —the
# host reads the external attributes and does `chmod(mode & 0o777)` when there
# is an execute bit—. Never from the Finder, which slips in `__MACOSX/` and
# `.DS_Store`.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
readonly ROOT
readonly MANIFEST=publishing/box/.claude-plugin/plugin.json

# shellcheck source=json.sh
. "$ROOT/publishing/json.sh"

die() {
  printf 'package: %s\n' "$1" >&2
  exit 1
}

the_version_in_cargo_toml() {
  local line
  line=$(sed -n '/^\[package\]/,/^\[/p' "$ROOT/Cargo.toml" | grep -m1 '^version') \
    || die 'Cargo.toml declares no version in its [package]'
  line=${line#*\"}
  printf '%s' "${line%%\"*}"
}

[ $# -eq 3 ] || die 'usage: package.sh <version-or-tag> <binary> <destination>'
readonly VERSION=${1#v}
readonly BINARY=$2
readonly DESTINATION=$3

[ -f "$BINARY" ] || die "there is no binary at $BINARY"

# The version that rules is the one in the `plugin.json` inside the zip —the
# `/plugin` UI does `manifest.version ?? "unknown"`—, and the tag's is the one
# the catalog is going to show. That they match is not checked out of tidiness:
# publishing them out of line leaves the user on a version other than the one
# they installed.
declared=$(field version "$(grep -m1 '"version"' "$ROOT/$MANIFEST")") \
  || die "$MANIFEST declares no version"
[ "$declared" = "$VERSION" ] || die "$MANIFEST declares $declared and the tag says $VERSION"
from_cargo=$(the_version_in_cargo_toml)
[ "$from_cargo" = "$VERSION" ] || die "Cargo.toml declares $from_cargo and the tag says $VERSION"

# The five files are copied one by one and there is no `cp -R` of a whole
# directory, which is what would let in a `.DS_Store` from the working tree or a
# second skill someone added along the way. The box is closed —four files by
# ADR-0013, the one skill of ADR-0018— so there is no need to check that it is:
# there is no way in for a sixth file.
readonly BOX="$DESTINATION/box"
rm -rf "$BOX"
mkdir -p "$BOX/.claude-plugin"
cp "$ROOT/publishing/box/.claude-plugin/plugin.json" "$BOX/.claude-plugin/plugin.json"
cp "$ROOT/publishing/box/.mcp.json" "$BOX/.mcp.json"
cp "$ROOT/launcher.sh" "$BOX/launcher.sh"
cp "$BINARY" "$BOX/flipchart"
mkdir -p "$BOX/skills/choosing-the-family"
cp "$ROOT/publishing/box/skills/choosing-the-family/SKILL.md" \
  "$BOX/skills/choosing-the-family/SKILL.md"
chmod 755 "$BOX/launcher.sh" "$BOX/flipchart"
chmod 644 "$BOX/.claude-plugin/plugin.json" "$BOX/.mcp.json" \
  "$BOX/skills/choosing-the-family/SKILL.md"

readonly ZIP="$DESTINATION/flipchart-$VERSION.zip"
rm -f "$ZIP"
( cd "$BOX" && zip -q -r -X "$ZIP" . )

# That the zip carries Unix attributes is not promised by the host's schema: it
# is promised by the packer, and this is what checks it instead of trusting it.
# Without them the binary would arrive with no execute bit, and the Launcher's
# `chmod +x` would stop being a backstop and become the mechanism.
looked_at=$(unzip -Z "$ZIP")
for executable in flipchart launcher.sh; do
  grep -qE "^-rwxr-xr-x +[0-9.]+ unx .* $executable\$" <<<"$looked_at" \
    || die "$executable does not travel as a Unix -rwxr-xr-x:
$looked_at"
done

# The archive ceiling is 256 MiB and it **has no valve**: there is no
# environment variable that widens it, so going past it does not degrade, it
# leaves the plugin with no way to install. The margin is enormous today and
# what eats it are the dependencies, which is exactly what nobody looks at when
# adding one.
readonly ARCHIVE_CEILING=$((256 * 1024 * 1024))
bytes=$(stat -f %z "$ZIP")
[ "$bytes" -le "$ARCHIVE_CEILING" ] \
  || die "the zip is $bytes bytes and the archive ceiling is $ARCHIVE_CEILING, with no valve"

printf 'package: %s bytes, ceiling %s\n' "$bytes" "$ARCHIVE_CEILING" >&2
printf 'package: %s\n' "$(cd "$DESTINATION" && shasum -a 256 "$(basename "$ZIP")")" >&2
printf '%s\n' "$ZIP"
