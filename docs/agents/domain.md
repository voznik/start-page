# Domain Context and ADR Layout

## Context Layout

This project uses a **single-context layout**.

- **Domain Glossary:** Single root `CONTEXT.md` (when present) defines ubiquitous language and domain entities for the project.
- **Multi-Context Mapping:** A `CONTEXT-MAP.md` is not needed since the repository is a unified single-binary dashboard application.

## ADR Lookup Rules

- Architecture Decision Records (ADRs) are located in `docs/adr/`.
- Architectural RFCs and historic architecture designs are located in `docs/rfc/` (notably `RFC-002-unified-ratatui-architecture.md`).
- When proposing or evaluating architectural changes, consult existing records in `docs/adr/` and `docs/rfc/` before drafting new decisions.
