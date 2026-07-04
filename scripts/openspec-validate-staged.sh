#!/usr/bin/env bash
# Validate only the openspec items (changes/specs) touched by staged files.
# Falls back to --all when staged files don't map to a named item
# (e.g. openspec/config.yaml).
#
# Usage: openspec-validate-staged.sh <staged-file> [<staged-file> ...]

set -euo pipefail

# Extract "change:<name>" or "spec:<name>" pairs from the staged paths.
# openspec/changes/<name>/... → change:<name>
# openspec/specs/<name>/...   → spec:<name>
items=$(
  printf '%s\n' "$@" \
    | awk -F'/' 'NF >= 3 && $2 == "changes" && $3 != "archive" { print "change:" $3 }
                 NF >= 3 && $2 == "specs"   { print "spec:"   $3 }' \
    | sort -u
)

if [ -z "$items" ]; then
  echo "openspec: no named items in staged paths — running --all"
  exec mise exec -- openspec validate --all --no-interactive
fi

echo "openspec: validating staged items: $(echo "$items" | tr '\n' ' ')"
while IFS=: read -r type name; do
  mise exec -- openspec validate "$name" --type "$type" --no-interactive
done <<< "$items"
