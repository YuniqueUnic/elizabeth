#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION_FILE="$ROOT_DIR/version.txt"
README_FILE="$ROOT_DIR/README.md"
COMPOSE_FILE="$ROOT_DIR/docker-compose.yml"
RELEASE_CONFIG_FILE="$ROOT_DIR/release-please-config.json"

fail() {
  echo "[release-version-sync] $*" >&2
  exit 1
}

VERSION="$(tr -d '[:space:]' < "$VERSION_FILE")"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]] ||
  fail "version.txt contains an invalid semantic version: '$VERSION'"

assert_marker_pair() {
  local file="$1"
  local start_count end_count
  start_count="$(grep -c 'x-release-please-start-version' "$file" || true)"
  end_count="$(grep -c 'x-release-please-end' "$file" || true)"

  [ "$start_count" -eq 1 ] ||
    fail "$(basename "$file") must contain exactly one version start marker"
  [ "$end_count" -eq 1 ] ||
    fail "$(basename "$file") must contain exactly one version end marker"
}

assert_marker_pair "$README_FILE"
assert_marker_pair "$COMPOSE_FILE"

README_VERSION="$(
  sed -n 's/^export ELIZABETH_VERSION=\([^[:space:]]*\)$/\1/p' "$README_FILE"
)"
COMPOSE_VERSION="$(
  sed -n 's|^[[:space:]]*image: ${ELIZABETH_IMAGE:-yunique001/elizabeth:\([^}]*\)}$|\1|p' \
    "$COMPOSE_FILE"
)"

[ -n "$README_VERSION" ] || fail "README.md is missing ELIZABETH_VERSION"
[ -n "$COMPOSE_VERSION" ] || fail "docker-compose.yml is missing the default image version"
[ "$README_VERSION" = "$VERSION" ] ||
  fail "README.md uses $README_VERSION but version.txt uses $VERSION"
[ "$COMPOSE_VERSION" = "$VERSION" ] ||
  fail "docker-compose.yml uses $COMPOSE_VERSION but version.txt uses $VERSION"

python3 - "$RELEASE_CONFIG_FILE" <<'PY'
import json
import pathlib
import sys

config_path = pathlib.Path(sys.argv[1])
with config_path.open(encoding="utf-8") as config_file:
    config = json.load(config_file)

extra_files = config["packages"]["."]["extra-files"]
generic_paths = {
    item["path"]
    for item in extra_files
    if isinstance(item, dict) and item.get("type") == "generic"
}
required_paths = {"README.md", "docker-compose.yml"}
missing_paths = required_paths - generic_paths
if missing_paths:
    missing = ", ".join(sorted(missing_paths))
    raise SystemExit(
        f"[release-version-sync] release-please generic extra-files missing: {missing}"
    )
PY

echo "[release-version-sync] README.md and docker-compose.yml match version $VERSION"
