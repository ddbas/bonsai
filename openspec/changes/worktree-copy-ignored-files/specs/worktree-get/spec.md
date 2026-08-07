## ADDED Requirements

### Requirement: Provisioning applies configured ignored-file copies as its final step

`bs get` SHALL copy any files listed in the `bonsai.copy` git config from the
origin worktree into the provisioned slot as its final provisioning step, after
the slot has reached its final state (branch checkout included, whether the slot
was newly created via `git worktree add` or reused via reset), and before
printing the slot path to stdout. This step applies uniformly to both the "reuse
an existing available slot" and "create a brand-new slot" code paths.

#### Scenario: Reused slot receives configured copies

- **WHEN** `bs get` reuses an existing available slot and `bonsai.copy` lists
  one or more files present in the origin worktree
- **THEN** those files SHALL be present in the reused slot once `bs get`
  finishes, in addition to the slot being reset to the resolved HEAD

#### Scenario: Newly created slot receives configured copies

- **WHEN** `bs get` creates a brand-new UUID-named slot via `git worktree add`
  and `bonsai.copy` lists one or more files present in the origin worktree
- **THEN** those files SHALL be present in the new slot once `bs get` finishes,
  in addition to the slot being created at the resolved HEAD

#### Scenario: No configuration means no behavior change

- **WHEN** `bonsai.copy` is unset
- **THEN** the provisioning flow SHALL behave exactly as it did before this
  capability was introduced, with no additional filesystem operations beyond
  slot creation/reset and branch checkout
