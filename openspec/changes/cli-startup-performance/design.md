## Context

`bs get` and `bs list` are intended to be millisecond-fast commands used inside
shell expressions like `cd $(bs get)`. In practice, on repositories with large
file trees (node_modules, Rust target/, etc.) they can take 3–15+ seconds
because every pool slot is probed with `lsof +D` — a recursive filesystem
traversal that is O(number of files under the slot). Profiling on a 40 k-file JS
repo shows each `lsof +D` call takes 1.1–1.3 s; with three managed slots that
alone accounts for 3–4 s of wall-clock latency before any git operation.

Two further process-spawn costs compound the problem: `git rev-parse HEAD` and
`git rev-parse --git-common-dir` are always spawned as separate subprocesses
even though a single `git rev-parse HEAD --git-common-dir` invocation returns
both values, and `git worktree prune` runs unconditionally on every `bs get`
regardless of whether there are any stale entries to clean up.

All changes land in `src/worktree/mod.rs`; no new dependencies are introduced.

## Goals / Non-Goals

**Goals:**

- Reduce `bs get` latency to < 200 ms on any repository (including those with
  `node_modules` or large `target/` directories).
- Reduce `bs list` latency proportionally.
- Preserve the correctness guarantee that a slot with a live shell session (CWD
  = slot root) is not handed to a second caller.
- Maintain backward compatibility for all existing CLI flags and outputs.

**Non-Goals:**

- Detecting editors with files open deep in subdirectories (currently caught by
  `lsof +D` but not by `lsof +d`). The `git status --porcelain` check already
  covers the case where those editors have created dirty state.
- Persistent daemon or server architecture.
- Cross-platform process listing beyond what `lsof` provides on macOS/Linux.

## Decisions

### Decision 1: Replace `lsof +D` (recursive) with `lsof +d` (non-recursive)

**Why this over alternatives:**

| Alternative                     | Why rejected                                                                        |
| ------------------------------- | ----------------------------------------------------------------------------------- |
| Keep `lsof +D` but run it async | Doesn't help `bs get` which blocks on the result                                    |
| Lock-file + `kill -0 <pid>`     | Requires managing lock-file lifecycle; PPID under `$()` may be a transient subshell |
| Skip lsof entirely              | Loses detection of shells cd'd into a slot                                          |
| `lsof +d` (non-recursive)       | **Chosen** — ~8× faster (40 ms vs 300–1300 ms), still catches CWD on slot root      |

`lsof +d <dir>` lists processes with any open file descriptor _directly_ in
`<dir>` (not in subdirectories). A shell with `CWD = <slot>` produces a `cwd`
entry at exactly `<dir>`, which `+d` captures. An editor with a buffer open at
`<dir>/src/main.rs` will **not** be captured — but if that editor has made
changes, `git status --porcelain` will report dirty state and the slot will be
classified `InUse` regardless.

The semantic gap (clean slot with editor buffer in a subdirectory) is accepted
as a known trade-off. It is documented in the spec.

### Decision 2: Merge `git rev-parse HEAD --git-common-dir` into one call

`git rev-parse` accepts multiple arguments and outputs one result per line.
Running `git rev-parse HEAD --git-common-dir` replaces two process spawns with
one. A new helper `resolve_head_and_common_dir() -> Result<(String, PathBuf)>`
encapsulates parsing, keeping `resolve_head()` and `git_common_dir()` as thin
wrappers for callers that still need them individually. `get_worktree()` is
updated to call the merged helper directly.

### Decision 3: Conditional `git worktree prune` — only when stale entries detected

The current `find_available_slot` calls `prune_worktrees()` unconditionally. The
revised logic inspects the output of `git worktree list --porcelain` (already
fetched by `list_pool_worktrees`) and invokes `git worktree prune` only when at
least one registered slot path does not exist on disk. In the common case (all
slots present) the prune subprocess is skipped entirely.

### Decision 4: Parallelise per-slot checks in `list_worktrees_status`

After `list_pool_worktrees` returns the set of slots, each slot requires two
blocking operations (`lsof +d` + `git status --porcelain`). These are
independent and can be executed concurrently using `std::thread::spawn`. A `Vec`
of `JoinHandle`s is collected and joined in order; results are assembled into
the final `Vec<WorktreeListEntry>`. No new crate dependency is required. This
makes wall-clock time proportional to the slowest single slot rather than the
sum.

## Risks / Trade-offs

- **Editor-file gap**: An editor with a file open inside a slot subdirectory
  (but no dirty git state) will not be detected as "in use" with `lsof +d`. The
  slot could be reset under the editor. Mitigation: the `git checkout --detach`
  reset only changes the index/HEAD; the on-disk file the editor has open is not
  deleted or altered unless git needs to update that file. The editor will see a
  reload prompt. This is the same behaviour that would occur if another terminal
  ran `git checkout` manually.

- **Thread overhead for small pools**: Spawning OS threads for one or two slots
  costs more than running them serially. Mitigation: the thread-spawn cost (~50
  µs) is negligible compared to the lsof and git calls (~40 ms each). Even for a
  single-slot pool the overhead is invisible.

- **`lsof` availability**: Both `lsof +D` and `lsof +d` require `lsof` on PATH.
  The existing "lsof not found" error path is unchanged.

## Migration Plan

1. Update `run_lsof` and `run_lsof_count` to use `+d` flag.
2. Add `resolve_head_and_common_dir` helper; update `get_worktree` to call it.
3. Move prune logic inside `find_available_slot` to be conditional on stale
   entries.
4. Parallelise the per-slot loop in `list_worktrees_status`.
5. Update specs (`worktree-open-file-detection`, `worktree-get`,
   `worktree-list`).
6. Update unit tests that assert on `+D` flag string.
7. Ship; no migration needed for users (no persistent state format change).

No rollback complexity — the change is entirely in-process logic with no
external state mutations.

## Open Questions

- Should `bs list` show a visual indicator when a slot was detected by lsof
  (implying a live shell) vs. detected by git dirty state only? Currently the
  stats column (`⚙N`) conflates both. Deferred to a future polish change.
