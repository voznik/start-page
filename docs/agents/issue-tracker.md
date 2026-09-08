# Issue Tracker

This project uses **GitHub Issues** as its primary issue tracker.

## Tooling

- Tool: GitHub CLI (`gh`)
- Repository: `voznik/start-page`

Verify connectivity and authentication with:
```bash
gh auth status
```

## Allowed Operations for Agents

| Operation | Permitted | Notes / CLI Command |
| --------- | --------- | ------------------- |
| Read issue list / details | Yes | `gh issue list`, `gh issue view <number>` |
| Search issues | Yes | `gh issue list --search "<query>"` |
| Add comments | Yes | Post progress or brief handoffs: `gh issue comment <number> --body "..."` |
| Apply / remove triage labels | Yes | When triaging or claiming: `gh issue edit <number> --add-label "..." --remove-label "..."` |
| Create new issues | Yes | Only when instructed or reporting distinct discovered bugs |
| Close issues | Restricted | Only close an issue if explicit verification passes and PR is merged / work item accepted |

## Label Mutation Policy

- When starting an issue with `ready-for-agent`, do not remove the label until implementation and verification are complete.
- Transition to `ready-for-human` if agent work is blocked, requires review, or acceptance testing is ready.
- If more details are required from the requester, assign `needs-info`.

## Local Work Items & Historical Tracking

For work completed locally or prior to GitHub Issues adoption (including the foundational 23 commits across Phases 0–2), tasks are tracked in:

- [`docs/agents/work-items.md`](file:///home/voznik/workspace/rust/start-page/docs/agents/work-items.md) — Chronological work items, commit mappings, acceptance criteria, and status.
- [`docs/plans/phase-0-2-work-items.md`](file:///home/voznik/workspace/rust/start-page/docs/plans/phase-0-2-work-items.md) — Original phased specification and decision gate definitions.

When working offline or on work items with structured IDs (e.g. `T0.1`–`T2.4`), update `docs/agents/work-items.md` directly.

