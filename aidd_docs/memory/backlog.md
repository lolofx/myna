# Backlog (Windows lightweight fork)

## Supports

| Support | Authority for |
| --- | --- |
| GitHub Issues on this fork | defects and evolutions |
| `aidd_docs/tasks/` | PRD, plans, run summaries, [resume-here.md](../tasks/2026_10/resume-here.md) |
| `docs/adr/` | decisions (next free number: 0020) |

## Roadmap

Status: **done**, **partly**, **todo**, **decide**. "Real PC" means the item needs a Windows PC with real audio, so CI cannot settle it.

| Phase | Outcome | Status |
| --- | --- | --- |
| 0 | Fork, AIDD docs, ADR 0017, first Windows `cargo check` and list of blockers | done |
| 1 | Windows build green locally and in CI; WASAPI loopback and mic validated on a real PC; device change mid-recording | **partly**: local build, lint and tests green; recipe T1 to T4 and T6 to T7 passed on a real PC. Still to do: trigger the Windows CI once, a real Teams or Slack call, speaker detection (T8), a packaged build (T6 figures) |
| 2 | Capture-only recording, deferred transcription with 1 to 4 threads, lower process priority, figures in `docs/perf-baseline.md` | **partly**: capture-only and deferred transcription done and measured (see `perf-baseline.md`); lowered priority not yet verified on a real PC |
| 3 | Markdown export contract (front matter and timestamped speaker lines) and a configurable export folder | **partly**: implemented, [ADR 0019](../../docs/adr/0019-markdown-export-contract-and-export-folder.md); recipe check T9 pending; `language` and `source` fields left out because the app does not store them |
| 3b | **Settings coverage**: every setting the fork promises is reachable from the Settings window | **partly**, see the table below |
| 4 | Tray icon, global hotkey, close-to-tray while capturing, installer | todo (real PC) |
| 5 | Optional summary on the user's own server (Ollama on the local network) | **partly**: engine written and tested in `crates/myna-llm/src/remote.rs`; not wired into the app yet; ADR 0020 to write |
| 6 | Interface language: English and French, switchable in Settings | todo; an ADR first, see resume-here |
| 7 | Update channel: the update check still points at the upstream project's releases | **decide**, see resume-here |
| 8 | Smaller local summary model evaluation, forced language, Whisper fallback | decide later |

### Settings coverage (phase 3b)

| Setting | Where it lives today | In the Settings window |
| --- | --- | --- |
| Update check consent | `preferences.json` | yes |
| Summary language | `preferences.json` | yes |
| Summary model (catalog of sizes) | `preferences.json` | yes |
| Summary guidelines | `preferences.json` | yes |
| Meetings storage location | pointer file | yes |
| **Markdown export folder** | `preferences.json` (`export`) | **yes** (phase 3) |
| **Transcription threads (1 to 4)** | `preferences.json` (`transcription.threads`) | **no**, hand-edited only |
| **Transcription mode (after or live)** | `preferences.json` and `MYNA_TRANSCRIPTION` | **no**, hand-edited or environment only |
| **Summary server (address, model)** | not stored yet | **no**, phase 5 |
| **Interface language** | not stored yet | **no**, phase 6 |

## Rules

- One commit per concern, upstream-friendly: Windows fixes separate from fork-specific work.
- No claim of Windows support in docs until the phase 1 recipe passes on a real PC.
- Deliver a CI-built installer plus a step-by-step test recipe.
- The repository is public: no personal data, network details, machine specs or private tooling in any file or commit. Performance figures stay generic.
- No signature or co-signature in commits or pull requests.
