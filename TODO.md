# TODO

- [ ] `lock` & `unlock` subcommand: lock and unlock a worktree (with an optional
      reason)
- [ ] `enter` subcommand: `cd` into a worktree
  - [ ] `--enter` option on `get` subcommand: `bs --enter` and `bs get --enter`
        should `cd` into the worktree after getting it
- [ ] `-b` and `-B` options on `get` subcommand: create a new branch when
      getting a worktree
- [ ] `config` subcommand: manage bonsai configuration directly in git
      (`[bonsai]` section).
  - [ ] Delegate to `git config` for all reads and writes. Include `--global`
        and `--local` flags.
  - [ ] `enter_command` config: set the command to run when entering a worktree
        (default: `cd`)
  - [ ] `get_enter` config: enter a worktree after getting it (default: `false`)
- [ ] Show current worktree in `list` subcommand
- [ ] `current` subcommand: show the current worktree
- [ ] `prune` subcommand
  - [ ] Reset the `HEAD` of all "available" worktrees to the `HEAD` of `main`
  - [ ] Optional and dynamic sized worktree argument to prune only specified
        worktrees
  - [ ] `--force` option to prune "in use" worktrees
