import { Injectable } from '@angular/core';

import { ExportFolderPort } from '../../core/ports/export-folder.port';

/** In-memory ExportFolderPort implementation for specs. */
@Injectable()
export class InMemoryExportFolderFake extends ExportFolderPort {
  private folder: string | null = null;
  /** Meeting ids passed to `exportMeeting`, in call order. */
  readonly exported: string[] = [];
  /** When set, `set` rejects with this message (simulates an unusable folder). */
  failSetWith: string | null = null;

  override async get(): Promise<string | null> {
    return this.folder;
  }

  override async set(folder: string | null): Promise<void> {
    if (this.failSetWith !== null) {
      throw new Error(this.failSetWith);
    }
    this.folder = folder;
  }

  override async exportMeeting(meetingId: string): Promise<string> {
    if (this.folder === null) {
      throw new Error('no export folder is configured');
    }
    this.exported.push(meetingId);
    return `${this.folder}/${meetingId}.md`;
  }

  /** Test helper: replace the stored folder. */
  seed(folder: string | null): void {
    this.folder = folder;
  }
}
