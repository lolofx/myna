import { AfterViewInit, ChangeDetectionStrategy, Component, ElementRef, HostListener, OnDestroy, inject, input, output, computed } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { I18n } from '../../../../../i18n/i18n.service';

import type { SummaryInstructionsDraft } from '../../../core/models/summary-instructions.model';
import { SummaryInstructionsEditorComponent } from '../summary-instructions-editor/summary-instructions-editor.component';

/**
 * Dumb regenerate-confirmation modal. Reuses the existing draft plumbing:
 * receives the active (meeting, template) draft, passes edits straight
 * through via `draftChanged`, and reports the user's decision via
 * `confirmed`/`cancelled`. The owning pane decides WHEN to show it and
 * what each output means.
 *
 * The host carries the `modal` class so the narrow-sidebar Escape guard
 * (`closeSidebarOnEscape`) defers to it like every other modal; focus moves
 * into the dialog on open and returns to the pane's Regenerate button on
 * close for keyboard users.
 */
@Component({
  selector: 'app-regenerate-instructions-dialog',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [SummaryInstructionsEditorComponent, TranslocoPipe],
  templateUrl: './regenerate-instructions-dialog.component.html',
  styleUrl: './regenerate-instructions-dialog.component.scss',
  host: { class: 'modal' },
})
export class RegenerateInstructionsDialogComponent implements AfterViewInit, OnDestroy {
  private readonly i18n = inject(I18n);

  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  /** The stored (meeting, template) draft; a new object identity means another draft was loaded. */
  readonly draft = input.required<SummaryInstructionsDraft>();
  /** Whitespace-collapsed, truncated preview of the general guidelines; `''` when there are none. */
  readonly guidelinesPreview = input('');
  /** Extra context shown below the editor (e.g. cost/latency note); hidden when empty. */
  readonly hint = input('');
  /** Disables the confirm button while a generation run is already in flight. */
  readonly confirmDisabled = input(false);
  /** Human-readable template name shown in the title/description; hidden when empty. */
  readonly templateLabel = input('');
  /** Title prefix shown before the optional template suffix; falls back to the translated Regenerate copy when empty. */
  readonly dialogTitlePrefix = input('');
  protected readonly titlePrefix = computed(() => this.dialogTitlePrefix() || this.i18n.t('summary.regenerateTitle'));
  /** Description shown below the title; falls back to the translated Regenerate copy when empty. */
  readonly description = input('');
  protected readonly descriptionText = computed(() => this.description() || this.i18n.t('summary.regenerateDescription'));
  /** Confirm button label; falls back to the translated Regenerate copy when empty. */
  readonly confirmLabel = input('');
  protected readonly confirmText = computed(() => this.confirmLabel() || this.i18n.t('summary.regenerate'));
  /** Selector of the trigger to return focus to on close; defaults to the Regenerate button. */
  readonly focusReturnSelector = input('.regenerate-button');

  /** The next draft to persist for this (meeting, template); passthrough from the embedded editor. */
  readonly draftChanged = output<SummaryInstructionsDraft>();
  readonly confirmed = output<void>();
  readonly cancelled = output<void>();

  /** Moves keyboard focus inside the dialog so it is operable without a pointer. */
  ngAfterViewInit(): void {
    const confirm = this.host.nativeElement.querySelector('.confirm');
    if (confirm instanceof HTMLButtonElement) {
      confirm.focus();
    }
  }

  /** Returns focus to the pane's trigger button; a no-op in isolation where none exists. */
  ngOnDestroy(): void {
    if (typeof document === 'undefined') {
      return;
    }
    const trigger = document.querySelector(this.focusReturnSelector());
    if (trigger instanceof HTMLButtonElement) {
      trigger.focus();
    }
  }

  /** Bound on the component's host, so it's only listening while this dialog is actually mounted. */
  @HostListener('document:keydown.escape')
  onEscape(): void {
    this.cancelled.emit();
  }

  onBackdropClick(event: MouseEvent): void {
    if (event.target === event.currentTarget) {
      this.cancelled.emit();
    }
  }

  onBackdropKeydown(event: KeyboardEvent): void {
    if (event.target === event.currentTarget && (event.key === 'Enter' || event.key === ' ')) {
      event.preventDefault();
      this.cancelled.emit();
    }
  }

  onDraftChanged(draft: SummaryInstructionsDraft): void {
    this.draftChanged.emit(draft);
  }

  confirm(): void {
    this.confirmed.emit();
  }

  cancel(): void {
    this.cancelled.emit();
  }
}
