<div align="center">

# 🌳 Bonsai

<h3 align="center">Instant, clean git worktrees on demand.</h3>

Bonsai manages a pool of git worktrees so you can jump between tasks without
ever stashing, committing half-finished work, or waiting for a fresh clone. Run
`bs` and you get a clean slot at the current HEAD — ready to use, already on
disk. When you're done, put it back in the pool; the next call reuses it in
milliseconds.

</div>

## Quick Start

```
$ bs get
🌳 ~/.bonsai/myrepo/a1b2c3d4
```

```
$ bs list
available  ~/.bonsai/myrepo/a1b2c3d4
in use     ~/.bonsai/myrepo/b5c6d7e8 (main)      ⚙2
in use     ~/.bonsai/myrepo/c9d0e1f2 (my-feature)
```

```
$ bs help
```

## Install

> **TBD** — installation instructions coming soon.
