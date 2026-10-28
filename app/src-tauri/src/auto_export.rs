//! Export folder (PRD R5): when the user picks a folder, every meeting is
//! written there as one Markdown file as soon as its transcript, speaker
//! labels or summary change, so downstream tooling can pick the file up with
//! no manual step.
//!
//! The folder lives under the `"export"` key of `<data_root>/preferences.json`
//! and mirrors [`crate::summary_prefs`]'s storage contract: reads never fail,
//! writes merge into the existing top-level object.
//!
//! Exporting is a convenience layered on top of operations that already
//! succeeded, so [`after_save`] never returns an error: a full disk or a
//! missing network drive must not fail a transcription.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::commands::export::{render_contract, to_local};
use crate::domain::{Meeting, MeetingId};
use crate::error::AppError;
use crate::paths;
use crate::store::MeetingStore;

const PREFERENCES_FILE: &str = "preferences.json";
const EXPORT_KEY: &str = "export";

/// Longest title slug, in characters, kept in an exported file name.
const MAX_SLUG_CHARS: usize = 60;

/// Persisted export settings.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExportPrefs {
    /// Absolute path of the export folder; `None` turns exporting off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
}

/// Loads the export settings. Never fails: a missing, unreadable or corrupt
/// file yields the defaults (exporting off).
pub fn load(root: &Path) -> ExportPrefs {
    let Ok(raw) = fs::read_to_string(root.join(PREFERENCES_FILE)) else {
        return ExportPrefs::default();
    };
    serde_json::from_str::<Value>(&raw)
        .ok()
        .and_then(|value| value.get(EXPORT_KEY).cloned())
        .and_then(|export| serde_json::from_value(export).ok())
        .unwrap_or_default()
}

