import { Injectable } from '@angular/core';
import type { Translation, TranslocoLoader } from '@jsverse/transloco';
import { of } from 'rxjs';

import en from './en.json';
import fr from './fr.json';
import type { UiLanguage } from './supported-languages';

export const TRANSLATIONS: Readonly<Record<UiLanguage, Translation>> = { en, fr };

/** Bundles the JSON files: no HTTP round trip, so the first paint is already translated and the app works offline. */
@Injectable({ providedIn: 'root' })
export class BundledTranslocoLoader implements TranslocoLoader {
  getTranslation(lang: string) {
    return of(TRANSLATIONS[lang as UiLanguage] ?? TRANSLATIONS.en);
  }
}
