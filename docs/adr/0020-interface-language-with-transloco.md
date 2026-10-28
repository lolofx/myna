# ADR 0020: Interface Language with Transloco

**Status**: Proposed (Windows fork)
**Date**: 2026-10-28
**Builds on**: [ADR 0019: Markdown Export Contract and Export Folder](0019-markdown-export-contract-and-export-folder.md)
**Context**: The interface is English only. Every label lives in an Angular template or a component, and a few user-visible strings come from Rust (native menu labels, the default meeting title, error messages). The fork's user speaks French and wants the interface in English or French, switchable in Settings. The summary language is a separate, already working setting and stays separate.

## Decision

Use **Transloco** (`@jsverse/transloco`) for runtime translation of the Angular interface.

- **Languages**: `en` and `fr`, as one JSON file each under `ui/src/assets/i18n/`, keyed by feature (`settings.exportFolder.title`). English is the source of truth and the fallback.
- **Choice and storage**: a language selector in Settings. The choice is stored under an `interface` key of `preferences.json` (merged like the other keys). With no stored choice, the OS language is used when it is `fr`, otherwise English. Switching applies immediately, without a reload.
- **Loading**: the active language is read at startup before the first render, so the interface never flashes in the wrong language.
- **Placeholders**: named interpolation only (`{{ count }}`). Plurals are avoided by wording (`Meetings: 3`) until a feature truly needs them; if one does, add the ICU message format plugin then.
- **Parity check**: a spec fails when a key exists in one language file and not the other, and when a template uses a key that is missing from the English file.
- **Rust-side strings**:
  - Error messages stay English technical details. The interface translates the error **code** (`IO`, `STORE`, `BUSY`, `PATH`…) and shows the raw message as a detail line.
  - The native menu has few entries; it takes a small Rust catalogue for `en` and `fr`, built from the stored language at startup (a change applies after a restart, and the interface says so).
  - The default meeting title (`Meeting 28 Oct 20:41`) stays English; it is a stored value, not a label.
- **Not translated, on purpose**: the Markdown export (keys, `status` values, `## Summary:`), file names, the JSON export, summary templates and the summary language list. They are interfaces with other tools and must not change with the display language.
- **Rollout in two steps** so each commit stays reviewable: (1) the infrastructure, the Settings window and the language selector; (2) the remaining components, one feature area per commit.

## Rationale

### Why Transloco and not Angular's own `@angular/localize`?

`@angular/localize` resolves translations at build time and produces one bundle per language. A desktop app that lets the user switch language in Settings would need a restart into a different bundle, and the installer would carry both. Runtime translation avoids both.

### Why not ngx-translate?

It would do the job and supports this Angular version. Transloco is chosen for its typed, signal-friendly API, per-language lazy loading and the tooling that finds missing or unused keys. The two are close enough that this is a preference, not a constraint.

### Why not a small home-made service?

About a hundred lines would cover lookup and interpolation. Missing-key detection, lazy loading and test helpers would then have to be rebuilt and maintained in a fork that already carries more than it should.

### Why keep the export untouched?

The export is read by other programs. A French `statut` instead of `status` would silently break a downstream tool when the user changes the display language.

## Consequences

- One runtime dependency, `@jsverse/transloco` (peer range accepts Angular 16 and later; this project uses Angular 20).
- Every hard-coded label eventually moves to the JSON files. This is the large part of the work and it touches nearly every template, which are files upstream also edits: expect merge conflicts there. Doing it in feature-area commits keeps each conflict small.
- Tests that assert on English text keep working while English is the default; new tests should assert on keys or on the English fallback.
- Strings assembled in code (not in templates) need an explicit pass.

## Open Risks

- Upstream template changes after the migration will reintroduce hard-coded English; the parity spec catches missing keys but not new literal text. A lint rule against literal text in templates could be added later.
- French UI text needs a proofreading pass by a native speaker.
- The language list is two for now; adding a third language later costs one file plus the Rust catalogue entries.
