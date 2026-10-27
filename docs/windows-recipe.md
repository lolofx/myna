# Windows recipe

Hands-on checks for the Windows build that CI cannot do: real audio devices, real meeting software, real memory and CPU figures. Run it on a Windows 10/11 PC with an audio output and a microphone. Results go back as a short report (template at the end).

Background: [ADR 0017](adr/0017-record-then-transcribe-on-windows.md), [ADR 0018](adr/0018-optional-selectable-summary-model.md) and the review in `aidd_docs/tasks/2026_10/review-windows-baseline.md`, whose items 3 and 4 are checks T3 and T4 below.

## 0. Setup

1. Install, once: Visual Studio Build Tools with the "Desktop development with C++" workload, Git, Node.js 20, and Rust through `rustup` (the toolchain version is pinned by `rust-toolchain.toml` and installs itself).
2. Clone and switch to the branch under test:

   ```powershell
   git clone https://github.com/<owner>/myna.git
   cd myna
   git checkout win/lightweight
   npm ci
   npm --prefix ui ci
   ```

3. Build settings that match CI and the release script (static C runtime, required for the link to succeed):

   ```powershell
   $env:RUSTFLAGS = "-C target-feature=+crt-static"
   $env:LLAMA_STATIC_CRT = "1"
   $env:CMAKE_MSVC_RUNTIME_LIBRARY = "MultiThreaded"
   ```

4. Models (about 0.7 GB; the summary model is optional and picked in the app):

   ```powershell
   .\scripts\download-models.ps1
   .\scripts\download-models.ps1 -Only diarization   # optional, speaker labels
   ```

5. Start the app from source, so debug logs print in the terminal: `npx tauri dev`. A packaged build comes from the `Windows CI` workflow or `scripts\release-windows.ps1`; use it for T6 (figures from a dev build overstate memory).

Record the machine in the report: CPU, RAM, Windows version, audio output and microphone models, headset type (wired, USB, Bluetooth).

## 1. Capture checks

Each check lists the steps, what to expect, and what to note.

### T1. System audio loopback works

1. `cargo run -p myna-wasapi-probe --locked` while a YouTube video plays.
2. Expect: a mix rate (usually 48000 Hz), channel count, non-zero RMS and peak, and a split of audible versus silent frames.

Note: the printed figures.

### T2. Mixed recording, two tracks

