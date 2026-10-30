import { inject, isDevMode, provideAppInitializer, type EnvironmentProviders, type Provider } from '@angular/core';
import { provideTransloco } from '@jsverse/transloco';

import { I18n } from './i18n.service';
import { DEFAULT_LANGUAGE, SUPPORTED_LANGUAGES } from './supported-languages';
import { BundledTranslocoLoader } from './transloco.loader';
import { UiLanguageService } from './ui-language.service';

export const provideI18n = (): (Provider | EnvironmentProviders)[] => [
  provideTransloco({
    config: {
      availableLangs: [...SUPPORTED_LANGUAGES],
      defaultLang: DEFAULT_LANGUAGE,
      fallbackLang: DEFAULT_LANGUAGE,
      reRenderOnLangChange: true,
      prodMode: !isDevMode(),
    },
    loader: BundledTranslocoLoader,
  }),
  provideAppInitializer(() => {
    inject(UiLanguageService).init();
    // Instantiated here so plain functions can translate through `tr()` from the first render.
    inject(I18n);
  }),
];
