# Mynota usage guide

Mynota records a meeting on your PC, transcribes it locally and writes a Markdown file. This guide covers the first run, recording, the export and where the data lives. Windows is the platform this fork works on; macOS behaviour is Myna's (see the notes at the end).

## First run

1. Install the app (see the [README](../README.md#install-beta)) or build it from source.
2. On first launch the onboarding screen lists the models. Click **Download models** and wait: it fetches the speech model (Parakeet-TDT, about 640 MB), the voice detector (Silero VAD) and the optional speaker detection models, once, from Hugging Face and GitHub. You can cancel and retry.
3. If the download fails, the error is shown under the button, and the script output is kept in `download-<name>.log` in the models folder (`%USERPROFILE%\mynota\models`).
4. A summary model is optional. Pick one later in **Settings > Summary model**.

From the command line: `.\scripts\download-models.ps1` (Windows) or `./scripts/download-models.sh` (macOS and Linux). Both are safe to re-run.

## Recording

Choose a capture source, click **Record**, then **Stop**.

- **Microphone**: your voice only.
- **System audio**: what the PC plays (the other people on a call, a video, a webinar). On Windows this uses WASAPI loopback and needs no permission.
- **Both**: two separate tracks, `Me` (microphone) and `Others` (system audio).

Use a headset when you record both. On speakers, the microphone also picks up the other people and they are transcribed twice.

On Windows nothing is transcribed while you record, so the meeting software keeps its CPU and memory. Transcription starts when you stop, at low priority, and the transcript appears a few minutes later. A meeting can be transcribed again from the library with **Re-transcribe**.

To change this, set `MYNA_TRANSCRIPTION=live` (or `after`) in the environment, or add this to `preferences.json` in the data folder:

```json
{ "transcription": { "mode": "after", "threads": 2 } }
```

`threads` (1 to 4, default 2 in `after` mode) is the number of CPU threads the speech model uses. Fewer is lighter on the PC and slower to finish. Live mode exists but is not the default and has not been validated on Windows.

## The Markdown export

In **Settings > Markdown export**, choose a folder. Each meeting then becomes one file there, written as soon as the transcript is ready and refreshed when speakers are renamed or a summary is added:

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

The file name is stable (`YYYY-MM-DD_hh-mm_title.md`), so a tool that watches the folder sees updates, not copies. The reasons for each choice are in [ADR 0019](adr/0019-markdown-export-contract-and-export-folder.md).

You can also export one meeting by hand from the library, as Markdown or JSON.

## Speakers

Speaker detection splits `Others` into numbered speakers (`Others 1`, `Others 2`…). Rename them in the transcript and the export follows. It is not perfect, especially with similar voices or overlapping speech; see [diarization-accuracy.md](diarization-accuracy.md).

## Summaries

Summaries are optional. You have three ways to get one:

- Give the Markdown file to another model or tool.
- Use a local model: in **Settings > Summary model**, pick a Qwen2.5-Instruct size and click **Download** once.

  | Model | Download | Notes |
  | ----- | -------- | ----- |
  | Qwen2.5 1.5B | about 1.1 GB | Lightest; quality on long French meetings is untested |
  | Qwen2.5 3B | about 2.1 GB | Balanced |
  | Qwen2.5 7B | about 4.7 GB | Best quality, most memory |

  Several can be installed side by side; with no explicit choice, the best installed one is used. In the library, select a meeting, click **Summarize** and choose a template. Without a model installed, the app points you to the settings.
- Use an LLM server on your own network. This is planned, not available yet.

Built-in templates: Key Points, Action Items, Meeting Notes, Decisions. To add your own, put a JSON file in `templates/` ([format](custom-template-prompts.md), [extra instructions](custom-summary-instructions.md)).

## Where the data lives

Everything is stored under one data root, `%USERPROFILE%\mynota\` by default:

```text
<data-root>/meetings/<id>/
├── meeting.json        # metadata and transcript
├── audio.wav           # recording, device-native stereo
├── track-mic.wav       # 16 kHz mono microphone track
├── track-system.wav    # 16 kHz mono system track
└── summaries/          # one Markdown file per template
```

The data root also holds `preferences.json` and `folders.json`. Model weights stay in `%USERPROFILE%\mynota\models` and never move with the meetings.

To move the meetings, open **Settings > Storage > Change**, choose a folder, choose **Move** (migrates existing meetings) or **Stay**, and restart when asked. **Reset** goes back to the default. Changing is disabled while recording or transcribing. If the chosen folder disappears, the app shows an error and a **Reset** option rather than silently using another place.

For development, `MYNA_DATA_DIR` overrides the data root and `MYNA_MODELS_DIR` overrides the models folder.

Disk use: one hour of recording is about 920 MB for `audio.wav` plus about 115 MB for each 16 kHz track. Delete meetings from the library to free space.

## Languages

Parakeet-TDT v3 covers 25 European languages, detected per segment: English, German, Dutch, Swedish, Norwegian, Danish, French, Spanish, Italian, Portuguese, Romanian, Polish, Czech, Slovak, Slovene, Croatian, Bulgarian, Hungarian, Finnish, Greek, Lithuanian, Latvian, Estonian, and others. Meetings in French and English are the ones this fork is tested with.

## Troubleshooting

- **Model download fails**: read the message under the button and the log file in the models folder. Check your connection and any proxy or antivirus, then retry.
- **No sound captured from the call**: check that the capture source includes system audio and that the call plays through the default output device. Changing the output device during a recording is supported but worth checking in [windows-recipe.md](windows-recipe.md).
- **Slow transcription**: the first run loads the model into memory. Lower the thread count to keep the PC responsive, or raise it to finish sooner.
- **Slow summary**: the first summary loads the model; later ones are faster.

## macOS

The macOS code is Myna's and this fork does not change it, but it does not test it either. The data folder is `~/mynota`, system audio capture needs macOS 14.4 or later and a permission (System Settings > Privacy & Security), and the app is not signed with an Apple Developer ID. For macOS, the original [Myna](https://github.com/fmflurry/myna) is the better reference.

## License

MIT, see [LICENSE](../LICENSE). Third-party components:

- Parakeet-TDT weights: CC-BY-4.0 (attribution required)
- sherpa-onnx: Apache-2.0
- llama.cpp: MIT
- Qwen2.5-Instruct: [Qwen research agreement](https://huggingface.co/Qwen/Qwen2.5-7B-Instruct-GGUF)
