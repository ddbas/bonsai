## ADDED Requirements

### Requirement: `bonsai.copy` git config lists files to carry into new slots

Bonsai SHALL read the list of files to copy into a provisioned pool slot from
the multi-valued git config key `bonsai.copy`, resolved via git's own
configuration system (equivalent to `git config --get-all bonsai.copy`), scoped
under the `bonsai` namespace. No bonsai-specific configuration file format SHALL
be introduced for this purpose.

#### Scenario: Single entry configured

- **WHEN** `bonsai.copy` is set to `.env` in the repository's git config
- **THEN** `bs get` SHALL treat `.env` as a file to copy into the provisioned
  slot

#### Scenario: Multiple entries configured

- **WHEN** `bonsai.copy` is set multiple times (e.g. `.env` and
  `config/local.json`) via repeated `git config --add bonsai.copy <value>` calls
- **THEN** `bs get` SHALL treat every configured value as a file to copy,
  preserving the order returned by `git config --get-all bonsai.copy`

#### Scenario: No entries configured

- **WHEN** `bonsai.copy` is not set anywhere in git config
- **THEN** `bs get` SHALL perform no file-copy step and behave exactly as before
  this capability was introduced

#### Scenario: Configured globally

- **WHEN** `bonsai.copy` is set in the user's global `~/.gitconfig` rather than
  a repository-local config
- **THEN** `bs get` SHALL still honor it for any repository, per git's normal
  config resolution precedence

### Requirement: Configured files are copied from the origin worktree into the provisioned slot

`bs get` SHALL copy each file listed in `bonsai.copy` from the origin worktree
(the working directory `bs get` was invoked from) into the corresponding
relative path inside the provisioned pool slot, after the slot has been created
or reset to the target HEAD/branch state.

#### Scenario: File exists in the origin worktree

- **WHEN** `bonsai.copy` lists `.env` and `.env` exists in the origin worktree
- **THEN** after `bs get` completes, the provisioned slot SHALL contain a copy
  of `.env` with the same contents as the origin worktree's `.env`

#### Scenario: Copy happens after slot provisioning

- **WHEN** `bs get` provisions a slot (new or reused) and `bonsai.copy` is
  non-empty
- **THEN** the slot SHALL first be fully reset/created (branch checkout
  included), and only then SHALL the configured files be copied into it

#### Scenario: Destination subdirectory does not yet exist

- **WHEN** `bonsai.copy` lists `config/local.json` and the slot has no `config/`
  directory yet
- **THEN** `bs get` SHALL create the necessary parent directories inside the
  slot before copying the file into place

### Requirement: Missing source files are skipped without error

`bs get` SHALL skip, silently and without error, any file listed in
`bonsai.copy` that does not exist in the origin worktree, and SHALL continue
processing the remaining entries.

#### Scenario: One of several entries is missing

- **WHEN** `bonsai.copy` lists `.env` and `.env.missing`, and only `.env` exists
  in the origin worktree
- **THEN** `bs get` SHALL copy `.env` into the slot, skip `.env.missing` without
  error, and exit successfully

#### Scenario: All entries are missing

- **WHEN** every file listed in `bonsai.copy` is absent from the origin worktree
- **THEN** `bs get` SHALL complete successfully with no files copied and no
  error reported

#### Scenario: Genuine copy failure still errors

- **WHEN** a listed file exists in the origin worktree but cannot be written to
  the destination in the slot (e.g. a permissions error)
- **THEN** `bs get` SHALL surface an error rather than silently skipping the
  entry
