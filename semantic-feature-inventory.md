# Semantic Feature Inventory

## Goal

Build a complete semantic inventory before designing the language.

Do not design syntax or implementation during this phase.

The inventory is the only source of truth for future language design.

---

## References

Primary

- Microsoft Office
- OOXML
- LaTeX

Secondary

- Typst
- OpenDocument
- HTML

---

## Workflow

For each feature:

1. Identify its semantic purpose.
2. Compare Office and LaTeX.
3. Review existing solutions if needed.
4. Classify the feature.
5. Make a design decision.

Repeat until all categories are complete.

---

## Categories

- Document Structure
- Text
- Layout
- Style
- Tables
- Figures
- Charts
- References
- Metadata
- Collaboration
- Presentation

---

## Feature Record

Every feature must contain:

- Purpose
- Office
- LaTeX
- Existing Solution
- Classification
- Decision
- Notes

---

## Classification

Choose one:

- Semantic
- Presentation
- Implementation

Only Semantic features are candidates for the language.

---

## Decision

Choose one:

- Core
- Optional
- Backend Only
- Reject

---

## Planning Rules

Planning documents may freely evolve.

Improve structure whenever a simpler or more consistent design is found.

Do not preserve outdated decisions.

---

## Execution Rules

Once implementation begins:

- Treat planning documents as stable specifications.
- Do not change the language model while implementing.
- Record proposed changes instead of applying them.
- Return to the planning phase before modifying the specification.

---

## Constraints

- Keep the language minimal.
- Prefer semantics over syntax.
- Prefer existing concepts over new ones.
- Ignore presentation unless it changes document meaning.
- Avoid backend-specific features.
- Avoid speculative design.
- Optimise for rapid implementation.

---

## Deliverable

The completed inventory will define the Semantic Model.

The Semantic Model will become the foundation of the IR and the DSL.

## Target Formats

### Tier 1

Fully supported.

- DOCX
- PPTX

### Tier 2

Best-effort support.

- PDF
- HTML

### Tier 3

Future consideration.

- ODT
- Markdown

Spreadsheet formats are out of scope.

## Design Constraints

The semantic model should remain independent of any specific output format.

A feature should only be included if it can be represented consistently across at least one primary target.

Backend-specific behaviour should not influence language design.

If a feature exists only in a single format, record it in the inventory but do not introduce language support unless it provides clear semantic value.

The first implementation should prioritise DOCX and PPTX.
