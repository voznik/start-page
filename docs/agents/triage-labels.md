# Triage Labels

This document maps canonical triage roles to concrete issue labels in the repository.

## Canonical Roles & Labels

| Canonical Role | Concrete Label | Description |
| -------------- | -------------- | ----------- |
| `needs-triage` | `needs-triage` | Newly opened issue awaiting classification, feasibility assessment, or scoping. |
| `needs-info` | `needs-info` | Blocked on missing reproduction steps, requirements clarification, or user feedback. |
| `ready-for-agent` | `ready-for-agent` | Fully scoped work item with unambiguous acceptance criteria and an implementation contract. |
| `ready-for-human` | `ready-for-human` | Requires human decision, architectural sign-off, credential access, or design review. |
| `wontfix` | `wontfix` | Decision made not to implement. Record durable reasoning in `.out-of-scope/` if applicable. |

## Role Lifecycle

```text
[New Issue]
     │
     ▼
needs-triage ──(insufficient details)──► needs-info
     │                                        │
     │                                        ▼
     ├────────────────────────────────► ready-for-agent
     │                                        │
     ├────────────────────────────────► ready-for-human
     │
     └──(rejected / out-of-scope)─────► wontfix
```
