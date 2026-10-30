import { provideAppInitializer } from '@angular/core';

import { provideI18n } from './app/i18n/i18n.providers';
import { UI_LANGUAGE_PREFERENCE_KEY } from './app/i18n/supported-languages';

// Every spec renders with the real English catalogue, so text assertions stay meaningful.
export default [
  // A language picked in one spec must not leak into the next one through localStorage.
  provideAppInitializer(() => localStorage.removeItem(UI_LANGUAGE_PREFERENCE_KEY)),
  provideI18n(),
];
