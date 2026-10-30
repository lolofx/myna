import { ChangeDetectionStrategy, Component, computed, effect, input, output, signal, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { I18n } from '../../../../../i18n/i18n.service';

import { IDLE_MODEL_DOWNLOAD, type MeetingsErrorInfo, type ModelDownloadState } from '../../../application/stores/meetings.store';
import type { ModelSlot, ModelsStatus } from '../../../core/models/models-status.model';
import { ErrorStateComponent } from '../error-state/error-state.component';

/**
 * Rotating reassurance lines shown while models download. Deliberately
 * indeterminate — no fake %, MB, or ETA figures, funny tone only.
 */
export const DOWNLOAD_WHIMSY_KEYS: readonly string[] = [
  'onboarding.whimsy.0',
  'onboarding.whimsy.1',
  'onboarding.whimsy.2',
  'onboarding.whimsy.3',
  'onboarding.whimsy.4',
  'onboarding.whimsy.5',
];

/** How long each reassurance line stays up before rotating to the next. */
export const WHIMSY_ROTATE_MS = 30000;

interface ModelGroup {
  readonly label: string;
  readonly slot: ModelSlot;
}

/**
 * Blocking state rendered inside the detail pane whenever
 * `modelsStatus()?.allPresent` is false — never a separate route. Lists every
 * expected model artefact by its real filename, sourced live from
 * `ModelsStatus`, never hardcoded. Copy is deliberately aimed at a
 * non-technical user: no shell commands, paths, or terminal instructions are
 * ever shown. Pure presentation: the owning page wires `recheckRequested` to
 * `facade.checkModels()`.
 */
@Component({
  selector: 'app-onboarding-panel',
  imports: [ErrorStateComponent, TranslocoPipe],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './onboarding-panel.component.html',
  styleUrl: './onboarding-panel.component.scss',
})
export class OnboardingPanelComponent {
  private readonly i18n = inject(I18n);

  readonly status = input<ModelsStatus | undefined>(undefined);
  readonly error = input<MeetingsErrorInfo | undefined>(undefined);
  readonly modelDownload = input<ModelDownloadState | undefined>(undefined);

  readonly recheckRequested = output<void>();
  /** "Close" on the embedded error banner — the owning page clears the shared error slot. */
  readonly dismissRequested = output<void>();
  readonly downloadRequested = output<void>();
  readonly downloadCancelRequested = output<void>();

  /** {@link modelDownload} normalized to a concrete state — the store's resting value before the first download run of the session. */
  protected readonly download = computed((): ModelDownloadState => this.modelDownload() ?? IDLE_MODEL_DOWNLOAD);
  protected readonly isDownloading = computed(() => this.download().phase === 'running');
  protected readonly downloadFailed = computed(() => this.download().phase === 'failed');

  /** Position in {@link DOWNLOAD_WHIMSY_KEYS}; advanced by the rotation timer below. */
  protected readonly whimsyIndex = signal(0);
  protected readonly whimsy = computed(
    (): string => this.i18n.t(DOWNLOAD_WHIMSY_KEYS[this.whimsyIndex() % DOWNLOAD_WHIMSY_KEYS.length] ?? ''),
  );

  constructor() {
    // 30 s reassurance rotation: runs only while downloading, restarts from
    // the first line on every new run, interval cleared on cleanup.
    effect((onCleanup) => {
      if (!this.isDownloading()) {
        return;
      }
      this.whimsyIndex.set(0);
      const timer = setInterval(() => {
        this.whimsyIndex.update((i) => i + 1);
      }, WHIMSY_ROTATE_MS);
      onCleanup(() => clearInterval(timer));
    });
  }

  protected readonly groups = computed((): readonly ModelGroup[] => {
    const current = this.status();
    if (!current) {
      return [];
    }
    return [
      { label: this.i18n.t('onboarding.groups.transcription'), slot: current.parakeet },
      { label: this.i18n.t('onboarding.groups.voiceDetection'), slot: current.silero },
    ];
  });

  recheck(): void {
    this.recheckRequested.emit();
  }

  dismiss(): void {
    this.dismissRequested.emit();
  }

  startDownload(): void {
    this.downloadRequested.emit();
  }

  cancelDownload(): void {
    this.downloadCancelRequested.emit();
  }
}
