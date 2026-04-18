---
description: Stage 4 — execute one phase with a fresh coder
argument-hint: <phase-number>
allowed-tools: Read, Write, Edit, Bash, Glob, Grep
---

Invoke the `coder` subagent to execute Phase $ARGUMENTS from `docs/plan.md`.

Before delegating:
1. Confirm `docs/plan.md` exists and has a Review Log entry. Otherwise recommend
   `/review-plan` first.
2. Confirm the specified phase's dependencies are all `Complete`. If not, list blockers.
3. Confirm the session is running from a Linux environment (WSL2 or native). If on
   Windows-native, stop and instruct the user to re-run from WSL2.

Pass to the coder ONLY:
- The phase number
- The path to `plan.md`

Do not summarize the plan for the coder — it reads the plan directly for a clean context.

When the coder returns, show me its status report. If it produced cross-phase Open
Questions, recommend `/review-plan` before continuing.
