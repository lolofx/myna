import en from './en.json';

type Params = Record<string, unknown>;
type Translator = (key: string, params?: Params) => string;

let translator: Translator | null = null;

/** Registered by `I18n` so plain functions (no injection context) follow the active interface language. */
export const registerTranslator = (next: Translator | null): void => {
  translator = next;
};

const lookup = (catalogue: unknown, key: string): string | undefined => {
  let node: unknown = catalogue;
  for (const part of key.split('.')) {
    if (node === null || typeof node !== 'object') {
      return undefined;
    }
    node = (node as Record<string, unknown>)[part];
  }
  return typeof node === 'string' ? node : undefined;
};

/** English fallback for code that runs before (or without) the translation service, e.g. isolated unit tests. */
const fallback: Translator = (key, params) => {
  const text = lookup(en, key) ?? key;
  return text.replace(/\{\{\s*(\w+)\s*\}\}/g, (_match, name: string) => String(params?.[name] ?? ''));
};

/** Translates from plain TypeScript functions; components and services should prefer `I18n.t`. */
export const tr: Translator = (key, params) => (translator ?? fallback)(key, params);
