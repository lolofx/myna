//! When transcription runs, and with how many threads, persisted to
//! `<data_root>/preferences.json` under the `"transcription"` key this
//! module owns. Mirrors [`crate::summary_prefs`]'s storage contract: reads
//! never fail (a missing or corrupt file yields defaults).
//!
//! Two modes (see ADR 0017):
//! - [`TranscriptionMode::Live`]: the speech model decodes while recording,
//!   giving live captions at the cost of memory and CPU during the meeting.
//! - [`TranscriptionMode::After`]: recording only captures audio; the
//!   meeting is transcribed once it stops, with a small thread count.
//!
//! The default is `After` on Windows and `Live` elsewhere, so macOS keeps
//! upstream behavior. `MYNA_TRANSCRIPTION=live|after` overrides both the
//! default and the stored value, which is how to switch without a settings
//! screen. Example `preferences.json` fragment:
//!
//! ```json
//! { "transcription": { "mode": "after", "threads": 2 } }
//! ```

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::paths;

const PREFERENCES_FILE: &str = "preferences.json";
const TRANSCRIPTION_KEY: &str = "transcription";

/// Environment variable overriding the transcription mode.
pub const MODE_ENV_VAR: &str = "MYNA_TRANSCRIPTION";

/// Thread count for the speech model in [`TranscriptionMode::After`] when
/// none is configured: enough to finish in reasonable time, few enough to
/// leave the machine usable.
pub const DEFERRED_THREADS_DEFAULT: i32 = 2;
/// Smallest and largest configurable thread count.
pub const THREADS_MIN: u8 = 1;
pub const THREADS_MAX: u8 = 4;

/// When the speech model runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionMode {
    /// Decode while recording (live captions).
    Live,
    /// Capture only; transcribe after the recording stops.
    After,
}

/// Persisted choices; `None` means "use the default".
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TranscriptionPrefs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<TranscriptionMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threads: Option<u8>,
}

/// The mode used when neither the environment nor preferences name one.
pub fn platform_default_mode() -> TranscriptionMode {
    if cfg!(windows) {
        TranscriptionMode::After
    } else {
        TranscriptionMode::Live
    }
}

/// Parses `live` / `after`, ignoring case and surrounding whitespace.
pub fn parse_mode(raw: &str) -> Option<TranscriptionMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "live" => Some(TranscriptionMode::Live),
        "after" => Some(TranscriptionMode::After),
        _ => None,
    }
}

/// Resolves the mode: a valid environment override wins, then the stored
/// choice, then the platform default. An unparseable override is ignored.
pub fn effective_mode(prefs: &TranscriptionPrefs, env_override: Option<&str>) -> TranscriptionMode {
    env_override
        .and_then(parse_mode)
        .or(prefs.mode)
        .unwrap_or_else(platform_default_mode)
}

/// Thread count for the speech model, or `None` to keep the automatic
/// choice. A configured value is clamped to [`THREADS_MIN`]..=[`THREADS_MAX`];
/// with none configured, [`TranscriptionMode::After`] uses
/// [`DEFERRED_THREADS_DEFAULT`] and [`TranscriptionMode::Live`] stays
/// automatic.
pub fn effective_threads(prefs: &TranscriptionPrefs, mode: TranscriptionMode) -> Option<i32> {
    match (prefs.threads, mode) {
        (Some(threads), _) => Some(i32::from(threads.clamp(THREADS_MIN, THREADS_MAX))),
        (None, TranscriptionMode::After) => Some(DEFERRED_THREADS_DEFAULT),
        (None, TranscriptionMode::Live) => None,
    }
}

