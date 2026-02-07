# Tasks: JSON Log Colorizer CLI

**Input**: Design documents from `/specs/001-log-colorizer/`
**Prerequisites**: plan.md, spec.md, data-model.md, contracts/cli.md, research.md, quickstart.md

**Tests**: Included — constitution principle II ("Testing Standards NON-NEGOTIABLE") mandates >80% coverage and TDD workflow.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/`, `benches/` at repository root

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, dependency management, tooling configuration

- [X] T001 Initialize Rust project with `cargo init`, add all dependencies (serde, serde_json, clap, owo-colors, jiff, toml, thiserror) and optional simd-json feature to Cargo.toml
- [X] T002 [P] Create project directory structure: tests/integration/, tests/fixtures/, benches/
- [X] T003 [P] Configure rustfmt (rustfmt.toml) and clippy (clippy.toml or .cargo/config.toml) with strict settings per constitution
- [X] T004 [P] Create CI pipeline (GitHub Actions workflow in .github/workflows/ci.yml): cargo fmt --check, cargo clippy -- -D warnings, cargo test, cargo bench -- per constitution principle I (static analysis enforced in CI)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types and utilities that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T005 [P] Implement error types with thiserror (ConfigError, IoError, ParseError variants) in src/error.rs
- [X] T006 [P] Implement Level enum (Trace/Debug/Info/Warn/Error/Fatal variants, Display as 5-char padded badge, Ord for >= filtering, color mapping via owo-colors, case-insensitive string parsing, numeric value parsing with nearest-match rounding, level alias table from research.md §2.3) in src/level.rs
- [X] T007 [P] Implement FieldAliases as static const arrays (timestamp, level, message, logger, caller, error alias lists from data-model.md) with first-match lookup function in src/fields.rs
- [X] T008 [P] Implement Timestamp struct (jiff::Timestamp value + original string, parse ISO 8601/RFC 3339, epoch seconds/ms/ns with heuristic from data-model.md, YYYY-MM-DD HH:MM:SS format, format as HH:MM:SS.mmm for display) in src/timestamp.rs

**Checkpoint**: Foundation ready — user story implementation can now begin

---

## Phase 3: User Story 1 — Pipe JSON Logs and See Colorized Output (Priority: P1) 🎯 MVP

**Goal**: Pipe JSON-structured log output from any major framework into `cor` and see colorized, human-readable output with auto-detected timestamp/level/message fields.

**Independent Test**: `echo '{"ts":"2026-01-15T10:30:00Z","level":"info","msg":"server started","port":8080}' | cor` → colorized output `10:30:00.000 INFO  server started port=8080`

### Fixtures for User Story 1

- [X] T009 [P] [US1] Create sample log fixture files (5-10 lines each, covering field variations) in tests/fixtures/logrus.jsonl, tests/fixtures/zap.jsonl, tests/fixtures/slog.jsonl, tests/fixtures/pino.jsonl, tests/fixtures/bunyan.jsonl, tests/fixtures/structlog.jsonl, and tests/fixtures/embedded.jsonl

### Tests for User Story 1 (write FIRST, verify they FAIL)

> **NOTE: Write these tests FIRST per constitution principle II (TDD). They MUST fail before implementation begins.**

- [X] T010 [P] [US1] Write integration tests for basic piping: verify output format for each of 6 frameworks (auto-detect timestamp/level/message), verify extra fields sorted alphabetically, verify dot-notation flattening of nested objects, verify truncation at 120 chars, verify empty stdin exits 0, verify broken pipe exits 0 silently (pipe to `head -1`), verify extremely long lines (>1MB) are processed without crash and values truncated in tests/integration/basic_pipe.rs
- [X] T011 [P] [US1] Write integration tests for mixed input: JSON lines interspersed with plain text, malformed JSON passthrough, JSON arrays passthrough as raw text, lines with no recognized fields rendered as key=value in tests/integration/mixed_input.rs
- [X] T012 [P] [US1] Write integration tests for color control: NO_COLOR disables colors, FORCE_COLOR enables colors, --color=never disables, --color=always overrides NO_COLOR, TERM=dumb disables in auto mode, piped stdout disables colors by default in tests/integration/color_control.rs
- [X] T013 [P] [US1] Write integration tests for embedded JSON: line with prefix text + JSON object colorized correctly, prefix preserved in output, invalid JSON after prefix treated as Raw, multiple '{' characters handled correctly in tests/integration/embedded_json.rs

### Implementation for User Story 1

- [X] T014 [US1] Implement minimal CLI definitions with clap derive (--color auto|always|never with -c short, --help, --version) and ColorMode enum in src/cli.rs
- [X] T015 [P] [US1] Implement LogLine parser: parse_line() returning LineKind (Json/EmbeddedJson/Raw), JSON field extraction using FieldAliases for timestamp/level/message, embedded JSON detection via first '{' scan per FR-014, JSON array passthrough as Raw, construct LogRecord with extra fields as BTreeMap in src/parser.rs
- [X] T016 [P] [US1] Implement colorized output formatter: format LogRecord to "HH:MM:SS.mmm LEVEL message key=value..." output, level badge coloring via owo-colors, 5-char blank badge for None level, dim timestamp, bold message, dim extra fields sorted alphabetically, dot-notation flattening of nested objects (1 level), truncation at max_field_length with '…' suffix, unquoted string values, compact JSON for arrays and deep nesting, embedded prefix in dim text before formatted record in src/formatter.rs
- [X] T017 [US1] Implement main I/O loop: BufReader<Stdin> (8KB), stdout().lock() wrapped in BufWriter, line-by-line processing with reused String buffer, ColorMode detection (TTY via IsTerminal, NO_COLOR, FORCE_COLOR, TERM=dumb per precedence in contracts/cli.md), broken pipe handling (suppress BrokenPipe, exit 0), non-JSON passthrough unchanged, exit codes 0/1/2 in src/main.rs

**Checkpoint**: User Story 1 fully functional — `cor` reads stdin, auto-detects log fields, outputs colorized text, handles non-JSON and embedded JSON lines

---

## Phase 4: User Story 2 — Filter Logs by Level (Priority: P2)

**Goal**: Filter log output by minimum severity level to focus on warnings and errors during debugging.

**Independent Test**: `cat mixed-logs.jsonl | cor --level warn` → only WARN, ERROR, and FATAL lines shown; non-JSON lines always shown

### Tests for User Story 2 (write FIRST, verify they FAIL)

- [X] T018 [P] [US2] Write integration tests for level filtering: --level warn shows only warn+error+fatal, --level trace shows all, numeric level values (bunyan 30=info, 40=warn) filtered correctly, non-JSON lines always pass through during filtering per FR-015, no --level flag shows all levels in tests/integration/level_filter.rs

### Implementation for User Story 2

- [X] T019 [US2] Add --level flag (-l short, accepts trace|debug|info|warn|error|fatal, case-insensitive) to CLI definitions in src/cli.rs
- [X] T020 [US2] Implement level filtering in main loop: compare LogRecord.level against min_level using Level::Ord, suppress lines below threshold, pass through non-JSON lines unchanged per FR-015, handle numeric levels (bunyan/pino) via Level numeric parsing from T006 in src/main.rs

**Checkpoint**: User Stories 1 AND 2 both work independently — piping and level filtering functional

---

## Phase 5: User Story 3 — Customize Field Display (Priority: P3)

**Goal**: Control which fields are shown, override field key mappings, and persist preferences via config file.

**Independent Test**: `echo '{"custom_msg":"hello","sev":"INFO"}' | cor --message-key custom_msg --level-key sev` → output `INFO  hello`

### Tests for User Story 3 (write FIRST, verify they FAIL)

- [X] T021 [P] [US3] Write unit tests for Config: TOML parsing, merge precedence, XDG path resolution, default values, custom level_aliases integration, and integration test for custom key overrides and field filtering in src/config.rs and tests/integration/basic_pipe.rs

### Implementation for User Story 3

- [X] T022 [P] [US3] Implement Config struct (all 11 fields from data-model.md) and TOML config file loading: XDG_CONFIG_HOME path resolution, file read with graceful missing-file handling, serde deserialization, merge precedence (CLI > config > defaults) in src/config.rs
- [X] T023 [P] [US3] Add remaining CLI flags to clap definitions: --message-key (-m), --level-key, --timestamp-key (-t), --include-fields (-i, comma-separated), --exclude-fields (-e, comma-separated), --json (-j), --max-field-length (-M, integer, default 120), --config (path), add mutual exclusivity validation for --include-fields vs --exclude-fields (exit code 1 with error) in src/cli.rs
- [X] T024 [US3] Integrate Config with parser: accept optional override keys for message/level/timestamp, use overrides instead of FieldAliases when provided, wire Config.level_aliases into Level parsing to extend built-in mappings, update parse_line() signature to accept config reference in src/parser.rs
- [X] T025 [US3] Implement field inclusion/exclusion filtering in formatter (filter extra fields by include/exclude lists), --json output mode (passthrough original JSON with level filtering applied; non-JSON lines suppressed in --json mode), configurable max_field_length from Config (replacing hardcoded 120), wire Config through main loop in src/formatter.rs and src/main.rs

**Checkpoint**: All user stories independently functional — full CLI with customization, filtering, and config file support

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Performance validation, documentation, final quality pass

- [X] T026 [P] Write throughput benchmark targeting ≥100k lines/sec with typical 200-500 byte JSON lines using criterion in benches/throughput.rs. Include startup latency measurement (<100ms to first output per SC-003) and peak RSS assertion (<50MB per SC-002)
- [X] T027 [P] Add rustdoc documentation to all public types and functions across src/*.rs
- [X] T028 Run quickstart.md validation: cargo build --release, cargo install --path ., execute all examples from quickstart.md, verify output matches expected
- [X] T029 Final quality pass: cargo clippy -- -D warnings, cargo fmt --check, cargo test, verify all 5 integration test files pass, confirm exit codes 0/1/2 work correctly

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion — BLOCKS all user stories
- **User Stories (Phase 3–5)**: All depend on Foundational phase completion
  - User stories proceed sequentially in priority order: P1 → P2 → P3
  - US2 extends src/cli.rs and src/main.rs from US1
  - US3 adds src/config.rs and extends src/cli.rs, src/parser.rs, src/formatter.rs, src/main.rs from US1+US2
- **Polish (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) — no dependencies on other stories
- **User Story 2 (P2)**: Depends on US1 completion (extends cli.rs and main.rs)
- **User Story 3 (P3)**: Depends on US1+US2 completion (extends multiple files)

### Within Each User Story

- Fixtures (T009) can be created in parallel with tests and implementation
- Tests MUST be written and FAIL before implementation begins (TDD per constitution)
- Parser (T015) and Formatter (T016) can be developed in parallel (different files)
- Main loop (T017) depends on CLI (T014), Parser (T015), and Formatter (T016)
- All integration test files for a story can run in parallel

---

## Parallel Opportunities

### Phase 2: All foundational tasks in parallel

```text
T004 (error.rs) ──┐
T005 (level.rs) ──┼──▶ Phase 2 complete
T006 (fields.rs) ─┤
T007 (timestamp.rs)┘
```

### User Story 1: Parser and Formatter in parallel

```text
T008 (fixtures) ────────────────────────┐
T009 (cli.rs) ──────────────────┐       │
T010 (parser.rs) ──┐            ├──▶ T012 (main.rs) ──▶ T013-T016 (tests, all parallel)
T011 (formatter.rs)┘            │
```

### User Story 3: Config and CLI in parallel

```text
T020 (config.rs) ──┐
T021 (cli.rs) ─────┼──▶ T022 (parser update) ──▶ T023 (formatter+main update) ──▶ T024 (tests)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001–T004)
2. Complete Phase 2: Foundational (T005–T008)
3. Complete Phase 3: User Story 1 (T009–T017)
4. **STOP and VALIDATE**: `echo '{"level":"info","msg":"hello"}' | cargo run` shows colorized output
5. Binary is usable for basic log colorization

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → **MVP!** (basic pipe + colorize)
3. Add User Story 2 → Test independently → Level filtering available
4. Add User Story 3 → Test independently → Full customization
5. Polish → Benchmarks, docs, final validation

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks in same phase
- [US*] label maps task to specific user story for traceability
- Each user story extends prior stories (sequential dependency US1→US2→US3)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Total tasks: 29 (4 setup + 4 foundational + 9 US1 + 3 US2 + 5 US3 + 4 polish)
