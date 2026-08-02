#!/usr/bin/env bash
# check-bs-ls-perf.sh — run the `bs ls` performance benchmark and enforce the
# SLOs documented as a requirement in
# openspec/changes/add-bs-status-command/specs/worktree-list/spec.md
# ("`bs list` meets a documented performance SLO, calibrated per slot-state
# scenario"):
#
#   1. All-locked / all-dirty scenarios (no or partial subprocess fan-out):
#      p95 @ 50 slots <= 100ms; scaling ratio p95@50/p95@5 <= 6.0x.
#   2. All-available scenario (full subprocess fan-out — one `git status` +
#      one `lsof` per slot): tracked and reported, but not gated on a fixed
#      absolute bound; only checked against a generous scaling-regression
#      bound so an unexpected additional cost beyond the inherent
#      `lsof`/`git status` floor is still caught.
#
# Usage: scripts/check-bs-ls-perf.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

P95_MAX_MS=100
SCALING_MAX_RATIO=6.0
# Generous scaling-regression bound for the all-available scenario: its cost
# is dominated by one `git status` + one `lsof` subprocess spawn per slot,
# which scales close to linearly with pool size by construction. This bound
# exists only to catch a regression *beyond* that expected linear-ish growth,
# not to demand sub-linear scaling that isn't achievable here.
AVAILABLE_SCALING_MAX_RATIO=15

echo "Running bs_ls benchmark…"
cargo bench --bench bs_ls -- --noplot

# p95_ns_for_bench <criterion-group-name> <bench-name>
# Computes the p95 per-iteration latency (nanoseconds) from criterion's raw
# sample.json (per-iteration time = times[i] / iters[i]), since criterion's
# estimates.json only reports mean/median/slope, not percentiles.
p95_ns_for_bench() {
  local sample_file="target/criterion/$1/$2/new/sample.json"
  if [[ ! -f "$sample_file" ]]; then
    echo "error: missing benchmark output $sample_file" >&2
    exit 1
  fi
  jq '
    . as $d
    | [range(0; ($d.iters | length)) | ($d.times[.] / $d.iters[.])] as $per_iter
    | ($per_iter | sort) as $sorted
    | ($sorted | length) as $n
    | $sorted[(($n - 1) * 0.95 | floor)]
  ' "$sample_file"
}

ns_to_ms() { echo "scale=3; $1 / 1000000" | bc; }

fail=0

# check_gated_scenario <group> <label>
# Enforces both the absolute p95@50 bound and the p95@50/p95@5 scaling bound.
check_gated_scenario() {
  local group="$1" label="$2"
  local p95_5_ns p95_50_ns p95_5_ms p95_50_ms ratio

  p95_5_ns="$(p95_ns_for_bench "$group" "5_slots")"
  p95_50_ns="$(p95_ns_for_bench "$group" "50_slots")"
  p95_5_ms="$(ns_to_ms "$p95_5_ns")"
  p95_50_ms="$(ns_to_ms "$p95_50_ns")"
  ratio="$(echo "scale=3; $p95_50_ns / $p95_5_ns" | bc)"

  echo ""
  echo "${label}:"
  echo "  p95 @  5 slots: ${p95_5_ms}ms"
  echo "  p95 @ 50 slots: ${p95_50_ms}ms  (limit: ${P95_MAX_MS}ms)"
  echo "  scaling ratio (50/5): ${ratio}x  (limit: ${SCALING_MAX_RATIO}x)"

  if (( $(echo "$p95_50_ms > $P95_MAX_MS" | bc) )); then
    echo "FAIL: ${label} p95 @ 50 slots (${p95_50_ms}ms) exceeds SLO of ${P95_MAX_MS}ms" >&2
    fail=1
  fi
  if (( $(echo "$ratio > $SCALING_MAX_RATIO" | bc) )); then
    echo "FAIL: ${label} scaling ratio (${ratio}x) exceeds SLO of ${SCALING_MAX_RATIO}x" >&2
    fail=1
  fi
}

# check_available_scenario <group> <label>
# Reports p95 at each pool size; only fails on a scaling-regression bound,
# not an absolute one (see AVAILABLE_SCALING_MAX_RATIO above).
check_available_scenario() {
  local group="$1" label="$2"
  local p95_5_ns p95_50_ns p95_5_ms p95_50_ms ratio

  p95_5_ns="$(p95_ns_for_bench "$group" "5_slots")"
  p95_50_ns="$(p95_ns_for_bench "$group" "50_slots")"
  p95_5_ms="$(ns_to_ms "$p95_5_ns")"
  p95_50_ms="$(ns_to_ms "$p95_50_ns")"
  ratio="$(echo "scale=3; $p95_50_ns / $p95_5_ns" | bc)"

  echo ""
  echo "${label} (reported, not absolute-bound gated):"
  echo "  p95 @  5 slots: ${p95_5_ms}ms"
  echo "  p95 @ 50 slots: ${p95_50_ms}ms"
  echo "  scaling ratio (50/5): ${ratio}x  (regression limit: ${AVAILABLE_SCALING_MAX_RATIO}x)"

  if (( $(echo "$ratio > $AVAILABLE_SCALING_MAX_RATIO" | bc) )); then
    echo "FAIL: ${label} scaling ratio (${ratio}x) exceeds the regression bound of ${AVAILABLE_SCALING_MAX_RATIO}x" >&2
    fail=1
  fi
}

check_gated_scenario "classify_slot_status_all_locked" "All-locked scenario"
check_gated_scenario "classify_slot_status_all_dirty" "All-dirty scenario"
check_available_scenario "classify_slot_status_all_available" "All-available scenario"

echo ""
if [[ "$fail" -ne 0 ]]; then
  echo "bs ls performance regression detected — see" >&2
  echo "openspec/changes/add-bs-status-command/specs/worktree-list/spec.md" >&2
  exit 1
fi

echo "OK: bs ls performance within SLOs."
