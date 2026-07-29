## Project

This project implements an AI-first semantic document language and compiler.

The language describes document semantics, not presentation.

Backends generate platform-specific output.

Current implementation language: Rust.

## Principles

- Semantics over appearance.
- Simplicity over features.
- Determinism over heuristics.
- Readability over cleverness.
- Backend independence whenever possible.
- The development is based on two workflows: Planning and Design.
- During planning, optimise the design. During implementation, protect the design.

## Rules

Before changing behaviour, update the corresponding documentation.

Do not introduce new syntax without strong semantic justification.

Prefer extending existing constructs over creating new ones.

Keep parsing deterministic.

Avoid backend-specific language features.

## Workflow

Read documentation before implementation.

Implement the smallest correct solution.

Explain design trade-offs before major architectural changes.

Keep code modular and explicit.

## Decision Priority

When multiple solutions satisfy the specification, choose the one that is:

1. Simpler
2. More deterministic
3. Easier to maintain
4. Easier to extend

Never choose additional flexibility unless it is required by the specification.
