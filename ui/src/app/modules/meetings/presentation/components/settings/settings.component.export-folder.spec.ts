import { TestBed } from '@angular/core/testing';

import { SettingsComponent } from './settings.component';

describe('SettingsComponent export folder', () => {
  const createFixture = () => {
    const fixture = TestBed.createComponent(SettingsComponent);
    fixture.componentRef.setInput('updateConsent', 'unset');
    fixture.componentRef.setInput('selectedSummaryLanguage', 'en');
    fixture.detectChanges();
    return fixture;
  };

  const text = (fixture: ReturnType<typeof createFixture>, selector: string): string =>
    fixture.nativeElement.querySelector(selector)?.textContent ?? '';

  it('shows a loading placeholder until the folder has loaded', () => {
    const fixture = createFixture();

    expect(text(fixture, '.export-folder-value')).toContain('Loading…');
  });

  it('says exporting is off when no folder is set, and offers no Turn off button', () => {
    const fixture = createFixture();
    fixture.componentRef.setInput('exportFolder', null);
    fixture.detectChanges();

    expect(text(fixture, '.export-folder-value')).toContain('Off');
    expect(fixture.nativeElement.querySelector('.clear-export-folder')).toBeNull();
    expect(fixture.nativeElement.querySelector('.change-export-folder')).not.toBeNull();
  });

  it('shows the export folder and a Turn off button once one is set', () => {
    const fixture = createFixture();
    fixture.componentRef.setInput('exportFolder', '/notes');
    fixture.detectChanges();

    expect(text(fixture, '.export-folder-value')).toContain('/notes');
    expect(fixture.nativeElement.querySelector('.clear-export-folder')).not.toBeNull();
  });

  it('emits exportFolderBrowseRequested on Choose… and exportFolderClearRequested on Turn off', () => {
    const fixture = createFixture();
    fixture.componentRef.setInput('exportFolder', '/notes');
    fixture.detectChanges();
    const browsed: void[] = [];
    const cleared: void[] = [];
    fixture.componentInstance.exportFolderBrowseRequested.subscribe(() => browsed.push(undefined));
    fixture.componentInstance.exportFolderClearRequested.subscribe(() => cleared.push(undefined));

    fixture.nativeElement.querySelector('.change-export-folder').click();
    fixture.nativeElement.querySelector('.clear-export-folder').click();

    expect(browsed.length).toBe(1);
    expect(cleared.length).toBe(1);
  });

  it('disables Choose and Turn off while busy', () => {
    const fixture = createFixture();
    fixture.componentRef.setInput('exportFolder', '/notes');
    fixture.componentRef.setInput('exportBusy', true);
    fixture.detectChanges();

    expect((fixture.nativeElement.querySelector('.change-export-folder') as HTMLButtonElement).disabled).toBe(true);
    expect((fixture.nativeElement.querySelector('.clear-export-folder') as HTMLButtonElement).disabled).toBe(true);
  });

  it('shows the inline export error, hidden when null', () => {
    const clean = createFixture();
    expect(clean.nativeElement.querySelector('.export-folder-error')).toBeNull();

    const failed = createFixture();
    failed.componentRef.setInput('exportError', 'the export folder must be an absolute path');
    failed.detectChanges();
    expect(text(failed, '.export-folder-error')).toContain('absolute path');
  });
});
