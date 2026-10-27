//! Catalog of the summary models a user can pick from, and the rules that
//! decide which one is used.
//!
//! The summary model is optional: nothing in onboarding requires it, and the
//! app only loads one when a summary is requested. Every entry is an
//! official Qwen2.5-Instruct GGUF (Q4_K_M) stored in its own directory under
//! the models root, so several can sit side by side. The 7B entry keeps the
//! directory name earlier releases used, so existing installs are found
//! without migration.

use std::path::Path;

use serde::Serialize;

use crate::model_init::DownloadArtifact;
use crate::paths;

/// One selectable summary model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryModel {
    /// Stable id persisted in preferences and exchanged over IPC.
    pub id: &'static str,
    pub label: &'static str,
    /// Directory name under the models root.
    pub dir_name: &'static str,
    /// Download size, to show before the user commits to it.
    pub approx_size_mb: u32,
    /// What `scripts/download-models` fetches for this model.
    pub artifact: DownloadArtifact,
}

/// Best quality first: [`resolve`] prefers the earliest installed entry when
/// no model was chosen explicitly, which keeps an existing 7B install in use.
pub const SUMMARY_MODELS: [SummaryModel; 3] = [
    SummaryModel {
        id: "qwen2.5-7b",
        label: "Qwen2.5 7B (best quality)",
        dir_name: "qwen2.5-7b-instruct",
        approx_size_mb: 4690,
        artifact: DownloadArtifact::Qwen,
    },
    SummaryModel {
        id: "qwen2.5-3b",
        label: "Qwen2.5 3B (balanced)",
        dir_name: "qwen2.5-3b-instruct",
        approx_size_mb: 2100,
        artifact: DownloadArtifact::Qwen3B,
    },
    SummaryModel {
        id: "qwen2.5-1.5b",
        label: "Qwen2.5 1.5B (lightest)",
        dir_name: "qwen2.5-1.5b-instruct",
        approx_size_mb: 1120,
        artifact: DownloadArtifact::Qwen1_5B,
    },
];

/// Looks a model up by its persisted id.
pub fn find(id: &str) -> Option<&'static SummaryModel> {
    SUMMARY_MODELS.iter().find(|model| model.id == id)
}

/// Whether a loadable GGUF is present for `model` under `models_root`.
/// Any `.gguf` counts, including a symlink, so a hand-placed model works.
pub fn is_installed(models_root: &Path, model: &SummaryModel) -> bool {
    paths::first_gguf(&models_root.join(model.dir_name)).is_some()
}

/// The model a summary would use right now, or `None` when none can run.
///
/// An explicit choice wins, but only while it is installed: a chosen model
/// that is missing yields `None` rather than silently running another, so
/// the UI can offer to download it. With no explicit choice, the best
/// installed model is used.
pub fn resolve(models_root: &Path, selected: Option<&str>) -> Option<&'static SummaryModel> {
    match selected {
        Some(id) => find(id).filter(|model| is_installed(models_root, model)),
        None => SUMMARY_MODELS
            .iter()
            .find(|model| is_installed(models_root, model)),
    }
}

/// Whether any catalog model is installed.
pub fn any_installed(models_root: &Path) -> bool {
    SUMMARY_MODELS
        .iter()
        .any(|model| is_installed(models_root, model))
}

/// One catalog entry as the UI sees it, IPC-facing.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SummaryModelSlot {
    pub id: String,
    pub label: String,
    pub approx_size_mb: u32,
    pub present: bool,
    /// Whether this is the model [`resolve`] picks.
    pub selected: bool,
}

