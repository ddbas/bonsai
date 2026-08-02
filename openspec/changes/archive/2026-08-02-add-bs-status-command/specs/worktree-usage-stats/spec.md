## REMOVED Requirements

### Requirement: Usage stats column shows open-process count with gear icon

**Reason**: `bs list` no longer displays a usage-stats column (though it still
displays the status badge — see `specs/worktree-list/spec.md`). Open-process
detail — now itemized by PID and command name rather than a bare count — is
reported by `bs status` instead (see `specs/worktree-status/spec.md`).

**Migration**: Use `bs status <path>` to see the open processes for a slot;
`bs list`'s badge still indicates at a glance whether a slot is in use.

### Requirement: Usage stats column shows uncommitted-file count with plus-minus icon

**Reason**: `bs list` no longer displays a usage-stats column. Uncommitted-file
detail — now itemized as individual `git status --porcelain` lines rather than a
bare count — is reported by `bs status` instead.

**Migration**: Use `bs status <path>` to see the uncommitted files for a slot;
`bs list`'s badge still shows `in use` for a dirty slot.

### Requirement: Usage stats column shows untracked-file count with question-mark icon

**Reason**: `bs list` no longer displays a usage-stats column. Untracked-file
detail — now itemized as individual `git status --porcelain` lines rather than a
bare count — is reported by `bs status` instead.

**Migration**: Use `bs status <path>` to see the untracked files for a slot;
`bs list`'s badge still shows `in use` for a slot with untracked files.

### Requirement: Clean idle slot SHALL display an empty stats column

**Reason**: There is no longer a stats column in `bs list` output to be empty or
populated (the status badge remains and shows `available` for a clean idle
slot). `bs status` indicates a clean/idle slot by omitting or explicitly noting
empty process/uncommitted/untracked sections.

**Migration**: Use `bs status <path>`; a clean, idle, unlocked slot is reported
with an `available` classification and empty detail sections — matching
`bs list`'s `available` badge for the same slot.

### Requirement: Non-zero stat components are separated by a space

**Reason**: The compact, space-separated `⚙N ±N ?N` stats column no longer
exists in `bs list`. `bs status` presents each category (processes, uncommitted
files, untracked files) as its own itemized section rather than a single compact
line.

**Migration**: Use `bs status <path>` for per-category detail.
