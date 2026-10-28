import { TestBed } from '@angular/core/testing';

import { ExportFolderPort } from '../../core/ports/export-folder.port';
import { InMemoryExportFolderFake } from '../testing/in-memory-export-folder.fake';
import { ExportFolderFacade } from './export-folder.facade';

describe('ExportFolderFacade', () => {
  let fake: InMemoryExportFolderFake;
  let facade: ExportFolderFacade;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [ExportFolderFacade, { provide: ExportFolderPort, useClass: InMemoryExportFolderFake }],
    });
    fake = TestBed.inject(ExportFolderPort) as InMemoryExportFolderFake;
    facade = TestBed.inject(ExportFolderFacade);
  });

  it('starts unloaded: the folder is undefined, not "off"', () => {
    expect(facade.folder()).toBeUndefined();
    expect(facade.error()).toBeNull();
    expect(facade.working()).toBe(false);
  });

  it('load() reports the stored folder, or null while exporting is off', async () => {
    await facade.load();
    expect(facade.folder()).toBeNull();

    fake.seed('/notes');
    await facade.load();
    expect(facade.folder()).toBe('/notes');
  });

  it('choose() saves the folder and shows what the backend stored', async () => {
    await facade.choose('/notes');

    expect(facade.folder()).toBe('/notes');
    expect(facade.error()).toBeNull();
    expect(facade.working()).toBe(false);
  });

  it('choose(null) turns exporting off', async () => {
    fake.seed('/notes');
    await facade.load();

    await facade.choose(null);

    expect(facade.folder()).toBeNull();
  });

  it('is never optimistic: a rejected folder keeps the previous one and surfaces the message', async () => {
    fake.seed('/notes');
    await facade.load();
    fake.failSetWith = 'the export folder must be an absolute path';

    await facade.choose('relative');

    expect(facade.folder()).toBe('/notes');
    expect(facade.error()).toBe('the export folder must be an absolute path');
    expect(facade.working()).toBe(false);
  });

  it('clears the previous error when the next attempt starts', async () => {
    fake.failSetWith = 'unusable';
    await facade.choose('relative');
    expect(facade.error()).toBe('unusable');

    fake.failSetWith = null;
    await facade.choose('/notes');

    expect(facade.error()).toBeNull();
    expect(facade.folder()).toBe('/notes');
  });

  it('reports working while a write is in flight', async () => {
    const seen: boolean[] = [];
    const pending = facade.choose('/notes');
    seen.push(facade.working());
    await pending;
    seen.push(facade.working());

    expect(seen).toEqual([true, false]);
  });
});
