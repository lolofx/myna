/**
 * The folder every meeting is exported to as one Markdown file (PRD R5).
 * Maps onto the Rust commands get_export_prefs, set_export_folder and
 * export_meeting_to_folder. Exporting happens automatically once a folder is
 * set; there is no separate on/off switch.
 */
export abstract class ExportFolderPort {
  /** The configured absolute folder, or `null` while exporting is off. */
  abstract get(): Promise<string | null>;
  /**
   * Sets the export folder (created when missing), or turns exporting off
   * with `null`. Rejects when the path is not absolute or cannot be used;
   * the stored setting is untouched on failure.
   */
  abstract set(folder: string | null): Promise<void>;
  /** Exports one meeting into the folder now and resolves with the file path. */
  abstract exportMeeting(meetingId: string): Promise<string>;
}
