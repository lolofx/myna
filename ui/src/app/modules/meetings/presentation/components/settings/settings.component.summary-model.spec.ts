import { TestBed } from '@angular/core/testing';

import type { ModelDownloadState } from '../../../application/stores/meetings.store';
import { IDLE_MODEL_DOWNLOAD } from '../../../application/stores/meetings.store';
import type { SummaryModelSlot } from '../../../core/models/models-status.model';
import type { SummaryLanguage } from '../../../core/models/summary-language.model';
import { SettingsComponent } from './settings.component';

const languages: readonly SummaryLanguage[] = [{ code: 'en', label: 'English' }];

describe('SettingsComponent summary model', () => {
  const models: readonly SummaryModelSlot[] = [
    { id: 'qwen2.5-7b', label: 'Qwen2.5 7B', approxSizeMb: 4690, present: false, selected: false },
    { id: 'qwen2.5-3b', label: 'Qwen2.5 3B', approxSizeMb: 2100, present: true, selected: true },
    { id: 'qwen2.5-1.5b', label: 'Qwen2.5 1.5B', approxSizeMb: 1120, present: false, selected: false },
  ];

  const createWithModels = (
    summaryModels: readonly SummaryModelSlot[] = models,
    modelDownload?: ModelDownloadState,
  ) => {
    const fixture = TestBed.createComponent(SettingsComponent);
    fixture.componentRef.setInput('updateConsent', 'unset');
    fixture.componentRef.setInput('summaryLanguages', languages);
    fixture.componentRef.setInput('selectedSummaryLanguage', 'en');
    fixture.componentRef.setInput('summaryModels', summaryModels);
    fixture.componentRef.setInput('modelDownload', modelDownload);
    fixture.detectChanges();
    return fixture;
  };

  const picker = (fixture: ReturnType<typeof createWithModels>): HTMLSelectElement =>
    fixture.nativeElement.querySelector('.model-picker select');

  const choose = (fixture: ReturnType<typeof createWithModels>, id: string): void => {
    const select = picker(fixture);
    select.value = id;
    select.dispatchEvent(new Event('change'));
    fixture.detectChanges();
  };

  it('lists every catalog model with its size and an installed mark', () => {
    const fixture = createWithModels();

    const labels = Array.from(picker(fixture).options).map((option) => option.textContent?.trim());
    expect(labels).toContain('Qwen2.5 3B — 2.1 GB (installed)');
    expect(labels).toContain('Qwen2.5 7B — 4.7 GB');
    expect(labels).toContain('Qwen2.5 1.5B — 1.1 GB');
  });

  it('preselects the model in use and shows it as installed', () => {
    const fixture = createWithModels();

    expect(picker(fixture).value).toBe('qwen2.5-3b');
    expect(fixture.nativeElement.querySelector('.model-installed')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('.download-model')).toBeNull();
  });

  it('shows no selection and offers no download when nothing is chosen', () => {
    const fixture = createWithModels(models.map((model) => ({ ...model, selected: false })));

    expect(picker(fixture).value).toBe('');
    expect(fixture.nativeElement.querySelector('.download-model')).toBeNull();
  });

  it('emits the picked id, and offers a download when that model is not installed', () => {
    const fixture = createWithModels();
    const emitted: string[] = [];
    fixture.componentInstance.summaryModelSelected.subscribe((id) => emitted.push(id));

    choose(fixture, 'qwen2.5-7b');

    expect(emitted).toEqual(['qwen2.5-7b']);
    const download: HTMLButtonElement = fixture.nativeElement.querySelector('.download-model');
    expect(download.textContent).toContain('Download 4.7 GB');
  });

  it('requests the download of the chosen model', () => {
    const fixture = createWithModels();
    const requested: string[] = [];
    fixture.componentInstance.summaryModelDownloadRequested.subscribe((id) => requested.push(id));
    choose(fixture, 'qwen2.5-1.5b');

    const download: HTMLButtonElement = fixture.nativeElement.querySelector('.download-model');
    download.click();

    expect(requested).toEqual(['qwen2.5-1.5b']);
  });

  it('disables the picker and the button while a download runs', () => {
    const fixture = createWithModels(models, { ...IDLE_MODEL_DOWNLOAD, phase: 'running' });
    choose(fixture, 'qwen2.5-7b');

    expect(picker(fixture).disabled).toBe(true);
    const download: HTMLButtonElement = fixture.nativeElement.querySelector('.download-model');
    expect(download.disabled).toBe(true);
    expect(download.textContent).toContain('Downloading');
  });

  it('shows a failed download but stays quiet after a cancellation', () => {
    const failed = createWithModels(models, { ...IDLE_MODEL_DOWNLOAD, phase: 'failed', message: 'Network down' });
    expect(failed.nativeElement.querySelector('.model-error')?.textContent).toContain('Network down');

    const cancelled = createWithModels(models, {
      ...IDLE_MODEL_DOWNLOAD,
      phase: 'failed',
      cancelled: true,
      message: null,
    });
    expect(cancelled.nativeElement.querySelector('.model-error')).toBeNull();
  });

  it('disables the picker until the model catalog has loaded', () => {
    const fixture = createWithModels([]);

    expect(picker(fixture).disabled).toBe(true);
  });
});
