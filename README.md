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

open processes:
  1234  node

uncommitted changes (1):
  M  README.md
```

For more information, run `bs help`.

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
