# AGENTS.md - AI Tool Guidance

This file provides guidance for AI agents working on this project.

## ba - Task Tracking

**When to use:** Track work items, claim tasks for sessions, manage project backlog.

**Protocol:**
- `ba ready` - See available tasks
- `ba claim <id> --session $SESSION_ID` - Claim a task for your session
- `ba create "description" -t task` - Create new tasks
- `ba done <id>` - Mark tasks complete
- `ba pr <id>` - Move task to PR review state

## superego - Metacognitive Advisor

**When to use:** Get feedback on reasoning, approach, and alignment with user goals. Currently in **pull mode** - reviews happen on request or before commits/PRs.

**Protocol:**
- `/superego:review` - Request a review of current work
- Superego automatically reviews before commits and PRs
- When you receive SUPEREGO FEEDBACK, critically evaluate it:
  - If you agree, incorporate it into your approach
  - If you disagree on non-trivial feedback, escalate to the user

## wm - Working Memory

**When to use:** Capture learnings, patterns, and context that should persist across sessions.

**Protocol:**
- Working memory captures happen automatically during work
- Use `wm:review` agent to review current state
- Use `wm:compress` to synthesize accumulated knowledge
- Patterns and learnings are extracted to `.wm/state.md`

---

## Project-Specific Guidance

<!-- Add project-specific AI guidance below this line -->
