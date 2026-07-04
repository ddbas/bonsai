#!/usr/bin/env bash
# view-proposal.sh — render an OpenSpec change to HTML and open an index in the browser.
# Usage: mise run openspec:open <change-id>
#        ./scripts/view-proposal.sh <change-id>

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <change-id>" >&2
  echo "Example: $0 testcontainers-e2e-isolation" >&2
  exit 1
fi

CHANGE_ID="$1"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHANGE_DIR="$REPO_ROOT/openspec/changes/$CHANGE_ID"

if [[ ! -d "$CHANGE_DIR" ]]; then
  echo "Error: change not found at $CHANGE_DIR" >&2
  echo "" >&2
  echo "Available changes:" >&2
  for d in "$REPO_ROOT/openspec/changes"/*/; do
    name="$(basename "$d")"
    [[ "$name" != "archive" ]] && echo "  $name" >&2
  done
  exit 1
fi

TMP_DIR="/tmp/openspec-${CHANGE_ID}"
rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

# --- Pandoc helper -----------------------------------------------------------
# render_md <src.md> <out.html> <title>
render_md() {
  local src="$1" out="$2" title="$3"
  pandoc \
    --standalone \
    --metadata title="$title" \
    --syntax-highlighting=pygments \
    "$src" -o "$out"
}

# --- Collect documents -------------------------------------------------------
declare -a DOCS   # parallel arrays: label, filename (relative to TMP_DIR)
declare -a FILES

add_doc() {
  local label="$1" src="$2" filename="$3"
  render_md "$src" "$TMP_DIR/$filename" "$label — $CHANGE_ID"
  DOCS+=("$label")
  FILES+=("$filename")
}

[[ -f "$CHANGE_DIR/proposal.md" ]] && add_doc "Proposal"  "$CHANGE_DIR/proposal.md" "proposal.html"
[[ -f "$CHANGE_DIR/design.md"   ]] && add_doc "Design"    "$CHANGE_DIR/design.md"   "design.html"
[[ -f "$CHANGE_DIR/tasks.md"    ]] && add_doc "Tasks"     "$CHANGE_DIR/tasks.md"    "tasks.html"

# Specs — one per subdirectory under specs/
if [[ -d "$CHANGE_DIR/specs" ]]; then
  while IFS= read -r -d '' spec_file; do
    spec_name="$(basename "$(dirname "$spec_file")")"
    out_file="spec-${spec_name}.html"
    add_doc "Spec: $spec_name" "$spec_file" "$out_file"
  done < <(find "$CHANGE_DIR/specs" -name "spec.md" -print0 | sort -z)
fi

# --- Build index page --------------------------------------------------------
INDEX="$TMP_DIR/index.html"

{
  cat <<'HEADER'
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/github-markdown-css/github-markdown-light.css">
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
           max-width: 680px; margin: 3rem auto; padding: 0 1.5rem;
           background: #fff; color: #1f2328; }
    h1   { font-size: 1.5rem; margin-bottom: 0.25rem; }
    p.id { font-family: monospace; color: #57606a; margin-top: 0; }
    ul   { list-style: none; padding: 0; margin-top: 2rem; }
    li   { margin: 0.6rem 0; }
    a    { display: inline-flex; align-items: center; gap: 0.5rem;
           font-size: 1rem; color: #0969da; text-decoration: none; }
    a:hover { text-decoration: underline; }
    .icon { font-size: 1.1rem; }
  </style>
</head>
<body>
HEADER

  echo "  <h1>OpenSpec Change</h1>"
  echo "  <p class=\"id\">$CHANGE_ID</p>"
  echo "  <ul>"

  for i in "${!DOCS[@]}"; do
    label="${DOCS[$i]}"
    file="${FILES[$i]}"
    case "$label" in
      Proposal*)  icon="📋" ;;
      Design*)    icon="🏗️" ;;
      Tasks*)     icon="✅" ;;
      Spec:*)     icon="📄" ;;
      *)          icon="📝" ;;
    esac
    echo "    <li><a href=\"$file\"><span class=\"icon\">$icon</span>$label</a></li>"
  done

  cat <<'FOOTER'
  </ul>
</body>
</html>
FOOTER
} > "$INDEX"

echo "Opening OpenSpec change '$CHANGE_ID'…"
open "$INDEX"
