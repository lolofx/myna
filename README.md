<picture>
  <source media="(prefers-color-scheme: dark)"
    srcset="myna-brand-kit/myna-logo-horizontal-dark.svg">
  <img alt="Myna"
    src="myna-brand-kit/myna-logo-horizontal.svg" width="320">
</picture>

# Myna for Windows (unofficial fork)

**A local engine that turns a meeting, a video or a webinar into a transcript, a summary and a Markdown file, on your own PC.**

[![License: MIT](https://img.shields.io/badge/license-MIT-FFC300?style=flat-square&logo=open-source-initiative)](LICENSE)
[![Platform: Windows beta](https://img.shields.io/badge/platform-Windows%20(beta)-0F1115?style=flat-square)](docs/windows-recipe.md)
[![Runs locally](https://img.shields.io/badge/runs-locally-6366F1?style=flat-square)](#privacy)

This is a fork of [Myna by fmflurry](https://github.com/fmflurry/myna), the local-first meeting recorder for macOS. It keeps the upstream engine and interface and reworks the parts that matter for a PC that is also running the meeting software. The goal is one app for **Windows, macOS and Linux**: macOS comes from upstream, Windows is what this fork is building now, and Linux follows. It is **not** the official project and is not endorsed by its author. Bugs in this fork belong in this repository's issues.

## Status: beta, partly verified

Windows support is **not claimed yet**, and neither is Linux (not started). macOS is upstream's, unchanged in intent, and is not re-tested by this fork yet. What has been checked on a real Windows PC so far, with a development build:

- Recording the microphone and the system output as two separate tracks, including a silent stretch of many minutes and a change of output device in the middle of a recording.
- Transcribing afterwards, in French and English, with the right language per segment.
- Memory and CPU while recording and while transcribing ([figures](docs/perf-baseline.md#windows-record-then-transcribe-fork)).
- Writing the Markdown file described below.

Not verified yet: the installer on other PCs, Bluetooth headsets, a real Teams or Slack call, speaker detection, a signed installer. The step-by-step checks are in [docs/windows-recipe.md](docs/windows-recipe.md); running them on your PC and reporting back is the most useful contribution.

## What it does

1. **Record** the microphone and the system audio (WASAPI loopback) as separate tracks, labelled `Me` and `Others`. Nothing is decoded while recording, so the meeting software keeps its CPU and memory.
2. **Transcribe** after you stop, at low priority, with a local speech model (Parakeet-TDT, 25 European languages, language detected per segment). Optional speaker detection splits `Others` into numbered speakers you can rename.
3. **Export** one Markdown file per meeting into a folder you choose, written automatically as soon as the transcript is ready and refreshed when speakers or summaries change.
4. **Summarize** with whatever you prefer: hand the Markdown file to your own tooling, use the optional local summary models, or (planned) point Myna at an LLM server on your own network.

### The Markdown file

```markdown
---
title: Point projet
date: 2026-10-27T14:30:00
duration: 00:42:10
speakers: [Me, Others 1, Others 2]
status: to-summarize
---

[00:00:04] Me: Bonjour à tous.
[00:00:07] Others 1: Bonjour, on commence par le planning.
```

The front matter only holds what the app really knows (no guessed language or source). Lines are in chronological order, and the file name is stable (`YYYY-MM-DD_hh-mm_title.md`), so a tool watching the folder sees updates, not copies. Details and reasons: [ADR 0019](docs/adr/0019-markdown-export-contract-and-export-folder.md).

## How this fork differs from upstream

| | Upstream Myna | This fork |
| --- | --- | --- |
| Platform | macOS | Windows (beta), macOS kept from upstream, Linux planned |
| Transcription | Live, while recording | After the recording stops on Windows (live mode still exists, not the default, not validated on Windows) |
| Summary model | One 7B model, required | Optional and selectable; the Markdown export is the main output |
| Export | Save dialog, bullet list | Export folder, automatic, front matter and timestamped speaker lines |
| System audio | Core Audio taps | WASAPI loopback |
| Interface language | English | English now, French planned ([ADR 0020](docs/adr/0020-interface-language-with-transloco.md)) |

Everything else (storage layout, templates, speaker detection, the meeting library) is upstream's and is kept in step with it where possible. See [the roadmap](aidd_docs/memory/backlog.md) and [the hand-over notes](aidd_docs/tasks/2026_10/resume-here.md).

## Privacy

- Recording, transcription and the optional local summary run on your machine. No audio and no transcript is sent anywhere by default.
- Your meetings live in `%USERPROFILE%\myna\` (changeable in Settings). The export folder is wherever you point it.
- No telemetry, no analytics, no bot joining your call.
- The network is used for: the one-time model download from Hugging Face (about 0.7 GB, plus an optional summary model); an **optional, opt-in** update check, which never installs anything without your click; and, only if you turn it on, a summary server you name yourself. A summary server must be on a private network (loopback, LAN or VPN address) and is reached over plain `http`; only the transcript text is sent, never audio.

## Install (beta)

1. Download the installer (`*-setup.exe`) from this repository's [Releases](../../releases).
2. The installer is **not code-signed yet**, so Windows SmartScreen will warn. Choose **More info**, then **Run anyway**. It installs for the current user and needs no administrator rights. The WebView2 runtime is downloaded if it is missing.
3. On first launch, click **Download** for the models (about 0.7 GB, once). A summary model is optional and chosen in Settings.
4. In **Settings > Markdown export**, choose the folder that should receive your meetings.
5. Choose the capture source, **Record**, then **Stop**. The transcript and the Markdown file appear when the transcription ends.

## Build from source

Prerequisites, build settings and the verification checks are in [docs/windows-recipe.md](docs/windows-recipe.md). In short: Rust (installed through `rustup`), Node 20, the Visual Studio C++ build tools with a Windows SDK, CMake and LLVM.

```powershell
npm ci; npm --prefix ui ci
.\scripts\download-models.ps1
npx tauri dev
```

## Roadmap

*Direction, not commitment. The fork deliberately adds little to upstream.*

**This fork**
- Windows beta first, then Linux, with macOS kept working from upstream.
- Interface language, English and French (Transloco).
- Summary on an LLM server of your own network, opt-in.
- Settings for the transcription threads and mode.
- Tray icon, global hotkey, close-to-tray while recording.
- Its own signed update channel and installer.

**Taken from upstream's near-term list** (adopted when upstream ships them, or built here when needed sooner)
- Global search across meetings.
- More export formats (TXT, SRT, VTT, PDF, Obsidian, Notion).
- Model picker (7B, 14B and larger).
- Custom vocabulary.

Everything else upstream plans (calendar, RAG chat, sync, team features…) is outside this fork's scope.

## Credits and licenses

Myna is © its upstream author and contributors, **MIT** licensed; this fork keeps the [LICENSE](LICENSE) and follows the upstream project at [fmflurry/myna](https://github.com/fmflurry/myna).

Third-party components:

- Parakeet-TDT weights: **CC-BY-4.0**
- sherpa-onnx runtime: **Apache-2.0**
- llama.cpp runtime: **MIT**
- Qwen2.5-Instruct: **Qwen research agreement** ([Hugging Face](https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF))
- Speaker detection models (pyannote segmentation 3.0, NeMo TitaNet-small): optional download; see each model card for its license terms

More: [usage guide](docs/usage.md) · [architecture](docs/stack-proposal.md) · [decisions](docs/adr/) · [custom summary instructions](docs/custom-summary-instructions.md) · [custom template prompts](docs/custom-template-prompts.md) · [speaker detection accuracy](docs/diarization-accuracy.md)
