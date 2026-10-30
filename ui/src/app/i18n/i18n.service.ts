import { Injectable, inject } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';

import { registerTranslator } from './tr';
import { UiLanguageService } from './ui-language.service';

type Params = Record<string, unknown>;

/**
 * Translation for TypeScript code. Reading the language signal first makes a
 * `computed()` that calls `t()` re-evaluate when the interface language
 * changes. Templates use the `transloco` pipe instead.
 */
@Injectable({ providedIn: 'root' })
export class I18n {
  private readonly transloco = inject(TranslocoService);
  private readonly language = inject(UiLanguageService);

  constructor() {
    registerTranslator((key, params) => this.t(key, params));
  }

  t(key: string, params?: Params): string {
    this.language.language();
    return this.transloco.translate(key, params);
  }
}
