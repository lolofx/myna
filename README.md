# Mynota

Record a meeting, a web conference or a video call on your PC, get a transcript on your machine, and export it as a Markdown file.

[![License: MIT](https://img.shields.io/badge/license-MIT-FFC300?style=flat-square)](LICENSE)

> **This is a fork.** Mynota is a small fork of [Myna](https://github.com/fmflurry/myna) by fmflurry (MIT). Myna is the real project, and it does much more. Mynota only keeps the part I need, and it has its own name and visuals so the two are not confused. It is not endorsed by Myna's author. Bugs in Mynota go in this repository's issues, not upstream's.

## What it is for

Mynota records what you hear and say, transcribes it locally, and writes a Markdown file. The summary is then up to you: give the file to another model, use a small local one, or (planned) use an LLM server on your own network.

It is deliberately limited. Beyond the Windows work and a few near-term changes, it will not grow into a full product.

## How it works

1. **Record** the microphone and the system audio as two tracks, `Me` and `Others`. Nothing is transcribed during the recording, so the meeting software keeps its CPU and memory.
2. **Transcribe** after you stop, at low priority, with a local speech model (Parakeet-TDT, French and English among 25 European languages). Optional speaker detection splits `Others` into numbered speakers you can rename.
3. **Export** one Markdown file per meeting into a folder you choose.
4. **Summarize** if you want to, with an optional local model chosen in Settings.

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

Details of the format: [ADR 0019](docs/adr/0019-markdown-export-contract-and-export-folder.md).

## Platforms

- **Windows**: the first target, in beta. This is what the first release will be.
- **macOS**: comes from upstream Myna. The macOS code is not modified on purpose, so it should keep working, but this fork does not test it.
- **Linux**: not started.

What has been checked on a Windows PC so far, with a development build: recording both tracks (including long silences and a change of output device), transcription in French and English, and the Markdown file. Not checked yet: the installer on other PCs, Bluetooth headsets, a real Teams call, speaker detection. The checklist is in [docs/windows-recipe.md](docs/windows-recipe.md).

## Install (beta)

1. Download `*-setup.exe` from [Releases](../../releases) once a beta is published.
2. The installer is not code-signed, so Windows SmartScreen warns: choose **More info**, then **Run anyway**. It installs for the current user, no administrator rights needed.
3. On first launch, click **Download models** (about 0.7 GB, once). A summary model is optional.
4. In **Settings > Markdown export**, pick the folder that receives your meetings.
5. Choose the capture source, **Record**, then **Stop**.

## Privacy

- Recording, transcription and the optional summary run on your machine. Nothing is sent anywhere by default.
- Meetings are stored in `%USERPROFILE%\mynota\`. No telemetry, no analytics.
- The network is used only for the one-time model download (Hugging Face and GitHub) and an opt-in update check that never installs anything without your click.

## Build from source

Needs Rust, Node 22, and on Windows the Visual Studio C++ build tools, CMake and LLVM. Details: [docs/windows-recipe.md](docs/windows-recipe.md).

```powershell
npm ci; npm --prefix ui ci
.\scripts\download-models.ps1
npx tauri dev
```

## License and credits

Mynota is MIT licensed, like Myna: see [LICENSE](LICENSE), which keeps the upstream copyright notice. Mynota is built on [fmflurry/myna](https://github.com/fmflurry/myna).

The Myna name and logo are upstream's and are not reused here.

Third-party components:

- Parakeet-TDT weights: CC-BY-4.0
- sherpa-onnx: Apache-2.0
- llama.cpp: MIT
- Qwen2.5-Instruct (optional summary model): [Qwen research agreement](https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF)
- Speaker detection models (pyannote segmentation 3.0, NeMo TitaNet-small): see each model card

More: [usage](docs/usage.md) · [architecture](docs/stack-proposal.md) · [decisions](docs/adr/) · [what comes next](aidd_docs/memory/backlog.md)
