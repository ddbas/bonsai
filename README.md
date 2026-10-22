<div align="center">

# 🌳 Bonsai

<h3 align="center">Instant, clean git worktrees on demand.</h3>

Bonsai manages a pool of git worktrees so you can jump between tasks without
ever stashing, committing half-finished work, or waiting for a fresh clone. Run
`bs` and you get a clean slot at the current HEAD — ready to use, already on
disk. When you're done, put it back in the pool to be recycled later.

</div>

## 🚀 Quick Start

```
$ bs get
🌳 /Users/myuser/.bonsai/myrepo/973f965e
```

```
$ bs list
▶ in use     ~/.bonsai/myrepo/a401c509 (some-branch)
  locked     ~/.bonsai/myrepo/973f965e (some-other-branch)
  available  ~/.bonsai/myrepo/a05e61c3
```

```
$ bs status ~/.bonsai/myrepo/b5c6d7e8
🌳 ~/.bonsai/myrepo/b5c6d7e8  (some-branch)
status: in use
branch state: 2 commits ahead of origin/main, no upstream configured (push with `-u` to enable merged-branch detection)

uncommitted changes (1):
  M  README.md
```

For more information, run `bs help`.

## When is a slot available?

A slot is reused by `bs get` only when resetting it cannot lose work. It is
classified, in priority order, as:

1. `locked` — locked with `bs lock`.
2. `in use` — uncommitted or untracked files (including files inside new
   untracked directories), or a branch with unmerged work.
3. `available` — clean, and its branch has nothing unmerged.

"Nothing unmerged" is judged from **local git state only** (no network, no
fetch), against the default ref (`origin/HEAD`, else `origin/main` /
`origin/master`; local `main` / `master` only when there is no `origin`):

- HEAD is already reachable from the default ref (fresh branch, fast-forward or
  merge-commit merge): available.
- Squash- or rebase-merged: the branch has upstream config, its remote-tracking
  branch is gone, and its content is already on the default ref: available.
- Anything else (unpushed commits, remote branch still exists, detached HEAD
  with own commits, default ref unknown): in use. `bs status` says why.

Notes:

- Running processes are **not** considered. A clean slot with an editor or agent
  running in it can be reused; use `bs lock` to protect active slots.
- Remote-tracking refs are only as fresh as your last fetch. Merged branches
  stay `in use` until `git fetch --prune`.
- Push with `git push -u` (or set `push.autoSetupRemote`) so squash/rebase
  merges can be recognized.

## Install

**Prerequisites:** [mise](https://mise.jdx.dev/).

```bash
git clone https://github.com/ddbas/bonsai.git
cd bonsai
mise run install
```

Make sure `~/.local/bin` is on your `PATH`.

## Agent Skill

```bash
npx skills add ddbas/bonsai --skill bonsai
```
