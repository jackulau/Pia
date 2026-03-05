---
id: ux-safety
name: Add UX safety guardrails for dangerous actions
wave: 1
priority: 3
dependencies: []
estimated_hours: 3
tags: [frontend, backend, ux]
---

## Objective

Add safety guardrails to the agent UX — confirmation prompts for destructive actions, visual warnings, and emergency stop improvements.

## Context

The agent can perform destructive actions (deleting files, closing windows, etc.) without user confirmation. This task adds safety mechanisms.

## Acceptance Criteria

- [ ] Dangerous actions trigger a confirmation prompt before execution
- [ ] Visual warning indicators for high-risk actions
- [ ] Kill switch remains responsive during all operations
