# PRD: Windows lightweight recording and markdown export

Status: draft, 2026-10-27. Decision record: [ADR 0017](../../../docs/adr/0017-record-then-transcribe-on-windows.md).

## Problem

A user wants to record Teams, Slack and other online meetings or videos on Windows without the app competing with the meeting software for RAM and CPU, and to get a clean transcript to summarize elsewhere.

## Users

A single user on a Windows PC. Meetings in French and English.

## Requirements

### Must

- R1. Record microphone and system output (WASAPI loopback) as separate tracks, labelled `Me` / `Others`.
- R2. While recording, no speech model is loaded and no decoding runs. Target on a Windows PC: under 5% CPU, under 150 MB RSS for the Rust process (webview excluded, to be confirmed by measurement).
- R3. After stopping, transcribe the saved tracks at low priority with 2 threads by default (1 to 4 configurable).
- R4. Diarize `Others` into numbered speakers that can be renamed in the UI.
- R5. Export one markdown file per meeting: YAML frontmatter (`title`, `date`, `duration`, `language`, `source`, `speakers`, `status`) and lines `[hh:mm:ss] Speaker: text`. Output folder configurable.
- R6. Recording survives webview reload or crash (existing ADR 0011 behavior).
- R7. Tray icon while recording; closing the window keeps capture running.

### Should

- R8. Global hotkey to start and stop.
- R9. Installer built by CI; unsigned build documented with the SmartScreen warning.
- R10. Peak RAM/CPU for record and transcribe recorded in `docs/perf-baseline.md` from a real Windows PC.

### Won't (this PRD)

- In-app summary by default, API calls, cloud sync, stealth features, Linux.

## Export example

```markdown
---
title: Point projet
date: 2026-10-27T14:30:00
duration: 00:42:10
language: fr
source: meeting
speakers: [Me, Others 1, Others 2]
status: to-summarize
---

[00:00:04] Me: Bonjour à tous.
[00:00:07] Others 1: Bonjour, on commence par le planning.
```

## Acceptance

- A real Teams call and a YouTube video each produce a markdown file with correct speaker split and language.
- Peak RAM/CPU during recording and during transcription noted with the machine specs.
- Recipe in `docs/` passes on a real Windows PC.

## Open questions

See `aidd_docs/memory/project-brief.md`.
