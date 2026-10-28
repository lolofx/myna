//! Integration tests for [`export_meeting_confined`] (the testable core of
//! `export_meeting_blocking`) proving Phase 3a's speaker attribution reaches
//! export output without breaking legacy (pre-speaker) meetings.

use std::fs;

use myna_app::commands::export::{export_meeting_confined, ExportFormat};
use myna_app::store::fs_store::FsMeetingStore;
use myna_app::store::MeetingStore;
use myna_stt::{Speaker, Transcript, TranscriptSegment};

fn store_at(root: &std::path::Path) -> FsMeetingStore {
    FsMeetingStore::new(root)
}

fn segment(start_sec: f32, end_sec: f32, text: &str, speaker: Speaker) -> TranscriptSegment {
    TranscriptSegment {
        start_sec,
        end_sec,
        text: text.to_string(),
        speaker,
        speaker_pinned: false,
        suspect_reasons: Vec::new(),
        original_text: None,
        edited: false,
    }
}

#[test]
fn a_transcript_whose_segments_are_all_unknown_exports_byte_identically_to_pre_speaker_output() {
    // Arrange: every segment defaults to `Speaker::unknown()` — exactly the
    // shape every meeting recorded before this phase has on disk.
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_at(dir.path());
    let meeting = store.create("Legacy meeting").expect("create");
    let transcript = Transcript {
        segments: vec![
            segment(0.0, 1.5, "Hello there.", Speaker::default()),
            segment(1.5, 3.2, "General kenobi.", Speaker::default()),
        ],
    };
    store
        .save(&meeting.with_transcript(transcript))
        .expect("save");

    // Act
    let md_dest = dir.path().join("legacy.md");
    export_meeting_confined(
        &store,
        meeting.id,
        ExportFormat::Markdown,
        &md_dest,
        dir.path(),
    )
    .expect("export markdown");
    let txt_dest = dir.path().join("legacy.txt");
    export_meeting_confined(
        &store,
        meeting.id,
        ExportFormat::Text,
        &txt_dest,
        dir.path(),
    )
    .expect("export text");

    // Assert: no speaker prefix of any kind leaks into a legacy export, and
    // the front matter lists no speakers.
    let markdown = fs::read_to_string(&md_dest).expect("read markdown");
    assert_eq!(
        body(&markdown),
        "[00:00:00] Hello there.\n[00:00:01] General kenobi.\n"
    );
    assert!(!markdown.contains("speakers:"));
    assert!(!markdown.contains("Others"));
    assert!(markdown.contains("\nduration: 00:00:03\n"), "{markdown}");
    assert!(markdown.contains("\nstatus: to-summarize\n"), "{markdown}");

    let text = fs::read_to_string(&txt_dest).expect("read text");
    assert_eq!(
        text,
        "Legacy meeting\n\nTranscript:\nHello there.\nGeneral kenobi.\n\n"
    );
}

#[test]
fn a_mixed_speaker_transcript_exports_with_prefixes_and_merges_consecutive_same_speaker() {
    // Arrange: two consecutive `me` segments (must merge under one header),
    // then an unidentified `others`, then a specific `others:2`.
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_at(dir.path());
    let meeting = store.create("Mixed meeting").expect("create");
    let transcript = Transcript {
        segments: vec![
            segment(0.0, 1.0, "Let's start.", Speaker::me()),
            segment(1.0, 2.0, "Agenda first.", Speaker::me()),
            segment(2.0, 3.0, "Sounds good.", Speaker::others()),
            segment(3.0, 4.0, "I agree too.", Speaker::others_id("2")),
        ],
    };
    store
        .save(&meeting.with_transcript(transcript))
        .expect("save");

    // Act
    let md_dest = dir.path().join("mixed.md");
    export_meeting_confined(
        &store,
        meeting.id,
        ExportFormat::Markdown,
        &md_dest,
        dir.path(),
    )
    .expect("export markdown");
    let txt_dest = dir.path().join("mixed.txt");
    export_meeting_confined(
        &store,
        meeting.id,
        ExportFormat::Text,
        &txt_dest,
        dir.path(),
    )
    .expect("export text");

    // Assert: every line carries its speaker, and the front matter lists
    // each distinct speaker once, in order of first appearance.
    let markdown = fs::read_to_string(&md_dest).expect("read markdown");
    assert_eq!(
        body(&markdown),
        "[00:00:00] Me: Let's start.\n\
         [00:00:01] Me: Agenda first.\n\
         [00:00:02] Others: Sounds good.\n\
         [00:00:03] Others 2: I agree too.\n"
    );
    assert!(
        markdown.contains("\nspeakers: [Me, Others, Others 2]\n"),
        "{markdown}"
    );

    let text = fs::read_to_string(&txt_dest).expect("read text");
    assert_eq!(
        text,
        "Mixed meeting\n\nTranscript:\n\
         Me:\n\
         Let's start.\n\
         Agenda first.\n\
         Others:\n\
         Sounds good.\n\
         Others 2:\n\
         I agree too.\n\n"
    );
}

