# Review: Windows baseline (upstream 0.6.2)

Read-only review of the existing Windows code before building on it. Scope: `crates/myna-wasapi-probe`, `crates/myna-audio/src/system_windows.rs`, Windows `cfg` sites in `app/src-tauri`, `scripts/download-models.ps1`, release workflow. Status of each item is **read from code, not yet run on Windows**; hardware items need the recipe.

CI evidence (run on `windows-latest`): fmt, `cargo check` and all `myna-audio` tests pass for `myna-wasapi-probe`, `myna-audio`, `myna-stt`, `myna-llm` after fixing one stale test (`c32edec`). The `myna-app` (Tauri) crate has never been compiled by any CI job on a branch.

## Findings

| # | Severity | Where | Finding | Proposed fix |
| - | -------- | ----- | ------- | ------------ |
| 1 | High (UB) | `myna-wasapi-probe/src/lib.rs:259` vs `:270` | `CoTaskMemFree(mix_format)` runs before `client.Initialize(..., mix_format, ...)`. The format pointer is used after being freed. Works by luck while the freed block is intact. | Free after `Initialize` (small RAII guard so every error path still frees). |
| 2 | High (data loss) | `lib.rs:366-367`, `:382-383` | Any WASAPI error, including `AUDCLNT_E_DEVICE_INVALIDATED`, makes the capture thread exit silently. Nothing is reported to the caller, so the handle still looks alive. Changing the default output (plugging a headset, Bluetooth profile switch) is routine during calls. | Report the error through a channel or callback; reopen on the new default endpoint, with a bounded retry. Add an `IMMNotificationClient` or re-resolve on invalidation. |
| 3 | High if confirmed | `lib.rs:369-371`, `capture.rs:1338` | Loopback is polled and only emits when a packet exists. Suspected: with nothing playing, shared-mode loopback may deliver no packets, so no callback fires, and `since_last_callback > 2 s` is read as a stall and triggers repeated rebuilds. The adapter comment claims silent packets arrive on schedule; that is unverified. | Test on hardware first. If confirmed, emit wall-clock-paced zero blocks while idle (or hold a silent render stream open) so the clock never gaps. |
| 4 | Medium | `capture.rs` mixer, `mixer.rs:4-11` | Mic is the master clock; the system ring is drift-corrected by fill level, no timestamps. If #3 is real, silent stretches let the ring drain and the `Me`/`Others` timelines can shift by the amount of silence. | Hardware test: play sound, pause 30 s, play again, compare track offsets. Use `GetBuffer` QPC positions if drift appears. |
| 5 | Medium | `app/src-tauri` | Tauri app crate is not built in any branch CI. Windows-only code paths (`model_init.rs` PowerShell spawn, `paths.rs` home dir, no Unix 0600 permissions) are unverified. | Add a CI job that builds the UI, then `cargo check -p myna-app`, then an unsigned `tauri build`. |
| 6 | Medium | `paths.rs:336-502` | File mode hardening (0600) is `cfg(unix)` only. On Windows session and meeting files inherit the profile ACLs. Acceptable (per-user profile), but no explicit test or doc. | Document; no code unless needed. |
| 7 | Medium (fit for this fork) | `download-models.ps1:121` | Default download fetches Parakeet, **Qwen 4.7 GB**, and VAD. Contradicts ADR 0017 (summary off by default). Windows download needs no Python (`Invoke-WebRequest`), which is good. | Default selector without Qwen; Qwen opt-in; keep the `-Only qwen` path. |
| 8 | Low | `lib.rs:222-240` | Polling thread with 10 ms sleep: cheap, but no MMCSS thread priority, so capture can glitch under heavy CPU load. | Optional `AvSetMmThreadCharacteristics("Pro Audio")` inside the probe crate. |
| 9 | Low | `lib.rs sample_layout` | Only float32, PCM16, PCM32. 24-bit packed mix formats return an error. Shared-mode mix is float32 in practice. | None unless seen. |
| 10 | Info | `system_windows.rs:5-9` | Loopback captures the whole default mix, including notifications and any other app. Per-process loopback exists on Windows 10 2004+ (`ActivateAudioInterfaceAsync`, process loopback) and would allow capturing only Teams. | Backlog idea, not baseline. |

## Good foundations

- All `unsafe` stays in one crate; the rest of the workspace keeps `forbid(unsafe_code)`.
- Sample-layout detection reads the unaligned `SubFormat` safely.
- Thread owns its COM apartment and is joined on drop; startup has a timeout.
- Debug-only logging of the captured source name; nothing in release logs.
- Session manifest and crash recovery are platform-neutral (ADR 0011).
- Static-CRT env for MSVC (`crt-static`, `LLAMA_STATIC_CRT`) is already worked out in the release workflow and reused in `windows-ci.yml`.

## Order of work

1. Fix #1 (small, safe, unit-neutral) and #2 (add reporting plus reopen).
2. Add CI job for the app crate (#5).
3. Hardware recipe for #3 and #4 before any further capture work.
4. Align model download defaults with ADR 0017 (#7).
