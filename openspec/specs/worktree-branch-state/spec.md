# worktree-branch-state Specification

## Purpose

Define how bonsai decides, from local git state alone, whether a clean pool
slot's branch carries unmerged work, so that slots are reusable only when
resetting them cannot lose work.

## Requirements

### Requirement: Branch state is computed from local git state only

The branch-state check SHALL NOT perform network access and SHALL NOT fetch. It
SHALL read only local refs, config and objects.

#### Scenario: Offline operation

- **WHEN** the machine has no network access
- **THEN** classification of every slot SHALL still complete without error

### Requirement: Default ref resolution

The default ref D SHALL be resolved once per invocation: `origin/HEAD` if set,
else `origin/main`, else `origin/master`; local `main` then `master` SHALL be
used only when the repository has no `origin` remote. If D cannot be resolved,
every otherwise-clean slot SHALL be classified `in use` with a reason stating
the default branch could not be determined.

#### Scenario: origin/HEAD set

- **WHEN** `refs/remotes/origin/HEAD` points at `origin/develop`
- **THEN** D SHALL be `origin/develop`

#### Scenario: origin/HEAD missing

- **WHEN** `origin/HEAD` is unset and `origin/main` exists
- **THEN** D SHALL be `origin/main`

#### Scenario: No default ref

- **WHEN** no candidate ref exists
- **THEN** clean unlocked slots SHALL be `in use` with an explanatory reason and
  the CLI SHALL NOT panic

### Requirement: Slot with no unmerged commits is available

A clean, unlocked slot whose HEAD is an ancestor of D SHALL be classified
`available`, regardless of any remote branch state.

#### Scenario: Fresh branch from D

- **WHEN** a slot's branch was created from D with no commits
- **THEN** the slot SHALL be `available`

#### Scenario: Detached at a merged commit

- **WHEN** a slot is detached at a commit reachable from D
- **THEN** the slot SHALL be `available`

#### Scenario: Fast-forward or merge-commit merged branch

- **WHEN** a slot's branch commits are all reachable from D
- **THEN** the slot SHALL be `available` even if the remote branch still exists

### Requirement: Rewritten merges are recognized only for pushed branches whose remote branch is gone

When HEAD is not an ancestor of D, the slot SHALL be `available` only if ALL
hold: (1) the branch has upstream config (`branch.<b>.remote` and `.merge`, with
remote not `.`); (2) the remote-tracking ref
`refs/remotes/origin/<merge-branch>` does not exist locally; and (3) the
branch's content is already on D, determined by `git cherry` showing no
unapplied commits (rebase merge) or by `git merge-tree --write-tree D HEAD`
succeeding with a result tree equal to D's tree (squash merge). Otherwise the
slot SHALL be `in use`.

#### Scenario: Squash-merged, remote deleted and pruned

- **WHEN** a pushed branch was squash-merged into D and its remote branch was
  deleted and pruned locally
- **THEN** the slot SHALL be `available`

#### Scenario: Rebase-merged, remote deleted and pruned

- **WHEN** a pushed branch was rebase-merged into D and its remote branch was
  deleted and pruned locally
- **THEN** the slot SHALL be `available`

#### Scenario: Merged but remote branch still present

- **WHEN** content is on D but `refs/remotes/origin/<branch>` still exists
- **THEN** the slot SHALL be `in use`

#### Scenario: Merged and remote deleted but not yet pruned

- **WHEN** the remote branch was deleted upstream but the local remote-tracking
  ref still exists
- **THEN** the slot SHALL be `in use` until a fetch with prune

#### Scenario: Commits never pushed

- **WHEN** a branch has commits not on D and no upstream config
- **THEN** the slot SHALL be `in use`

#### Scenario: Pushed, PR open

- **WHEN** a branch has commits not on D and its remote branch exists
- **THEN** the slot SHALL be `in use`

#### Scenario: Remote deleted without merging

- **WHEN** a branch has commits whose content is not on D and its remote branch
  is gone
- **THEN** the slot SHALL be `in use`

#### Scenario: Detached HEAD with own commits

- **WHEN** a slot is detached at a commit not reachable from D
- **THEN** the slot SHALL be `in use`

#### Scenario: Squash detection unavailable

- **WHEN** git does not support `merge-tree --write-tree`, or it reports a
  conflict or error
- **THEN** the content-on-D check SHALL be treated as not merged and the slot
  SHALL be `in use`

### Requirement: Branch state yields a human-readable reason

For every slot classified `in use` because of branch state, the system SHALL
provide a reason (e.g. number of commits ahead of D, no upstream configured,
remote branch still exists, default branch unknown).

#### Scenario: Unpushed commits

- **WHEN** a slot's branch has 3 commits not on D and no upstream config
- **THEN** the reason SHALL state 3 commits ahead of D and that no upstream is
  configured
