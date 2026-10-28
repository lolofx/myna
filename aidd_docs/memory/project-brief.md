# Project Brief (fork: Windows, lightweight)

## What it is

- Fork of [fmflurry/myna](https://github.com/fmflurry/myna) (MIT), a local-first meeting recorder, transcriber and summarizer built on Tauri 2 + Rust + Angular.
- Goal of the fork: one engine for transcribing, summarizing and exporting a meeting, a video or a webinar, delivered on **Windows, macOS and Linux**. macOS is upstream's; Windows is the current work, a build that stays light on RAM and CPU for Teams, Slack and any online meeting or video (YouTube, webinars); Linux follows.
- Scope: upstream's near-term list (global search, more export formats, model picker, custom vocabulary) plus what the fork already added (Windows capture, Markdown export contract and folder, summary on the user's own server, interface language). Upstream's mid-term and long-term items are out of scope.
- The fork keeps macOS code and the Angular UI so upstream changes keep merging. Windows-specific work stays in small, separable commits so it can be proposed upstream.

## Why it exists

- Upstream is macOS-first. Windows is a beta flagged "unverified" in `CHANGELOG.md`.
- Upstream transcribes live (about 250% CPU, 21 to 23 ORT threads, about 1 GB RSS on an M5 Pro) and summarizes with Qwen2.5-7B (4.7 GB on disk). Too heavy for a PC that also runs the meeting software.

## Target workflow

1. During the meeting: capture only (mic + WASAPI loopback of the system output, two tracks). No live decoding.
2. After the meeting: transcribe offline at low priority with 2 to 4 threads, run diarization, write a markdown file.
3. Summary: not done in the app by default. The markdown is handed to an LLM by separate tooling outside this repo, which chooses the prompt, writes the report and files it.

## Domain language

| Term | Meaning |
| ---- | ------- |
| Loopback | WASAPI capture of the default render endpoint, i.e. whatever the user hears |
| Record-then-transcribe | Capture first, decode after the meeting, off the realtime path |
| Export contract | The markdown + YAML frontmatter file the app produces; the only interface with downstream tooling |
| Downstream tooling | Notes or LLM tooling that consumes the export; owns summary prompts and storage, outside this repo |

## Languages

- Meetings mostly in French; webinars sometimes in English.
- Parakeet-TDT v3 detects the language per segment. It has no language parameter; pinning would need another model family (see `CLAUDE.md`).
- Translating or summarizing a transcript in another language is the downstream tooling's job, not the app's.

## Non-goals

- No cloud calls from the app, no API key, no bot joining a meeting. The one exception is an opt-in summary server on the user's own private network.
- No Linux work before the Windows recipe has passed; Linux is a goal, not a non-goal.

## Open questions

- Default thread count, to be set from measurements on typical 16 to 32 GB PCs.
- Whisper as fallback if mixed FR/EN language detection proves unreliable.
- Default export folder.
