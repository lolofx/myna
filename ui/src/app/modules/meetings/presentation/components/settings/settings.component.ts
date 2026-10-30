import { ChangeDetectionStrategy, Component, computed, inject, input, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';

import { I18n } from '../../../../../i18n/i18n.service';
import { SUPPORTED_LANGUAGES, isUiLanguage } from '../../../../../i18n/supported-languages';
import { UiLanguageService } from '../../../../../i18n/ui-language.service';
import type { ModelDownloadState } from '../../../application/stores/meetings.store';
import type { SummaryModelSlot } from '../../../core/models/models-status.model';
import type { SummaryLanguage } from '../../../core/models/summary-language.model';
import type { UpdateCheck, UpdateConsent } from '../../../core/models/update.model';

/**
 * The minimal honest settings surface: only controls whose state is fully
 * wired to the facade render here — update-check consent + manual check
 * (same semantics as About > Updates), the summary output language, the
 * optional summary model (choose and download), and the
 * general summary guidelines (save-on-blur plus an explicit Save button that
 * stays disabled while unchanged).
 * Injects only the interface-language service; the shell owns every `MeetingsFacade` call.
 */
@Component({
  selector: 'app-settings',
  imports: [TranslocoPipe],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './settings.component.html',
  styleUrl: './settings.component.scss',
})
export class SettingsComponent {
  private readonly i18n = inject(I18n);
  private readonly language = inject(UiLanguageService);

  protected readonly interfaceLanguages = SUPPORTED_LANGUAGES;
  protected readonly uiLanguage = this.language.language;

  /** Sourced from `meetingsFacade.updates.consent()` — required: the shell always has a resolved consent value by the time Settings can render. */
  readonly updateConsent = input.required<UpdateConsent>();
  readonly lastCheck = input<UpdateCheck | undefined>(undefined);
  readonly checking = input(false);
  /** True while a recording is in progress; disables "Check now" so a manual check never competes with STT for resources. */
  readonly recording = input(false);
  /** The Rust-owned language list (`SummarizerPort.listLanguages()` via the store) — never a second hardcoded copy. */
  readonly summaryLanguages = input<readonly SummaryLanguage[]>([]);
  readonly selectedSummaryLanguage = input.required<string>();
  /** The selectable summary models with their presence, from `modelsStatus().summaryModels`; empty until the status loads. */
  readonly summaryModels = input<readonly SummaryModelSlot[]>([]);
  /** Download slice of the store, shown inline while a summary model downloads. */
  readonly modelDownload = input<ModelDownloadState | undefined>(undefined);
  /** The persisted general summary guidelines (`facade.summaryGuidelines()` via the shell); `''` until loaded. */
  readonly guidelines = input('');
  /**
   * Effective meetings data root (`facade.storageLocation()?.path` via the
   * shell); `undefined` until `loadStorageLocation` resolves — the template
   * shows a loading placeholder, never a guessed path.
   */
  readonly storagePath = input<string | undefined>(undefined);
  /** True while recording/importing or a storage move is in flight; disables Change/Reset. */
  readonly storageBusy = input(false);
  /** Inline storage error (PATH/BUSY validation or move failure); `null` hides it. */
  readonly storageError = input<string | null>(null);
  /** True right after a move that needs an app restart (`restartRequired`). */
  readonly storageRestartRequired = input(false);
  /** Markdown export folder (`ExportFolderFacade.folder()` via the shell): `undefined` while loading, `null` while exporting is off. */
  readonly exportFolder = input<string | null | undefined>(undefined);
  /** True while the export folder is loading or being saved; disables Choose/Turn off. */
  readonly exportBusy = input(false);
  /** Inline export folder error; `null` hides it. */
  readonly exportError = input<string | null>(null);

  readonly closed = output<void>();
  readonly updateConsentChanged = output<UpdateConsent>();
  readonly checkNowRequested = output<void>();
  readonly summaryLanguageSelected = output<string>();
  /** The id of the summary model picked; the shell persists it, never this component. */
  readonly summaryModelSelected = output<string>();
  /** Download requested for the summary model currently chosen. */
  readonly summaryModelDownloadRequested = output<string>();
  /** The trimmed guidelines text; emitted on blur and on Save — the shell persists it, never this component. */
  readonly guidelinesChanged = output<string>();
  /**
   * Pending Move vs Stay choice (default Move checked); the shell mirrors it
   * so Change…/Reset forward the same flag to `facade.setStorageLocation` /
   * `facade.resetStorageLocation` — this component never calls the facade.
   */
  readonly moveExisting = input(true);
  /** Change… clicked — carries the pending Move vs Stay choice; the shell opens `FileDialogPort.selectDirectory` and calls `facade.setStorageLocation`, never this component. */
  readonly browseStorageRequested = output<boolean>();
  /** Reset-to-default clicked — carries the pending Move vs Stay choice; the shell calls `facade.resetStorageLocation`, never this component. */
  readonly resetStorageRequested = output<boolean>();
  /** Move checkbox toggled — carries the new choice; the shell owns the pending signal, never this component. */
  readonly moveExistingChanged = output<boolean>();
  /** Choose… clicked for the export folder; the shell opens `FileDialogPort.selectDirectory` and saves it, never this component. */
  readonly exportFolderBrowseRequested = output<void>();
  /** Turn off clicked for the export folder; the shell clears it, never this component. */
  readonly exportFolderClearRequested = output<void>();

  protected readonly exportFolderLabel = computed(() => {
    const folder = this.exportFolder();
    if (folder === undefined) {
      return this.i18n.t('common.loading');
    }
    return folder ?? this.i18n.t('settings.export.off');
  });

  protected readonly autoCheckEnabled = computed(() => this.updateConsent() === 'granted');
  protected readonly checkNowDisabled = computed(() => this.checking() || this.recording());
  /** Session-only: no timestamp exists in `UpdateCheck`, so this never claims a precise elapsed time — just whether a check has run since launch. */
  protected readonly lastCheckedLabel = computed(() => (this.lastCheck() === undefined ? this.i18n.t('updates.notThisSession') : this.i18n.t('updates.justNow')));

  /** Pick made in this session; wins over the persisted selection until the status refresh catches up, so a chosen-but-not-yet-installed model stays shown. */
  private readonly pickedModelId = signal<string | null>(null);

  protected readonly chosenModelId = computed(
    () => this.pickedModelId() ?? this.summaryModels().find((model) => model.selected)?.id ?? null,
  );
  protected readonly chosenModel = computed(() => this.summaryModels().find((model) => model.id === this.chosenModelId()));
  protected readonly modelDownloadRunning = computed(() => this.modelDownload()?.phase === 'running');
  /** A failure worth showing: cancellation is the user's own doing, so it stays silent. */
  protected readonly modelDownloadError = computed((): string | null => {
    const download = this.modelDownload();
    return download?.phase === 'failed' && !download.cancelled ? (download.message ?? this.i18n.t('settings.summaryModel.downloadFailed')) : null;
  });

  /** Uncommitted textarea value, bound to the guidelines text it started from — a later input change can never clobber typing, and vice versa. */
  private readonly guidelinesEdit = signal<{ readonly source: string; readonly value: string } | null>(null);

  protected readonly guidelinesValue = computed(() => {
    const current = this.guidelines();
    const edit = this.guidelinesEdit();
    return edit !== null && edit.source === current ? edit.value : current;
  });

  /** Save is offered only for a genuinely different trimmed value; a successful write re-seeds `guidelines` and re-disables the button. */
  protected readonly guidelinesDirty = computed(() => this.guidelinesValue().trim() !== this.guidelines());

  onAutoCheckToggled(event: Event): void {
    const checked = (event.target as HTMLInputElement).checked;
    this.updateConsentChanged.emit(checked ? 'granted' : 'declined');
  }

  checkNow(): void {
    this.checkNowRequested.emit();
  }

  onInterfaceLanguageChanged(event: Event): void {
    const code = (event.target as HTMLSelectElement).value;
    if (isUiLanguage(code)) {
      this.language.set(code);
    }
  }

  onSummaryLanguageChanged(event: Event): void {
    this.summaryLanguageSelected.emit((event.target as HTMLSelectElement).value);
  }

  onSummaryModelChanged(event: Event): void {
    const modelId = (event.target as HTMLSelectElement).value;
    if (modelId !== '') {
      this.pickedModelId.set(modelId);
      this.summaryModelSelected.emit(modelId);
    }
  }

  requestSummaryModelDownload(): void {
    const modelId = this.chosenModelId();
    if (modelId !== null) {
      this.summaryModelDownloadRequested.emit(modelId);
    }
  }

  /** "Qwen2.5 3B (balanced) — 2.1 GB", with a mark once installed. */
  modelOptionLabel(model: SummaryModelSlot): string {
    const installed = model.present ? ` ${this.i18n.t('settings.summaryModel.installedMark')}` : '';
    return `${model.label} — ${this.modelSizeLabel(model)}${installed}`;
  }

  protected modelSizeLabel(model: SummaryModelSlot): string {
    return model.approxSizeMb >= 1000
      ? this.i18n.t('common.sizeGb', { value: (model.approxSizeMb / 1000).toFixed(1) })
      : this.i18n.t('common.sizeMb', { value: model.approxSizeMb });
  }

  onGuidelinesInput(event: Event): void {
    this.guidelinesEdit.set({ source: this.guidelines(), value: (event.target as HTMLTextAreaElement).value });
  }

  onMoveExistingToggled(event: Event): void {
    this.moveExistingChanged.emit((event.target as HTMLInputElement).checked);
  }

  /** Blur saves and the button clicks here; a rejected write leaves `guidelines` untouched, so the text and the enabled Save persist for a retry. */
  saveGuidelines(): void {
    const text = this.guidelinesValue().trim();
    if (text !== this.guidelines()) {
      this.guidelinesChanged.emit(text);
    }
  }
}
