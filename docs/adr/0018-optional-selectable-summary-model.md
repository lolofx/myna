# ADR 0018: Optional, Selectable Summary Model

**Status**: Proposed (Windows fork)
**Date**: 2026-10-27
**Supersedes in part**: [ADR 0013: Summarization Model](0013-summarization-model-qwen25-7b.md) (the fixed 7B choice and its place in onboarding)
**Builds on**: [ADR 0017: Record Then Transcribe on Windows](0017-record-then-transcribe-on-windows.md)
**Context**: Onboarding required three models, including the 4.7 GB Qwen2.5-7B, and the default download fetched it. The 7B needs several GB of RAM while a summary runs. A user who exports the transcript and summarizes elsewhere pays that cost for nothing, and a user on a modest machine has no lighter option.

## Decision

The summary model becomes optional and selectable.

- **Catalog**: three official Qwen2.5-Instruct Q4_K_M GGUFs, each in its own directory under the models root: 7B (`qwen2.5-7b-instruct`, the directory earlier releases used), 3B (`qwen2.5-3b-instruct`, about 2.1 GB) and 1.5B (`qwen2.5-1.5b-instruct`, about 1.1 GB). Defined in `app/src-tauri/src/summary_models.rs`.
- **Not required**: `all_present` covers transcription and voice detection only. Onboarding no longer lists the summary model.
- **Not downloaded by default**: `missing_artifacts` omits it, and `scripts/download-models.{sh,ps1}` fetch Parakeet and the VAD model unless asked (`--only qwen`, `qwen-3b`, `qwen-1.5b`).
- **Chosen in Settings**: the choice is stored as `model` under the `summary` key of `preferences.json`. Two commands: `set_summary_model` and `start_summary_model_download`. The download reuses the existing `models://progress` and `models://done` events.
- **Resolution** (`summary_models::resolve`): an explicit choice is used only while installed; a chosen model that is missing resolves to nothing instead of silently running another. With no explicit choice, the best installed model is used, so an existing 7B install keeps working untouched.
- **No model installed**: generating a summary fails with a message that points to Settings. Recording, transcription and export are unaffected.

## Rationale

### Why a catalog of Qwen sizes and not arbitrary models?

The prompts, chat template and language handling are tuned for Qwen2.5. Sizes of one family give a clear quality-versus-memory trade without new prompt work. A hand-placed GGUF in a catalog directory is still picked up, because presence means "any `.gguf` in the directory".

### Why not fall back to another installed model when the chosen one is missing?

Quality differs a lot between 1.5B and 7B. Silently using a different model than the one shown would make results hard to explain. Showing "download" is honest and cheap.

### Why keep the `qwen` slot in the status payload?

Older UI builds and fixtures read it. It now means "any summary model is installed". New fields (`summaryModels`, `selectedSummaryModel`) carry the detail, which keeps the fork mergeable with upstream.

## Consequences

- First-run needs about 0.7 GB (Parakeet and VAD) instead of about 5.4 GB.
- Users who want in-app summaries must pick and download a model once.
- The 3B and 1.5B sizes have not been evaluated for French quality; the 7B stays the reference. Evaluation belongs in the Windows recipe.

## Open Risks

- Summary quality on 1.5B and 3B for French meetings is unmeasured.
- The Windows download path is PowerShell with `Invoke-WebRequest`; large downloads have no resume.
