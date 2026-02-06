<!--
  Sync Impact Report
  ==================
  Version change: N/A → 1.0.0 (initial ratification)
  Modified principles: N/A (initial creation)
  Added sections:
    - Core Principles (4 principles)
    - Development Workflow
    - Quality Gates
    - Governance
  Removed sections: N/A
  Templates requiring updates:
    - .specify/templates/plan-template.md ✅ no changes needed (dynamic references)
    - .specify/templates/spec-template.md ✅ no changes needed (compatible structure)
    - .specify/templates/tasks-template.md ✅ no changes needed (compatible structure)
    - .specify/templates/checklist-template.md ✅ no changes needed (generic)
    - .specify/templates/agent-file-template.md ✅ no changes needed (generic)
  Follow-up TODOs: None
-->

# jgb Constitution

## Core Principles

### I. Code Quality First

All code MUST adhere to consistent, enforceable quality standards.

- Every module, function, and class MUST have a single, clear
  responsibility. Functions exceeding 40 lines MUST be decomposed.
- All public APIs MUST include documentation (docstrings, JSDoc, or
  equivalent) describing purpose, parameters, return values, and
  error conditions.
- Static analysis (linting, type checking) MUST be configured and
  enforced in CI. Zero warnings policy: warnings are treated as
  errors.
- Code MUST pass automated formatting checks before merge. Manual
  formatting is not acceptable.
- Dead code, unused imports, and commented-out code MUST NOT be
  committed to the main branch.
- Naming conventions MUST be consistent across the entire codebase
  and follow language-idiomatic standards.

**Rationale**: Consistent code quality reduces cognitive load during
reviews, accelerates onboarding, and prevents technical debt
accumulation.

### II. Testing Standards (NON-NEGOTIABLE)

Every feature MUST be accompanied by tests that prove correctness.

- Test-Driven Development is the default workflow: write tests first,
  verify they fail, then implement until they pass (Red-Green-Refactor).
- Unit tests MUST cover all public functions and edge cases. Minimum
  coverage target: 80% line coverage for new code.
- Integration tests MUST be written for: cross-module interactions,
  API contract boundaries, data persistence operations, and external
  service integrations.
- Tests MUST be deterministic — no flaky tests allowed. Any test that
  fails intermittently MUST be fixed or quarantined immediately.
- Test names MUST clearly describe the scenario being verified using
  the pattern: `test_<unit>_<scenario>_<expected_outcome>`.
- Mocking MUST be limited to external dependencies. Internal module
  interactions SHOULD use real implementations where feasible.

**Rationale**: Tests are the project's living specification. Without
reliable tests, refactoring becomes dangerous and regressions
become inevitable.

### III. User Experience Consistency

All user-facing interfaces MUST deliver a predictable, coherent
experience.

- Error messages MUST be actionable: state what went wrong, why, and
  what the user can do to resolve it.
- All CLI tools MUST follow a consistent argument pattern and support
  `--help`, `--version`, and structured output (JSON) flags.
- UI components (if applicable) MUST follow a shared design system
  or component library. Ad-hoc styling is not permitted.
- Response formats MUST be stable across versions. Breaking changes
  to output schemas require a major version bump and migration guide.
- User-facing text MUST be reviewed for clarity, grammar, and
  tone consistency. Technical jargon MUST be avoided in end-user
  messages.
- Accessibility standards MUST be met: WCAG 2.1 AA for web
  interfaces, appropriate exit codes and stderr/stdout separation
  for CLI tools.

**Rationale**: Users form trust through consistency. Inconsistent
interfaces erode confidence and increase support burden.

### IV. Performance Requirements

All components MUST meet defined performance baselines and MUST NOT
regress without explicit justification.

- Performance budgets MUST be defined for critical paths before
  implementation begins. Budgets are documented in the feature plan.
- Response time targets: API endpoints MUST respond within 200ms at
  p95 under expected load. CLI commands MUST complete within 2s for
  typical inputs.
- Memory usage MUST be profiled for data-intensive operations.
  Unbounded memory growth is a blocking defect.
- Performance-critical code MUST include benchmarks that run in CI.
  Regressions exceeding 10% MUST block the merge.
- Database queries MUST be reviewed for N+1 patterns, missing
  indexes, and unnecessary full-table scans before merge.
- Startup time MUST be tracked. Applications MUST be ready to serve
  within 5s of process launch under normal conditions.

**Rationale**: Performance is a feature. Degradation compounds over
time and is exponentially harder to fix retroactively.

## Development Workflow

All contributors MUST follow this workflow to ensure traceability
and quality.

- **Branch strategy**: One branch per feature or fix, created from
  the latest main branch. Branch names MUST follow the pattern
  `<issue-number>-<short-description>`.
- **Commit discipline**: Commits MUST be atomic and descriptive.
  Use conventional commit format: `type(scope): description`
  (e.g., `feat(auth): add JWT validation`).
- **Code review**: Every change MUST be reviewed by at least one
  other contributor before merge. Self-merges are not permitted
  for production code.
- **CI pipeline**: All tests, linting, type checks, and formatting
  MUST pass before a PR can be merged. No manual overrides of
  failing CI.
- **Documentation**: User-facing changes MUST include documentation
  updates in the same PR. Documentation debt is treated as a defect.

## Quality Gates

These gates MUST be satisfied at each development milestone.

- **Pre-implementation gate**: Feature plan reviewed and approved.
  Constitution Check passed. Performance budgets defined.
- **Pre-review gate**: All tests pass locally. No linting or type
  errors. Code formatted. Documentation updated.
- **Pre-merge gate**: CI green. Code review approved. No unresolved
  review comments. Integration tests pass.
- **Post-merge gate**: Deployment verification in staging. Smoke
  tests pass. Performance baselines met. No error rate increase.

## Governance

This constitution is the supreme authority for development practices
in the jgb project. All pull requests, code reviews, and design
decisions MUST comply with these principles.

- **Supremacy**: Where other guidelines conflict with this
  constitution, the constitution prevails.
- **Amendment process**: Proposed changes MUST be documented with
  rationale, reviewed by project maintainers, and include a
  migration plan for any affected code or workflows.
- **Versioning**: The constitution follows semantic versioning.
  MAJOR for principle removals or redefinitions, MINOR for new
  principles or material expansions, PATCH for clarifications.
- **Compliance review**: At least quarterly, the team MUST review
  adherence to these principles and document findings.
- **Complexity justification**: Any deviation from these principles
  MUST be documented in the Complexity Tracking section of the
  relevant feature plan with explicit rationale.

**Version**: 1.0.0 | **Ratified**: 2026-02-06 | **Last Amended**: 2026-02-06
