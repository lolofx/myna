import { TestBed } from '@angular/core/testing';

import { TauriExportFolderAdapter } from './tauri-export-folder.adapter';
import { installTauriInternalsStub, uninstallTauriInternalsStub } from './testing/tauri-internals.stub';

describe('TauriExportFolderAdapter', () => {
  let adapter: TauriExportFolderAdapter;

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [TauriExportFolderAdapter] });
    adapter = TestBed.inject(TauriExportFolderAdapter);
  });

  afterEach(() => uninstallTauriInternalsStub());

  it('get() reads the folder from get_export_prefs, null while exporting is off', async () => {
    const received: unknown[] = [];
    installTauriInternalsStub((cmd, args) => {
      received.push([cmd, args]);
      return { folder: received.length === 1 ? '/notes' : null };
    });

    const first = await adapter.get();
    const second = await adapter.get();

    expect(first).toBe('/notes');
    expect(second).toBeNull();
    expect(received[0]).toEqual(['get_export_prefs', {}]);
  });

  it('set() invokes set_export_folder with the folder, or null to turn exporting off', async () => {
    const received: unknown[] = [];
    installTauriInternalsStub((cmd, args) => {
      received.push([cmd, args]);
      return undefined;
    });

    await adapter.set('/notes');
    await adapter.set(null);

    expect(received).toEqual([
      ['set_export_folder', { folder: '/notes' }],
      ['set_export_folder', { folder: null }],
    ]);
  });

  it('exportMeeting() invokes export_meeting_to_folder and resolves with the file path', async () => {
    let receivedCmd: string | undefined;
    let receivedArgs: unknown;
    installTauriInternalsStub((cmd, args) => {
      receivedCmd = cmd;
      receivedArgs = args;
      return '/notes/2026-10-28_21-57_Point.md';
    });

    const path = await adapter.exportMeeting('meeting-1');

    expect(receivedCmd).toBe('export_meeting_to_folder');
    expect(receivedArgs).toEqual({ meetingId: 'meeting-1' });
    expect(path).toBe('/notes/2026-10-28_21-57_Point.md');
  });
});