/// Loads transcription preferences from `<root>/preferences.json`. Never
/// errors: a missing, unreadable or corrupt file, or a `"transcription"`
/// value of the wrong shape, yields [`TranscriptionPrefs::default`].
pub fn load(root: &Path) -> TranscriptionPrefs {
    let Ok(raw) = fs::read_to_string(root.join(PREFERENCES_FILE)) else {
        return TranscriptionPrefs::default();
    };
    let Ok(root_value) = serde_json::from_str::<Value>(&raw) else {
        return TranscriptionPrefs::default();
    };
    root_value
        .get(TRANSCRIPTION_KEY)
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

fn load_current() -> TranscriptionPrefs {
    paths::data_root()
        .map(|root| load(&root))
        .unwrap_or_default()
}

/// The mode in force now, from the real environment and preferences.
pub fn current_mode() -> TranscriptionMode {
    effective_mode(&load_current(), std::env::var(MODE_ENV_VAR).ok().as_deref())
}

/// The speech-model thread count in force now, or `None` for automatic.
pub fn current_engine_threads() -> Option<i32> {
    let prefs = load_current();
    let mode = effective_mode(&prefs, std::env::var(MODE_ENV_VAR).ok().as_deref());
    effective_threads(&prefs, mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefs(mode: Option<TranscriptionMode>, threads: Option<u8>) -> TranscriptionPrefs {
        TranscriptionPrefs { mode, threads }
    }

    #[test]
    fn parse_mode_accepts_both_names_in_any_case() {
        assert_eq!(parse_mode("live"), Some(TranscriptionMode::Live));
        assert_eq!(parse_mode(" AFTER "), Some(TranscriptionMode::After));
        assert_eq!(parse_mode("later"), None);
        assert_eq!(parse_mode(""), None);
    }

    #[test]
    fn the_default_mode_is_after_on_windows_and_live_elsewhere() {
        let expected = if cfg!(windows) {
            TranscriptionMode::After
        } else {
            TranscriptionMode::Live
        };
        assert_eq!(platform_default_mode(), expected);
        assert_eq!(
            effective_mode(&TranscriptionPrefs::default(), None),
            expected
        );
    }

    #[test]
    fn a_stored_mode_beats_the_platform_default() {
        let stored = prefs(Some(TranscriptionMode::Live), None);
        assert_eq!(effective_mode(&stored, None), TranscriptionMode::Live);
        let stored = prefs(Some(TranscriptionMode::After), None);
        assert_eq!(effective_mode(&stored, None), TranscriptionMode::After);
    }

    #[test]
    fn the_environment_beats_the_stored_mode() {
        let stored = prefs(Some(TranscriptionMode::Live), None);
        assert_eq!(
            effective_mode(&stored, Some("after")),
            TranscriptionMode::After
        );
    }

    #[test]
    fn an_invalid_environment_value_is_ignored() {
        let stored = prefs(Some(TranscriptionMode::After), None);
        assert_eq!(
            effective_mode(&stored, Some("nonsense")),
            TranscriptionMode::After
        );
    }

    #[test]
    fn deferred_transcription_defaults_to_two_threads() {
        let none = TranscriptionPrefs::default();
        assert_eq!(effective_threads(&none, TranscriptionMode::After), Some(2));
    }

    #[test]
    fn live_transcription_keeps_the_automatic_thread_choice() {
        let none = TranscriptionPrefs::default();
        assert_eq!(effective_threads(&none, TranscriptionMode::Live), None);
    }

    #[test]
    fn configured_threads_are_clamped_to_the_allowed_range() {
        let low = prefs(None, Some(0));
        let high = prefs(None, Some(64));
        let ok = prefs(None, Some(3));
        assert_eq!(effective_threads(&low, TranscriptionMode::After), Some(1));
        assert_eq!(effective_threads(&high, TranscriptionMode::Live), Some(4));
        assert_eq!(effective_threads(&ok, TranscriptionMode::After), Some(3));
    }

    #[test]
    fn load_returns_defaults_for_a_missing_or_corrupt_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(load(dir.path()), TranscriptionPrefs::default());

        fs::write(dir.path().join(PREFERENCES_FILE), b"{ not json").expect("write");
        assert_eq!(load(dir.path()), TranscriptionPrefs::default());
    }

    #[test]
    fn load_reads_the_transcription_key_and_ignores_the_rest() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
            dir.path().join(PREFERENCES_FILE),
            br#"{"summary":{"guidelines":"x"},"transcription":{"mode":"after","threads":3}}"#,
        )
        .expect("write");

        let loaded = load(dir.path());

        assert_eq!(loaded, prefs(Some(TranscriptionMode::After), Some(3)));
    }

    #[test]
    fn load_falls_back_to_defaults_for_an_unknown_mode_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
            dir.path().join(PREFERENCES_FILE),
            br#"{"transcription":{"mode":"someday"}}"#,
        )
        .expect("write");

        assert_eq!(load(dir.path()), TranscriptionPrefs::default());
    }
}
