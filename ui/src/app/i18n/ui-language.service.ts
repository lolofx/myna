import { DOCUMENT } from '@angular/common';
import { Injectable, inject, signal } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';

import {
  DEFAULT_LANGUAGE,
  UI_LANGUAGE_PREFERENCE_KEY,
  isUiLanguage,
  languageFromLocale,
  type UiLanguage,
} from './supported-languages';

/**
 * Owns the interface language: the stored choice wins, then the browser
 * locale, then English. Independent of the summary output language, which is
 * a separate setting.
 */
@Injectable({ providedIn: 'root' })
export class UiLanguageService {
  private readonly transloco = inject(TranslocoService);
  private readonly document = inject(DOCUMENT);

  private readonly current = signal<UiLanguage>(DEFAULT_LANGUAGE);

  readonly language = this.current.asReadonly();

  /** Applies the stored or detected language; called once at startup. */
  init(): void {
    this.apply(this.readStored() ?? languageFromLocale(this.document.defaultView?.navigator.language));
  }

  set(language: UiLanguage): void {
    this.apply(language);
    try {
      this.document.defaultView?.localStorage.setItem(UI_LANGUAGE_PREFERENCE_KEY, language);
    } catch {
      // Storage unavailable: the choice still applies for this session.
    }
  }

  private apply(language: UiLanguage): void {
    // Load before switching so `translate()` never sees an empty catalogue.
    this.transloco.load(language).subscribe();
    this.transloco.setActiveLang(language);
    this.current.set(language);
    this.document.documentElement.lang = language;
  }

  private readStored(): UiLanguage | null {
    try {
      const stored = this.document.defaultView?.localStorage.getItem(UI_LANGUAGE_PREFERENCE_KEY);
      return isUiLanguage(stored) ? stored : null;
    } catch {
      return null;
    }
  }
}
