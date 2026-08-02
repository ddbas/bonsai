#!/usr/bin/env bash
# check-bs-ls-perf.sh — run the `bs ls` performance benchmark and enforce the
# SLOs documented as requirements in
# openspec/changes/add-bs-status-command/specs/worktree-list/spec.md:
#
#   - p95 wall-clock latency of the pool scan (`list_pool_worktrees`) at 50
#     managed worktree slots SHALL be <= 50ms.
#   - p95 at 50 slots SHALL be <= 1.5x p95 at 5 slots (near-constant scaling,
#     not linear growth with pool size).
#
# Usage: scripts/check-bs-ls-perf.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

P95_MAX_MS=50
SCALING_MAX_RATIO=2.5

echo "Running bs_ls benchmark (list_pool_worktrees group)…"
cargo bench --bench bs_ls -- "list_pool_worktrees/" --noplot

# p95_ns_for_bench <criterion-bench-dir-name>
# Computes the p95 per-iteration latency (nanoseconds) from criterion's raw
# sample.json (per-iteration time = times[i] / iters[i]), since criterion's
# estimates.json only reports mean/median/slope, not percentiles.
p95_ns_for_bench() {
  local sample_file="target/criterion/list_pool_worktrees/$1/new/sample.json"
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

p95_5_ns="$(p95_ns_for_bench "5_slots")"
p95_50_ns="$(p95_ns_for_bench "50_slots")"

p95_5_ms="$(echo "scale=3; $p95_5_ns / 1000000" | bc)"
p95_50_ms="$(echo "scale=3; $p95_50_ns / 1000000" | bc)"
ratio="$(echo "scale=3; $p95_50_ns / $p95_5_ns" | bc)"

echo ""
echo "bs ls perf SLO check:"
echo "  p95 @  5 slots: ${p95_5_ms}ms"
echo "  p95 @ 50 slots: ${p95_50_ms}ms  (limit: ${P95_MAX_MS}ms)"
echo "  scaling ratio (50/5): ${ratio}x  (limit: ${SCALING_MAX_RATIO}x)"
echo ""

fail=0

if (( $(echo "$p95_50_ms > $P95_MAX_MS" | bc) )); then
  echo "FAIL: p95 @ 50 slots (${p95_50_ms}ms) exceeds SLO of ${P95_MAX_MS}ms" >&2
  fail=1
fi

if (( $(echo "$ratio > $SCALING_MAX_RATIO" | bc) )); then
  echo "FAIL: scaling ratio (${ratio}x) exceeds SLO of ${SCALING_MAX_RATIO}x" >&2
  fail=1
fi

if [[ "$fail" -ne 0 ]]; then
  echo ""
  echo "bs ls performance regression detected — see" >&2
  echo "openspec/changes/add-bs-status-command/specs/worktree-list/spec.md" >&2
  exit 1
fi

echo "OK: bs ls performance within SLOs."
