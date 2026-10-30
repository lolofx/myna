import { ChangeDetectionStrategy, Component, computed, input, output, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { I18n } from '../../../../../i18n/i18n.service';

import type { MeetingsErrorCode } from '../../../core/models/recording-state.model';


/**
 * Shared error surface: a human message plus retry and dismiss affordances,
 * keyed off the stable error code. Pure presentation — the owning page decides
 * what "retry" means and clears the shared error slot on "dismiss".
 */
@Component({
  selector: 'app-error-state',
  imports: [TranslocoPipe],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './error-state.component.html',
  styleUrl: './error-state.component.scss',
})
export class ErrorStateComponent {
  private readonly i18n = inject(I18n);

  readonly code = input.required<MeetingsErrorCode>();
  readonly retryClicked = output<void>();
  /** "Close": the user wants the banner gone without retrying anything. */
  readonly dismissClicked = output<void>();

  /**
   * Human-facing copy per stable error code (`errors.<CODE>`). Never derived
   * from the raw error message string — the message is backend-authored free
   * text and must not be pattern-matched to decide what the UI shows.
   */
  readonly message = computed(() => {
    const key = `errors.${this.code()}`;
    const text = this.i18n.t(key);
    return text === key ? this.i18n.t('errors.UNKNOWN') : text;
  });
}