/// Persists `prefs` under the `"export"` key, keeping every other key.
pub fn save(root: &Path, prefs: &ExportPrefs) -> Result<(), AppError> {
    let path = root.join(PREFERENCES_FILE);
    let mut map = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| match value {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default();
    map.insert(
        EXPORT_KEY.to_string(),
        serde_json::to_value(prefs).map_err(|err| AppError::Store(err.to_string()))?,
    );
    paths::create_dir_all_0700(root)?;
    let json = serde_json::to_string_pretty(&Value::Object(map))
        .map_err(|err| AppError::Store(err.to_string()))?;
    paths::write_0600(&path, json.as_bytes())?;
    Ok(())
}

/// Validates a folder chosen in Settings: it must be an absolute path that
/// is, or can become, a directory.
pub fn prepare_folder(raw: &str) -> Result<String, AppError> {
    let folder = raw.trim();
    let path = Path::new(folder);
    if !path.is_absolute() {
        return Err(AppError::Path(
            "the export folder must be an absolute path".to_string(),
        ));
    }
    fs::create_dir_all(path)
        .map_err(|err| AppError::Path(format!("cannot use this export folder: {err}")))?;
    if !path.is_dir() {
        return Err(AppError::Path(
            "the export folder is not a directory".to_string(),
        ));
    }
    Ok(folder.to_string())
}

/// `YYYY-MM-DD_hh-mm_title-slug.md`, in local time.
///
/// Deterministic for a given meeting, so re-exporting overwrites the same
/// file instead of piling up copies. The date prefix also means the name can
/// never be a reserved Windows device name.
pub fn file_name(meeting: &Meeting) -> String {
    let local = to_local(meeting.created_at);
    format!(
        "{:04}-{:02}-{:02}_{:02}-{:02}_{}.md",
        local.year(),
        u8::from(local.month()),
        local.day(),
        local.hour(),
        local.minute(),
        slug(&meeting.title)
    )
}

/// A file-system-safe rendering of `title`: letters and digits kept
/// (accents included), everything else collapsed into single dashes.
fn slug(title: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in title.chars() {
        if c.is_alphanumeric() {
            if gap && !out.is_empty() {
                out.push('-');
            }
            gap = false;
            out.push(c);
        } else {
            gap = true;
        }
    }
    let trimmed: String = out.chars().take(MAX_SLUG_CHARS).collect();
    let trimmed = trimmed.trim_end_matches('-');
    if trimmed.is_empty() {
        "meeting".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Writes `id` into `folder` as one Markdown file and returns its path.
///
/// Written to a hidden temporary file then renamed, so a tool watching the
/// folder never reads a half-written file.
pub fn export_to_folder(
    store: &dyn MeetingStore,
    id: MeetingId,
    folder: &Path,
) -> Result<PathBuf, AppError> {
    let (meeting, markdown) = render_contract(store, id)?;
    fs::create_dir_all(folder)?;
    let name = file_name(&meeting);
    let target = folder.join(&name);
    let temporary = folder.join(format!(".{name}.tmp"));
    fs::write(&temporary, markdown)?;
    fs::rename(&temporary, &target).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })?;
    Ok(target)
}

/// Exports `id` when an export folder is configured. Call it right after a
/// meeting is saved. Never fails the caller; problems are logged.
pub fn after_save(store: &dyn MeetingStore, id: MeetingId) {
    let Ok(root) = paths::data_root() else {
        return;
    };
    let Some(folder) = load(&root).folder else {
        return;
    };
    if let Err(err) = export_to_folder(store, id, Path::new(&folder)) {
        eprintln!("myna-app: export to folder failed: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::fs_store::FsMeetingStore;
    use myna_stt::{Speaker, Transcript, TranscriptSegment};

    fn segment(start: f32, text: &str, speaker: Speaker) -> TranscriptSegment {
        TranscriptSegment {
            start_sec: start,
            end_sec: start + 1.0,
            text: text.to_string(),
            speaker,
            speaker_pinned: false,
            suspect_reasons: Vec::new(),
            original_text: None,
            edited: false,
        }
    }

    fn saved_meeting(store: &FsMeetingStore, title: &str) -> Meeting {
        let meeting = store.create(title).expect("create");
        let updated = meeting
            .with_transcript(Transcript {
                segments: vec![segment(4.0, "Bonjour.", Speaker::me())],
            })
            .with_duration(42.0);
        store.save(&updated).expect("save");
        updated
    }

    #[test]
    fn slug_keeps_letters_and_digits_and_collapses_the_rest() {
        assert_eq!(slug("Point projet"), "Point-projet");
        assert_eq!(slug("  Réunion: équipe / Q3 !! "), "Réunion-équipe-Q3");
        assert_eq!(slug("a<b>c|d?e*f\"g\\h"), "a-b-c-d-e-f-g-h");
    }

    #[test]
    fn slug_falls_back_when_nothing_usable_is_left() {
        assert_eq!(slug(""), "meeting");
        assert_eq!(slug("???"), "meeting");
    }

    #[test]
    fn slug_is_capped_without_a_trailing_dash() {
        let long = "mot ".repeat(40);
        let out = slug(&long);
        assert!(out.chars().count() <= MAX_SLUG_CHARS);
        assert!(!out.ends_with('-'));
    }

    #[test]
    fn file_name_is_dated_and_markdown() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsMeetingStore::new(dir.path());
        let meeting = saved_meeting(&store, "Point projet");

        let name = file_name(&meeting);

        assert!(name.ends_with("_Point-projet.md"), "{name}");
        let (date, _) = name.split_once('_').expect("date prefix");
        assert_eq!(date.len(), 10, "{name}");
    }

    #[test]
    fn exporting_writes_one_file_and_overwrites_it_on_a_second_export() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsMeetingStore::new(dir.path());
        let meeting = saved_meeting(&store, "Point projet");
        let out = dir.path().join("notes").join("deeper");

        let first = export_to_folder(&store, meeting.id, &out).expect("first export");
        let second = export_to_folder(&store, meeting.id, &out).expect("second export");

        assert_eq!(first, second);
        let entries: Vec<_> = fs::read_dir(&out).expect("read dir").collect();
        assert_eq!(entries.len(), 1, "no temporary file may be left behind");
        let text = fs::read_to_string(&first).expect("read");
        assert!(text.starts_with("---\ntitle: Point projet\n"), "{text}");
        assert!(text.contains("[00:00:04] Me: Bonjour.\n"), "{text}");
    }

    #[test]
    fn prefs_round_trip_and_keep_other_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
            dir.path().join(PREFERENCES_FILE),
            br#"{"summary":{"guidelines":"Be brief."}}"#,
        )
        .expect("seed");

        save(
            dir.path(),
            &ExportPrefs {
                folder: Some("C:\\notes".into()),
            },
        )
        .expect("save");

        assert_eq!(load(dir.path()).folder.as_deref(), Some("C:\\notes"));
        let raw = fs::read_to_string(dir.path().join(PREFERENCES_FILE)).expect("read");
        let value: Value = serde_json::from_str(&raw).expect("json");
        assert_eq!(value["summary"]["guidelines"], "Be brief.");
    }

    #[test]
    fn prefs_default_to_exporting_off_when_the_file_is_missing_or_corrupt() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(load(dir.path()), ExportPrefs::default());
        fs::write(dir.path().join(PREFERENCES_FILE), b"{not json").expect("write");
        assert_eq!(load(dir.path()), ExportPrefs::default());
    }

    #[test]
    fn prepare_folder_requires_an_absolute_path_and_creates_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(prepare_folder("relative/notes").is_err());
        assert!(prepare_folder("").is_err());

        let wanted = dir.path().join("a").join("b");
        let stored = prepare_folder(&format!(" {} ", wanted.display())).expect("prepare");

        assert_eq!(stored, wanted.display().to_string());
        assert!(wanted.is_dir());
    }
}
