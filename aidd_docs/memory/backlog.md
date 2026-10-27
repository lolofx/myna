# Backlog (Windows lightweight fork)

## Supports

| Support | Authority for |
| --- | --- |
| GitHub Issues on this fork | defects and evolutions |
| `aidd_docs/tasks/` | PRD, plans, run summaries |
| `docs/adr/` | decisions (next free number: 0018) |

## Roadmap

| Phase | Outcome | Needs Windows PC |
| --- | --- | --- |
| 0 | Fork, AIDD docs, ADR 0017, first Windows `cargo check` and list of blockers | no |
| 1 | Windows build green in CI; WASAPI loopback + mic validated on real Teams/Slack/YouTube; device change mid-meeting | yes (recipe) |
| 2 | Capture-only recording mode; deferred transcription at low priority, 1 to 4 threads; peak RAM/CPU measured and written to `docs/perf-baseline.md` | yes (measure) |
| 3 | Markdown export contract (frontmatter + speaker lines), configurable output folder | partly |
| 4 | Tray icon, global hotkey, close-to-tray while capturing, installer | yes (recipe) |
| 5 | Optional: smaller local summary model, forced language, Whisper fallback | decide later |

## Rules

- One commit per concern, upstream-friendly: Windows fixes separate from fork-specific export work.
- No claim of Windows support in docs until Phase 1 recipe passes on a real PC.
- Deliver a CI-built installer plus a step-by-step test recipe; the dev machine is Linux and cannot run the app.
