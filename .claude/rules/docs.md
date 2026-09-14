---
paths:
  - "docs/**"
---

# Documentation rules

- Documents `00` to `09` describe the agreed design. The implementing model does not rewrite them. It appends to `docs/99-decisiones.md` and, when a decision changes a document, adds a dated note at the end of that document under "Registro de cambios" pointing to the `99` entry.
- Documents are in Spanish. Code identifiers, commands and API names stay verbatim in English.
- `docs/research/` is read-only reference material with source URLs. Do not edit.
- Every claim about an external API added to the docs must cite a URL, as the research reports do.
