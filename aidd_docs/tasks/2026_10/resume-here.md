# Resume here

Where the fork stands and what to do next. Read [project-brief.md](../../memory/project-brief.md) and [backlog.md](../../memory/backlog.md) first; this file is the practical hand-over.

## Working rules (non negotiable, the repository is public)

- Branch `win/lightweight`. `main` mirrors upstream and receives no fork work.
- Commit identity is set per repository: the GitHub login with its `users.noreply.github.com` address. Check `git config user.name user.email` before the first commit; never rely on a global identity.
- No `Co-Authored-By`, no "Generated with" line, no session link, in commits or pull requests.
- No personal data in any file or commit: no real names, e-mail addresses, IP addresses, host names, machine models or specs, or private tooling. Examples use generic values (`http://ollama-host:11434`). Performance figures are relative and never name the PC.
- One concern per commit, conventional and short. Windows fixes apart from fork-specific features.
- Windows CI runs on demand only: `gh workflow run windows-ci.yml --ref <branch> --repo <owner>/myna` (always pass `--repo`). Verify locally, group fixes, trigger once.

## Set up a Windows PC

Prerequisites and the build settings are in [docs/windows-recipe.md](../../../docs/windows-recipe.md), section 0. The short version, in PowerShell:

```powershell
$env:RUSTFLAGS = "-C target-feature=+crt-static"
$env:LLAMA_STATIC_CRT = "1"
$env:CMAKE_MSVC_RUNTIME_LIBRARY = "MultiThreaded"
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
npm ci; npm --prefix ui ci
.\scripts\download-models.ps1
npx tauri dev
```

## Verification gate

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --no-fail-fast
cd ui; npm run lint; npx tsc -p tsconfig.json --noEmit; npm run build; npm test -- --watch=false
```

State at the last run: all green (about 800 Rust tests, 1561 UI tests). The UI test script uses `cross-env`, so it runs the same on Windows, macOS and Linux.

## Next steps, in order

### 1. Settings coverage (small, high value)

`transcription.threads` (1 to 4) and `transcription.mode` (after or live) exist in `preferences.json` and are read by `app/src-tauri/src/transcription_prefs.rs`, but the Settings window cannot change them. Add a "Transcription" section with a threads selector and a mode choice, following the pattern used for the export folder: a small facade next to `ExportFolderFacade`, a port and Tauri adapter, new commands in `commands.ts`, inputs and outputs on `SettingsComponent`, wiring in `meetings-shell.page.settings.support.ts`. The engine is cached, so changing threads needs a restart; say so in the UI.

### 2. Summary on the user's own server (phase 5)

Done: `crates/myna-llm/src/remote.rs` with 24 tests. Decisions already taken:

- Ollama native `/api/chat`, not the OpenAI-compatible route, because only the native one can pin `num_ctx` and switch thinking off (`think: false`).
- Plain `http://` only, no TLS stack, no new dependency. Addresses must resolve to loopback, private, link-local or shared (`100.64.0.0/10`) ranges; anything else is refused before connecting.
- Only text is sent, never audio. Idle timeout of 5 minutes, because a sleeping server must load its model first. Cancelling closes the socket, which stops generation.
- `num_ctx` 16384 and the same temperature, top-p and seed as the local engine. Long transcripts use map-reduce with a 3-characters-per-token estimate.

To do:

1. `SummaryPrefs` (`app/src-tauri/src/summary_prefs.rs`): add `remote: Option<{ enabled, base_url, model }>` with `#[serde(default)]`; update the struct literals in its tests.
2. Commands `get_remote_summary`, `set_remote_summary`, `test_remote_summary` (the last one calls `list_remote_models` and returns the model names, so Settings can offer a picker). Register them in `lib.rs` and `commands.ts`.
3. In `summarize_and_persist` (`commands/summary.rs`), when the server is enabled use `RemoteSummarizer::summarize_transcript` and skip `state.summarizer(app)?`, which also removes the "no model installed" error. Keep the same token streaming and cancel flag.
4. Settings section: enable switch, address, model picker filled by "Test connection", with a plain warning that the meeting text leaves the machine. Off by default. Never write the address anywhere but the local `preferences.json`.
5. Write ADR 0021 (privacy exception, private-network guard, why native route). The README privacy section already describes the opt-in exception.

