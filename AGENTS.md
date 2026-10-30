# Bonsai

`bs` is a CLI that provisions clean git worktrees from a managed pool for fast
context-switching. It is a TypeScript pnpm + Turborepo monorepo with a hexagonal
layout: `packages/core` holds the domain and ports, the other packages are
adapters, and `apps/bonsai` is the thin CLI.

## Rules

1. You MUST never modify AGENTS.md files without explicit permission from the
   user.
1. You MUST never skip git hooks without explicit permission from the user. If a
   hook fails, fix the issue.
1. You MUST always use Mise for running all tasks and dev tools.
1. You MUST always use OpenSpec for formalizing all requirements.
1. You MUST always separate functional (and non-functional) requirements from
   architectural/technical requirements. They should NEVER be in the same spec.
1. You MUST always commit and push your changes once you've completed a task.
1. You MUST never create, modify or delete documentation without explicit
   permission from the user (including README.md and CONTRIBUTING.md).
