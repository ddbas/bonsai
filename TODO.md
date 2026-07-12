# TODO

- [ ] `lock` & `unlock` subcommand: lock and unlock a worktree (with an optional
      reason)
- [ ] `config` subcommand: manage bonsai configuration directly in git
      (`[bonsai]` section).
  - [ ] Delegate to `git config` for all reads and writes. Include `--global`
        and `--local` flags.
- [ ] Show current worktree in `list` subcommand
- [ ] `current` subcommand: show the current worktree
- [ ] `prune` subcommand
  - [ ] Reset the `HEAD` of all "available" worktrees to the `HEAD` of `main`
  - [ ] Optional and dynamic sized worktree argument to prune only specified
        worktrees
  - [ ] `--force` option to prune "in use" worktrees
- [ ] `prime` subcommand: a markdown output that agents can read to understand
      how to use bonsai
