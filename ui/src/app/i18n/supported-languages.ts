/** Interface languages the app ships translations for; each has a matching `<code>.json` here. */
export const SUPPORTED_LANGUAGES = ['en', 'fr'] as const;

export type UiLanguage = (typeof SUPPORTED_LANGUAGES)[number];

export const DEFAULT_LANGUAGE: UiLanguage = 'en';

/** localStorage key the interface language is persisted under. */
export const UI_LANGUAGE_PREFERENCE_KEY = 'ui.language';

export const isUiLanguage = (value: string | null | undefined): value is UiLanguage =>
  value != null && (SUPPORTED_LANGUAGES as readonly string[]).includes(value);

/** First supported language matching a browser locale such as `fr-FR`, else the default. */
export const languageFromLocale = (locale: string | null | undefined): UiLanguage => {
  const base = locale?.toLowerCase().split('-')[0] ?? '';
  return isUiLanguage(base) ? base : DEFAULT_LANGUAGE;
};
