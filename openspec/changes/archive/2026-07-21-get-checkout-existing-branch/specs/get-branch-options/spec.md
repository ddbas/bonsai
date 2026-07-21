## MODIFIED Requirements

### Requirement: `-b` and `-B` are mutually exclusive with the positional branch argument

The `-b` flag, the `-B` flag, and the positional `<branch>` argument SHALL be
pairwise mutually exclusive. Supplying any two of these on the same invocation
SHALL result in a non-zero exit and a usage error.

#### Scenario: Both flags supplied

- **WHEN** the user runs `bs get -b foo -B bar`
- **THEN** the CLI SHALL exit with a non-zero status
- **THEN** stderr SHALL describe the mutual-exclusion constraint

#### Scenario: `-b` supplied with a positional branch argument

- **WHEN** the user runs `bs get -b foo existing-branch`
- **THEN** the CLI SHALL exit with a non-zero status
- **THEN** stderr SHALL describe the mutual-exclusion constraint

#### Scenario: `-B` supplied with a positional branch argument

- **WHEN** the user runs `bs get -B foo existing-branch`
- **THEN** the CLI SHALL exit with a non-zero status
- **THEN** stderr SHALL describe the mutual-exclusion constraint