/// Every catalog entry with its presence and whether it is the one in use.
pub fn slots(models_root: &Path, selected: Option<&str>) -> Vec<SummaryModelSlot> {
    let active = resolve(models_root, selected).map(|model| model.id);
    SUMMARY_MODELS
        .iter()
        .map(|model| SummaryModelSlot {
            id: model.id.to_string(),
            label: model.label.to_string(),
            approx_size_mb: model.approx_size_mb,
            present: is_installed(models_root, model),
            selected: active == Some(model.id),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install(root: &Path, model: &SummaryModel) {
        let dir = root.join(model.dir_name);
        std::fs::create_dir_all(&dir).expect("create model dir");
        std::fs::write(dir.join("model.gguf"), b"gguf").expect("write gguf");
    }

    #[test]
    fn catalog_ids_and_directories_are_unique() {
        // Arrange / Act
        let mut ids: Vec<_> = SUMMARY_MODELS.iter().map(|model| model.id).collect();
        let mut dirs: Vec<_> = SUMMARY_MODELS.iter().map(|model| model.dir_name).collect();
        ids.sort_unstable();
        ids.dedup();
        dirs.sort_unstable();
        dirs.dedup();

        // Assert
        assert_eq!(ids.len(), SUMMARY_MODELS.len());
        assert_eq!(dirs.len(), SUMMARY_MODELS.len());
    }

    #[test]
    fn seven_b_keeps_the_directory_earlier_releases_used() {
        // Assert: existing installs must be found without migration.
        assert_eq!(
            find("qwen2.5-7b").map(|model| model.dir_name),
            Some("qwen2.5-7b-instruct")
        );
    }

    #[test]
    fn find_rejects_an_unknown_id() {
        assert!(find("gpt-4").is_none());
    }

    #[test]
    fn nothing_resolves_on_an_empty_models_root() {
        // Arrange
        let dir = tempfile::tempdir().expect("tempdir");

        // Act / Assert
        assert!(resolve(dir.path(), None).is_none());
        assert!(resolve(dir.path(), Some("qwen2.5-3b")).is_none());
        assert!(!any_installed(dir.path()));
    }

    #[test]
    fn without_a_choice_the_best_installed_model_is_used() {
        // Arrange: 1.5B and 3B installed.
        let dir = tempfile::tempdir().expect("tempdir");
        install(dir.path(), find("qwen2.5-1.5b").expect("1.5b"));
        install(dir.path(), find("qwen2.5-3b").expect("3b"));

        // Act
        let model = resolve(dir.path(), None);

        // Assert
        assert_eq!(model.map(|model| model.id), Some("qwen2.5-3b"));
    }

    #[test]
    fn an_explicit_choice_wins_over_a_better_installed_model() {
        // Arrange
        let dir = tempfile::tempdir().expect("tempdir");
        install(dir.path(), find("qwen2.5-7b").expect("7b"));
        install(dir.path(), find("qwen2.5-1.5b").expect("1.5b"));

        // Act
        let model = resolve(dir.path(), Some("qwen2.5-1.5b"));

        // Assert
        assert_eq!(model.map(|model| model.id), Some("qwen2.5-1.5b"));
    }

    #[test]
    fn a_chosen_model_that_is_not_installed_does_not_fall_back() {
        // Arrange: 7B installed, 3B chosen but never downloaded.
        let dir = tempfile::tempdir().expect("tempdir");
        install(dir.path(), find("qwen2.5-7b").expect("7b"));

        // Act / Assert
        assert!(resolve(dir.path(), Some("qwen2.5-3b")).is_none());
    }

    #[test]
    fn an_unknown_chosen_id_resolves_to_nothing() {
        // Arrange
        let dir = tempfile::tempdir().expect("tempdir");
        install(dir.path(), find("qwen2.5-7b").expect("7b"));

        // Act / Assert
        assert!(resolve(dir.path(), Some("not-a-model")).is_none());
    }

    #[test]
    fn slots_mark_presence_and_the_active_model() {
        // Arrange
        let dir = tempfile::tempdir().expect("tempdir");
        install(dir.path(), find("qwen2.5-3b").expect("3b"));

        // Act
        let slots = slots(dir.path(), None);

        // Assert
        let by_id = |id: &str| {
            slots
                .iter()
                .find(|slot| slot.id == id)
                .expect("slot")
                .clone()
        };
        assert!(by_id("qwen2.5-3b").present && by_id("qwen2.5-3b").selected);
        assert!(!by_id("qwen2.5-7b").present && !by_id("qwen2.5-7b").selected);
        assert!(!by_id("qwen2.5-1.5b").present && !by_id("qwen2.5-1.5b").selected);
    }
}