1. In the app choose capture source **Both** (microphone and system audio). Play a YouTube video in French and speak over it for 60 s. Stop.
2. Expect: `track-mic.wav` and `track-system.wav` in the meeting folder (`%USERPROFILE%\myna\meetings\<id>\`), both about 60 s, and a transcript with `Me` and `Others` labels.

Note: both durations, and whether the transcript text matches what was said.

### T3. Silence gap (review items 3 and 4)

Suspected problem: shared-mode loopback may send no data while nothing plays, which the stall detector can read as a dead capture and restart in a loop, and which can shift the two tracks against each other.

1. `cargo run -p myna-wasapi-probe --locked` with **nothing playing** for the whole run. Note whether any frames arrive (the audible/silent split).
2. In the app, record **Both**: speak 20 s of speech through the speakers, pause everything for 30 s, play 20 s again. Stop.
3. Expect: no repeated restart messages in the terminal, `track-mic.wav` and `track-system.wav` equal in duration within 1 s, and the second burst of system speech lined up with the same moment in the mic track (clap or count aloud to check).

Note: whether frames arrive in silence, any "stall" or "rebuild" log lines and how many, both track durations, any visible offset.

### T4. Output device change mid-recording (review item 2)

1. Start a **Both** recording with a video playing through the speakers.
2. After 20 s, switch the default output: plug in a headset, or change the default device in Sound settings. Keep playing for 20 s more. Stop.
3. Expect: system audio continues on the new device within about 5 s, with no gap longer than that.
4. If the new device has another sample rate (Settings, Sound, device properties), capture is expected to stop with a message about the sample rate change. Note the exact text.

Note: gap length in `track-system.wav`, behaviour for wired, USB and Bluetooth devices, any error text.

### T5. Bluetooth headset call quality

1. With a Bluetooth headset as output and microphone, start a **Both** recording.
2. Expect the headset to switch to its call profile once the microphone opens (quieter, lower quality output). Note whether capture survives the switch.

## 2. Transcription and resources

### T6. Memory and CPU

Use Task Manager (Details tab) on a packaged build when possible.

1. Idle app, window open: note working set of `myna.exe` and of the `msedgewebview2.exe` processes.
2. During a 30 min **Both** recording (a long video works): note CPU % and working set of `myna.exe`, with the window open and with it minimized.
3. After stopping, during transcription: note peak working set, CPU %, and wall time versus audio length.
4. Compare against the targets in `aidd_docs/tasks/2026_10/prd-windows-lightweight.md` (R2, R3).

Note: all figures, and which build (dev or packaged). Add them to `docs/perf-baseline.md` as a Windows section.

### T6b. Deferred transcription (capture only, then transcribe)

1. Check the mode is `after` (the Windows default). To force it: `$env:MYNA_TRANSCRIPTION = "after"` before starting the app.
2. Record a 10 minute **Both** meeting. While recording, Task Manager must show no speech-model memory: the `myna.exe` working set stays far below a live recording's.
3. Stop. Expect the import progress to appear at once and the transcript to land when it ends, without pressing anything.
4. Note: time from stop to transcript against the audio length, peak working set and CPU during that time, whether the PC stayed usable (browser, Teams).
5. Repeat with `$env:MYNA_TRANSCRIPTION = "live"` and compare memory and CPU during the recording.
6. Set `{"transcription":{"threads":4}}` under `preferences.json` in `%USERPROFILE%\myna` (or the configured data folder) and note the change in transcription time. The engine is cached, so restart the app between runs.

### T7. French and English

1. Record or import a French meeting of about 10 minutes and an English webinar.
2. Expect: correct language per segment, readable punctuation. Mixed French and English in one recording: note how often a segment comes out in the wrong language.

### T8. Speaker labels

With diarization models installed, run **Detect speakers** on T2 or T7. Note the number of speakers found against the real number.

## 3. Summary models

1. Settings, **Summary model**: pick Qwen2.5 3B, click **Download**, wait for the end. Expect "Installed and ready".
2. Generate a summary of the T7 French transcript. Repeat with 1.5B and 7B if disk allows.
3. Note for each size: time to first output, peak memory, and a one-line quality judgement (misses, invented items, language).
4. Restart the app: the choice must persist.

## 4. Automated checks that need real hardware

```powershell
cargo test --locked -p myna-audio --test system_sources_naming -- --test-threads=1 --nocapture
cargo test --locked -p myna-audio -- --ignored --test-threads=1
```

On the hosted CI runner the first one dies with `STATUS_ACCESS_VIOLATION` (no audio endpoint). Note whether it passes here. If it crashes here too, keep the full output.

## 5. Report template

Paste this back, filled in:

```markdown
## Windows recipe report
Date: 
Branch / commit: 
Machine: CPU, RAM, Windows version
Audio: output device, microphone, headset type
Build: dev / packaged

| Check | Result (pass / fail / partial) | Notes and figures |
| ----- | ------------------------------ | ----------------- |
| T1 loopback | | |
| T2 mixed, two tracks | | |
| T3 silence gap | | |
| T4 device change | | |
| T5 Bluetooth | | |
| T6 memory and CPU | | |
| T6b deferred transcription | | |
| T7 French / English | | |
| T8 speaker labels | | |
| Summary 1.5B / 3B / 7B | | |
| Hardware tests (section 4) | | |

Problems seen that no check covers: 
```

## Working rules for whoever runs this

- Commits and pull requests carry no co-author trailer. Commit with the GitHub noreply address, and keep personal details out of every file in this public repository.
- The heavy Windows CI jobs run on demand (`gh workflow run windows-ci.yml --ref win/lightweight`); group fixes before triggering one.
