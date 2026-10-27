# ADR 0017: Record Then Transcribe on Windows, Markdown Export as the Interface

**Status**: Accepted. Capture-only recording and the post-stop transcription trigger are implemented; the lowered process priority and the markdown export are not yet.
**Date**: 2026-10-27
**Builds on**: [ADR 0006: System Audio Capture](0006-system-audio-capture.md), [ADR 0008: Dual-Track Audio with Speaker Attribution](0008-dual-track-audio-with-speaker-attribution.md), [ADR 0011: Disk-Backed Session State](0011-session-resilience.md), [ADR 0013: Summarization Model](0013-summarization-model-qwen25-7b.md)
**Context**: `docs/perf-baseline.md` measures live recording at about 250 to 270% CPU with 21 to 23 ORT threads and about 1 GB RSS on an Apple M5 Pro (15 cores). The Qwen2.5-7B summary adds a 4.7 GB model and several GB of RAM while loaded. On a typical Windows PC used for meetings at the same time, this competes with Teams or a browser.

## Decision

On Windows, the default pipeline becomes capture first, decode after.

- **During recording**: only the capture path runs. Mic and loopback tracks are written to WAV by the existing writers. The live decode worker is not started, so no VAD or model is loaded. Session manifest and crash recovery (ADR 0011) are unchanged.
- **After stop**: a transcription job runs on the saved tracks with a configurable thread count (default 2, allowed 1 to 4) and a lowered process or thread priority. Diarization follows, then the export.
- **Export contract**: a markdown file with YAML frontmatter (`title`, `date`, `duration`, `language`, `source`, `speakers`, `status: to-summarize`) and a body of `[hh:mm:ss] Speaker: text` lines, adjacent segments of one speaker merged. Written to a configurable folder. This file is the only interface with downstream tools.
- **Summary**: the in-app Qwen summary stays in the code but is off by default on Windows, so the 7B model is neither required nor loaded. Summaries are produced outside the app from the exported markdown.
- **UI**: kept. The window can be closed to the tray while capture continues.

Live mode stays available so upstream behavior is preserved and the change can be proposed upstream.

### How it works

- `app/src-tauri/src/transcription_prefs.rs` resolves the mode: `MYNA_TRANSCRIPTION=live|after`, then `preferences.json` (`{"transcription":{"mode":"after","threads":2}}`), then the platform default (`after` on Windows, `live` elsewhere).
- In `after` mode `start_recording` loads no speech model and builds no streamer, so the decode worker discards what it receives. Memory and CPU stay at the level of plain capture.
- `stop_recording` returns the meeting with `transcriptionPending` when audio tracks were captured. The facade then calls the existing re-transcribe path, so progress events, cancellation and errors behave as for "Re-transcribe".
- Speech-model threads: the configured `threads` (1 to 4), else 2 in `after` mode, else the previous CPU-count heuristic in `live` mode. The engine is cached, so a change applies from the next model load.

There is no settings screen for the mode yet.

## Rationale

### Why not tune live decoding instead?

Live captions need the model resident and decoding in real time, concurrent with the meeting software. Throttling it trades latency for CPU but keeps about 1 GB resident for the whole meeting. Decoding after the fact removes both costs from the meeting and still gets a full transcript, which is what the user needs.

### Why a markdown contract instead of an in-app summary?

Summaries are better produced by a stronger external model driven by separate tooling, where the prompts can live. Keeping prompts out of the app removes code and models from this repo and keeps the app free of API keys, consistent with the local-first promise.

### Why keep live mode in the code?

The fork should keep merging with upstream. A setting is a smaller divergence than deleting the live path.

## Consequences

- Transcript is available minutes after the meeting, not during it. Accepted.
- Low-priority decoding of a one-hour meeting may take longer than real time on weak CPUs. Measure on a real Windows PC before fixing the default thread count.
- The repo forbids `unsafe_code`. Lowering process priority needs either a safe crate or a call inside the crate that already owns Windows unsafe code (`myna-wasapi-probe`). To decide at implementation.

## Open Risks

- Windows capture has never been verified on real meetings (CHANGELOG). Phase 1 must validate it before anything is built on it.
- Language detection on mixed FR/EN segments may be unreliable; Whisper fallback is not planned.
