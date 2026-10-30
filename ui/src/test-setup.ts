import '@angular/compiler';

import { TestBed } from '@angular/core/testing';
import { provideAppInitializer } from '@angular/core';

import { provideI18n } from './app/i18n/i18n.providers';
import { UI_LANGUAGE_PREFERENCE_KEY } from './app/i18n/supported-languages';

// Every spec renders with the real English catalogue, so text assertions stay meaningful.
beforeEach(() => {
  TestBed.configureTestingModule({
    providers: [
      // A language picked in one spec must not leak into the next one through localStorage.
      provideAppInitializer(() => localStorage.removeItem(UI_LANGUAGE_PREFERENCE_KEY)),
      provideI18n(),
    ],
  });
});

// jsdom has no Tauri IPC: an event stream opened after a spec removed its stub rejects asynchronously and
// would be reported as an unhandled error attributed to whichever spec happens to be running.
window.addEventListener(
  'error',
  (event) => {
    if (String(event.error?.message ?? event.message).includes('transformCallback')) {
      event.preventDefault();
      event.stopImmediatePropagation();
    }
  },
  true,
);
