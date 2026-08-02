## 1. Top-level CLI help

- [ ] 1.1 Trim the top-level `about` string on `Cli` to a single concise
      sentence describing what `bs` does.
- [ ] 1.2 Rewrite the `--log-level` doc comment to state accepted values
      (`trace`, `debug`, `info`, `warn`, `error`), the default (`info`), and
      that it only affects the file log (not stdout/stderr), without explaining
      log-level semantics or verbosity trade-offs in prose.

## 2. `bs get` command and its arguments

- [ ] 2.1 Rewrite the `Get` variant's doc comment to a short summary (what it
      does, that it's the implicit default command) plus concise usage facts for
      `-b`/`-B`/positional `<branch>` mutual exclusivity — remove the prose
      walkthrough of internal branch-resolution edge cases (e.g. the
      already-checked-out-elsewhere case).
- [ ] 2.2 Rewrite the positional `branch` field's doc comment: what it does,
      that it requires an existing branch, and its mutual exclusivity with
      `-b`/`-B` — drop the edge-case explanation of resolving branches checked
      out in other pool slots.
- [ ] 2.3 Rewrite the `-b`/`new_branch` doc comment: what it does, default
      behavior (fails if branch exists), mutual exclusivity with `-B`.
- [ ] 2.4 Rewrite the `-B`/`reset_branch` doc comment: what it does (create or
      reset), mutual exclusivity with `-b`.
- [ ] 2.5 Rewrite the `--tmux-session` doc comment: what it does, default
      session name behavior when no value is given, that it requires `tmux` on
      `PATH` — drop the detailed naming-convention prose and internal
      requirement mechanics.
- [ ] 2.6 Rewrite the `--no-attach` doc comment: what it does, that it requires
      `--tmux-session` — drop the cleanup-mechanics aside.

## 3. Remaining subcommands (`list`, `current`, `help`, `lock`, `unlock`, `info`)

- [ ] 3.1 Rewrite the `List` variant's doc comment to a short summary of what it
      prints, keeping the color-coding legend concise (one line).
- [ ] 3.2 Rewrite the `Current` variant's doc comment to a short summary plus
      the exit-status fact (0 inside a managed slot, 1 otherwise), dropping
      elaboration.
- [ ] 3.3 Rewrite the `Help` variant's doc comment to a one-line summary.
- [ ] 3.4 Rewrite the `Lock` variant's and `reason`/`path` field doc comments to
      state what each does and its default, dropping mechanism detail about how
      git performs the lock.
- [ ] 3.5 Rewrite the `Unlock` variant's and `path` field doc comment similarly.
- [ ] 3.6 Rewrite the `Info` variant's doc comment to a short summary of what
      fields it prints and that it performs no filesystem writes, dropping the
      "useful as a first debugging step" framing.

## 4. Verification

- [ ] 4.1 Run `cargo build` to confirm the crate still compiles after the doc
      comment edits.
- [ ] 4.2 Manually run `bs --help` and `bs <command> --help` for every changed
      command (`get`, `list`, `current`, `help`, `lock`, `unlock`, `info`) and
      confirm: each summary is one sentence, all previously documented
      defaults/accepted-values/constraints are still present, and no
      rationale/internal-mechanics prose remains.
- [ ] 4.3 Run `cargo test` to confirm existing tests in `main.rs` are unaffected
      by the doc comment changes.