### 3. Interface language, English and French (phase 6)

There is no i18n library today and all UI text is hard-coded English in templates and in a few Rust strings (menu labels, default meeting title, error messages). The summary language is a separate, already working setting; do not merge the two.

Decided in [ADR 0020](../../../docs/adr/0020-interface-language-with-transloco.md) (Proposed): Transloco, `en` and `fr` JSON files, language stored in `preferences.json`, applied before the first render, a parity spec, the Markdown export left untranslated on purpose. Do it in two steps so every commit stays reviewable: infrastructure plus the Settings window first, then the remaining components one feature area at a time. Expect merge conflicts in templates upstream also edits.

### 4. Beta releases and the fork's update channel (phase 7)

`app/src-tauri/tauri.conf.json` still sets the updater endpoint to the **upstream project's** `releases/latest/download/latest.json` with the upstream public key. The fork has to publish its own signed releases and point there. Decided: the fork gets its own channel. Prepared, not yet switched on:

- `.github/workflows/beta-windows.yml`: Windows-only, runs on `v*-beta*` tags, builds the NSIS installer (an MSI cannot carry a `-beta.1` version), publishes a GitHub pre-release and refreshes a rolling `beta-manifest` release that holds `latest.json`. GitHub's `releases/latest` ignores pre-releases, hence the rolling manifest.
- `scripts/make-latest-json-windows.ps1`: Windows-only manifest, with the tag-versus-version guard.
- `.github/workflows/release.yml` (upstream's macOS plus Windows flow) now skips beta tags.
- [docs/releasing-windows-beta.md](../../../docs/releasing-windows-beta.md): the one-time key setup and the release steps.

Remaining, in order: generate the fork's updater key pair (private key outside the repository, never committed), store it as the `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repository secrets, put the public key and the new endpoint in `tauri.conf.json` in the same commit, bump the version to `0.7.0-beta.1`, validate the packaged installer locally, then tag. Decide the product name and identifier too: the fork still installs as "Myna" with the upstream identifier, which collides with an upstream install.

### 4b. Keep the three platforms honest (phase 8)

Add a CI job that compiles and tests the Rust workspace on macOS and Linux, so a fork change cannot silently break upstream's platform. Fix the local-time gap of the Markdown export off Windows (ADR 0019, open risks).

### 5. Tray, hotkey, installer (phase 4)

See the PRD requirements R7 to R9. Needs a real PC to validate. Keep `unsafe_code = "forbid"`.

### 6. Finish the recipe

Run T8 (speaker detection), T9 (export folder), the summary model checks, the section 4 hardware tests, and the figures on a packaged build. Only then update the docs to claim Windows support.

## Keeping up with upstream

The fork has diverged: roughly 15 commits and a few dozen files on top of upstream. It can keep absorbing upstream releases if it stays additive and each change to an upstream-owned file stays small.

```powershell
git remote add upstream git@github.com:fmflurry/myna.git   # once; fetch only
git fetch upstream main
git switch main; git merge --ff-only upstream/main          # main must stay equal to upstream
git switch win/lightweight; git merge main                  # resolve conflicts here
```

Conflict hotspots, because both sides edit them: `settings.component.{ts,html}`, `meetings-shell.page.{html,settings.support.ts}`, `meetings.providers.ts`, `commands.ts`, `lib.rs` (command list), `commands/export.rs` and `tests/export.rs` (the Markdown shape was replaced), `summary_prefs.rs`, `meetings.facade.ts`. Merge often (after every upstream release) so conflicts stay small. Fork-only features live in their own files (`auto_export.rs`, `remote.rs`, `export-folder.*`), which merge cleanly. Contributing back is realistic for the Windows fixes only; the export shape, the export folder and the remote summary are fork-specific by design.
