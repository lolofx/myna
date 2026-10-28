import { Injectable } from '@angular/core';

import { ExportFolderPort } from '../../core/ports/export-folder.port';
import { invokeCommand } from './ipc';

/**
 * `ExportFolderPort` implementation backed by the Tauri IPC command surface.
 * Reaches the runtime exclusively through `invokeCommand`, so the Tauri
 * boundary stays inside the two allowlisted files.
 */
@Injectable()
export class TauriExportFolderAdapter extends ExportFolderPort {
  override async get(): Promise<string | null> {
    const prefs = await invokeCommand('get_export_prefs', {});
    return prefs.folder;
  }

  override async set(folder: string | null): Promise<void> {
    await invokeCommand('set_export_folder', { folder });
  }

  override async exportMeeting(meetingId: string): Promise<string> {
    return invokeCommand('export_meeting_to_folder', { meetingId });
  }
}
