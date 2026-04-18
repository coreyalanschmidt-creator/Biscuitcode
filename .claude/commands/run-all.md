---
description: Full pipeline with interactive checkpoints
argument-hint: <topic or vision statement>
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, WebSearch, WebFetch
---

Run the full C.Alan pipeline on: $ARGUMENTS

Proceed in this order, pausing for my confirmation between stages:

1. `researcher` with the topic. Show summary + assumptions. **Pause.**
2. `planner`. Show summary. **Pause.**
3. `reviewer`. Show findings. **Pause.** If Blocked, stop.
4. For each phase N in order:
   - Fresh `coder` for Phase N.
   - Show status report.
   - If `Complete` with no cross-phase Open Questions, continue to N+1.
   - If `Partial`, `Blocked`, or cross-phase concerns arose, **pause** and ask whether to
     re-run the reviewer or stop.

Never run two phases in the same coder invocation — each phase gets its own fresh subagent.
