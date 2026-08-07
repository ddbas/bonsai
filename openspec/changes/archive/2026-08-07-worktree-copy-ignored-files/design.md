## Context

`bs get` provisions a pool slot (a linked git worktree under
`~/.bonsai/<repo-slug>/<uuid>/`) by resetting an existing clean slot or creating
a brand-new one via `git worktree add`. Both paths only ever bring across
git-tracked content plus whatever `git checkout`/`git worktree add` populate.
Git-ignored files (`.env`, local overrides, IDE settings, etc.) that a developer
keeps in their "origin" worktree — the worktree `bs get` was invoked from, which
may be the main worktree or another pool slot — never make it into the new slot.

Bonsai has no configuration file or format of its own today; all repo-scoping
and root-resolution logic reads from git plumbing commands or the `BONSAI_ROOT`
env var. Introducing a bespoke `bonsai.toml`/`.bonsairc` config file would add a
second configuration system to maintain (parsing, validation,
discovery/precedence rules, docs) when git already solves this problem for
arbitrary tools via `git config` with vendor namespaces.

## Goals / Non-Goals

**Goals:**

- Let users declare, once, a list of git-ignored file names/relative paths that
  should be copied from the origin worktree into every newly provisioned or
  reused pool slot.
- Reuse git's own config system (`git config`), scoped under a `bonsai.*`
  namespace, so users can set this per-repo (`.git/config`) or globally
  (`~/.gitconfig`) with zero new tooling.
- Make missing source files a silent no-op — the list is best-effort, not a
  strict manifest.
- Apply consistently regardless of whether the slot was freshly created
  (`git worktree add`) or reused/reset (existing clean slot).

**Non-Goals:**

- Not building a generic file-sync/watch mechanism — this is a one-shot copy at
  provisioning time only.
- Not copying directories recursively or supporting globs in this change;
  entries are literal relative file paths/names.
- Not validating that listed entries are actually git-ignored (copying a tracked
  file that happens to be listed is harmless but out of scope to detect/warn
  about).
- Not adding any new CLI flags to `bs get`; behavior is 100% config-driven.

## Decisions

### Use `git config --get-all bonsai.copy` for configuration

Git's config system already supports multi-valued keys, per-repo vs. global
scoping, and a well-known namespacing convention (`<tool>.<key>`). Using
`bonsai.copy` (repeatable) avoids inventing a new file format, a new discovery
path, and new precedence rules. `git config --get-all bonsai.copy` returns each
configured value on its own line, in file order, which maps directly to a
`Vec<String>` of relative file paths.

**Alternatives considered:**

- A dedicated `.bonsai.toml`/`.bonsairc` file checked into or ignored by the
  repo: rejected — duplicates git's existing config machinery, needs its own
  parser/dependency, and raises questions about where it lives relative to
  worktrees (main repo root vs. slot vs. `$HOME`).
- An environment variable (`BONSAI_COPY`, colon-separated): rejected — harder to
  set persistently per-repo, no natural list semantics, inconsistent with how
  the rest of bonsai's per-repo config would be discovered (there is none yet).

### Resolve config from the origin worktree, not the slot

The list of files to copy is read via `git config --get-all bonsai.copy` run
from the **origin** worktree (the CWD `bs get` was invoked from), before/at the
same time as HEAD resolution. This must happen before the new/reused slot is
created, since the source files also live in the origin worktree and
`bonsai.copy` scoping should reflect wherever the user actually configured it
(typically the main worktree's `.git/config`, which is shared by all linked
worktrees via `--git-common-dir`, so reading it from any worktree yields the
same repo-level answer). Global (`~/.gitconfig`) entries apply uniformly
regardless of CWD.

### Copy step runs last, after slot provisioning and branch checkout

Sequencing: (1) resolve HEAD + repo slug, (2) find/create slot, (3) reset or add
the slot (including branch checkout), (4) copy configured files from the origin
worktree into the slot. Running the copy last ensures it does not interfere with
git's own checkout mechanics and that copied files always land in the final slot
state, not into an intermediate state that a `git checkout` could subsequently
touch.

### Missing source files are silently skipped

Each entry in `bonsai.copy` is looked up relative to the origin worktree root.
If the source path does not exist, it is skipped without error or warning — the
config list is inherently best-effort (e.g. a list shared across multiple
repos/branches where not every entry always exists). Other copy failures (e.g.
permission errors, destination write failures) still surface as errors, since
those indicate a real problem rather than an absent optional file.

### File paths only; parent directories created as needed

`bonsai.copy` entries are relative paths (e.g. `.env`, `config/local.json`).
When copying, destination parent directories are created (`create_dir_all`) if
they don't already exist in the slot, since a fresh `git worktree add` slot
won't have untracked subdirectories that only ever held ignored files.

## Risks / Trade-offs

- [Symlink or special-file entries in `bonsai.copy`] → Copying follows regular
  file semantics (`std::fs::copy`); symlinks are not specially
  resolved/preserved as symlinks in this iteration. Acceptable for the common
  case (dotfiles, JSON/env configs); documented as a known limitation.
- [Large files in the copy list slow down every `bs get`] → Copies are
  synchronous and best-effort per entry; the risk is accepted since the feature
  is opt-in and users control the list.
- [Divergent config between global and local scope confuses users about which
  files get copied] → `git config --get-all` merges global + local per git's
  normal precedence rules (both are additive for multi-valued keys), which is
  the same behavior users already expect from git; no special bonsai-side
  merging logic is introduced.
- [Silent skip on missing files could mask a typo in a configured path] →
  Accepted per explicit product requirement; users can verify their config with
  `git config --get-all bonsai.copy` directly.
