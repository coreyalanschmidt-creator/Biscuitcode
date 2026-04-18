---
description: Stage 1 — deep research on a topic
argument-hint: <topic or vision statement>
allowed-tools: Read, Write, Edit, Glob, Grep, WebSearch, WebFetch
---

Invoke the `researcher` subagent with this topic and any relevant project context:

$ARGUMENTS

The researcher produces `docs/research.md`. When it returns, show me its summary and any
assumptions it wants confirmed. Recommend `/plan` if ready.