#[test]
fn a_named_speaker_renders_its_display_name_in_markdown_and_text_export() {
    // Arrange: "others:1" has a user-assigned display name containing
    // uppercase, an accent, and a hyphen — none of which are legal in a
    // `Speaker` label itself, so it must survive only via `speaker_names`.
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_at(dir.path());
    let meeting = store.create("Named meeting").expect("create");
    let transcript = Transcript {
        segments: vec![segment(
            0.0,
            1.0,
            "I'll own the migration.",
            Speaker::others_id("1"),
        )],
    };
    let mut speaker_names = std::collections::BTreeMap::new();
    speaker_names.insert("others:1".to_string(), "Jean-Éric".to_string());
    store
        .save(
            &meeting
                .with_transcript(transcript)
                .with_speaker_names(speaker_names),
        )
        .expect("save");

    // Act
    let md_dest = dir.path().join("named.md");
    export_meeting_confined(
        &store,
        meeting.id,
        ExportFormat::Markdown,
        &md_dest,
        dir.path(),
    )
    .expect("export markdown");
    let txt_dest = dir.path().join("named.txt");
    export_meeting_confined(
        &store,
        meeting.id,
        ExportFormat::Text,
        &txt_dest,
        dir.path(),
    )
    .expect("export text");

    // Assert
    let markdown = fs::read_to_string(&md_dest).expect("read markdown");
    assert!(markdown.contains("] Jean-Éric: I'll own the migration.\n"));
    assert!(markdown.contains("\nspeakers: [Jean-Éric]\n"), "{markdown}");
    assert!(!markdown.contains("Others 1"));

    let text = fs::read_to_string(&txt_dest).expect("read text");
    assert!(text.contains("Jean-Éric:\n"));
    assert!(!text.contains("Others 1"));
}

#[test]
fn two_distinct_labels_sharing_a_display_name_export_as_separate_blocks() {
    // Arrange: "others:1" and "others:2" both happen to be named "Sam" — the
    // grouping must stay keyed by the underlying label, not the display
    // name, so these must not merge into one run.
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_at(dir.path());
    let meeting = store.create("Same name meeting").expect("create");
    let transcript = Transcript {
        segments: vec![
            segment(0.0, 1.0, "First voice.", Speaker::others_id("1")),
            segment(1.0, 2.0, "Second voice.", Speaker::others_id("2")),
        ],
    };
    let mut speaker_names = std::collections::BTreeMap::new();
    speaker_names.insert("others:1".to_string(), "Sam".to_string());
    speaker_names.insert("others:2".to_string(), "Sam".to_string());
    store
        .save(
            &meeting
                .with_transcript(transcript)
                .with_speaker_names(speaker_names),
        )
        .expect("save");

    // Act
    let md_dest = dir.path().join("same-name.md");
    export_meeting_confined(
        &store,
        meeting.id,
        ExportFormat::Markdown,
        &md_dest,
        dir.path(),
    )
    .expect("export markdown");

    // Assert: both lines keep their own speaker, and the shared display name
    // is listed once in the front matter.
    let markdown = fs::read_to_string(&md_dest).expect("read markdown");
    assert_eq!(
        body(&markdown),
        "[00:00:00] Sam: First voice.\n[00:00:01] Sam: Second voice.\n"
    );
    assert!(markdown.contains("\nspeakers: [Sam]\n"), "{markdown}");
}

#[test]
fn segments_are_written_in_chronological_order_with_one_line_each() {
    // Arrange: the two tracks interleave, and a segment holds a line break.
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_at(dir.path());
    let meeting = store.create("Ordering").expect("create");
    let transcript = Transcript {
        segments: vec![
            segment(61.0, 62.0, "Second.", Speaker::others()),
            segment(5.0, 6.0, "First\nline  broken.", Speaker::me()),
            segment(3725.0, 3726.0, "Last.", Speaker::me()),
        ],
    };
    store
        .save(&meeting.with_transcript(transcript).with_duration(3726.0))
        .expect("save");

    // Act
    let dest = dir.path().join("ordering.md");
    export_meeting_confined(
        &store,
        meeting.id,
        ExportFormat::Markdown,
        &dest,
        dir.path(),
    )
    .expect("export markdown");

    // Assert
    let markdown = fs::read_to_string(&dest).expect("read markdown");
    assert_eq!(
        body(&markdown),
        "[00:00:05] Me: First line broken.\n\
         [00:01:01] Others: Second.\n\
         [01:02:05] Me: Last.\n"
    );
    assert!(markdown.contains("\nduration: 01:02:06\n"), "{markdown}");
}

#[test]
fn the_front_matter_is_well_formed_and_quotes_what_yaml_would_misread() {
    // Arrange: a title with a colon would otherwise start a nested mapping.
    let dir = tempfile::tempdir().expect("tempdir");
    let store = store_at(dir.path());
    let meeting = store.create("Q3: budget \"review\"").expect("create");
    store
        .save(&meeting.with_transcript(Transcript {
            segments: vec![segment(0.0, 1.0, "Hi.", Speaker::me())],
        }))
        .expect("save");

    // Act
    let dest = dir.path().join("front.md");
    export_meeting_confined(
        &store,
        meeting.id,
        ExportFormat::Markdown,
        &dest,
        dir.path(),
    )
    .expect("export markdown");

    // Assert
    let markdown = fs::read_to_string(&dest).expect("read markdown");
    assert!(markdown.starts_with("---\ntitle: \"Q3: budget \\\"review\\\"\"\ndate: "));
    let front: Vec<&str> = markdown
        .lines()
        .skip(1)
        .take_while(|line| *line != "---")
        .collect();
    let keys: Vec<&str> = front
        .iter()
        .map(|line| line.split(':').next().unwrap_or(""))
        .collect();
    assert_eq!(keys, ["title", "date", "duration", "speakers", "status"]);
    let date = front[1].trim_start_matches("date: ");
    assert_eq!(date.len(), 19, "{date}");
    assert_eq!(&date[10..11], "T");
}

/// The part of an exported file after its front matter.
fn body(markdown: &str) -> &str {
    markdown
        .split_once("\n---\n\n")
        .map(|(_, rest)| rest)
        .expect("front matter is closed by a blank line")
}
