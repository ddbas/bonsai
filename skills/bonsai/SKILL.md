---
name: bonsai
description:
  Use this tool when you need to perform operations on git worktrees, such as
  creating new worktrees, listing existing ones, or retrieving specific worktree
  information.
argument-hint: <git worktree query>
author: David Bastien (ddbas)
---

# Bonsai CLI (`bs`)

The `bs` CLI manages git worktrees. It allows you to quickly acquire worktrees
from an existing pool of worktrees. It tracks which worktrees are currently in
use based on their git status and list of open files. It reuses available
worktrees when possible in order to minimize the overhead of creating new
worktrees.

## Usage

Get an available worktree from the pool with a detached HEAD state. Returns the
path to the worktree.

```sh
bs get
# 🌳 /Users/username/.bonsai/repo-name/84cd7c88
```

Checkout a worktree with an existing branch.

```sh
bs get <branch-name>
# 🌳 /Users/username/.bonsai/bonsai/f6a6bb50  (<branch-name>)
```

Use `-b` or `-B` to create or recreate (respectively) a branch when getting a
worktree. Use `--tmux-session [<NAME>]` to create a new tmux session in the
worktree, and `--no-attach` to prevent automatically attaching to the session.

Get current worktree (if any)

```sh
bs current
# 🌳 ~/.bonsai/repo-name/2a836a6f  (<branch-name>)
```

Get all worktrees in the pool

```sh
bs list
# ▶ in use     ~/.bonsai/repo-name/2a836a6f (branch-name)
#   available  ~/.bonsai/repo-name/84cd7c88 (other-branch)
#   available  ~/.bonsai/repo-name/f6a6bb50
```

Get the worktree status (i.e. whether the worktree is in use or available, its
git status, and the list of open files)

```sh
bs status
# 🌳 ~/.bonsai/repo-name/2a836a6f  (branch-name)
# status: in use
#
# open processes:
#   2023  nvim
#   2233  node
#
# uncommitted changes (1):
#    M  README.md
#
# untracked files (1):
#   ??  foo/
```

Pass a worktree path to `bs status <worktree-path>` to get the status of a
specific worktree.
