import { Injectable, inject, signal } from '@angular/core';

import { ExportFolderPort } from '../../core/ports/export-folder.port';

/**
 * State and actions for the Markdown export folder (PRD R5). Kept apart from
 * `MeetingsFacade`, which is already large, so this small concern does not
 * grow it further.
 *
 * Never optimistic: the folder signal changes only after the write succeeded,
 * so a rejected path leaves the previous setting on screen for another try.
 * Provided at the route injector next to the other facades, like them never
 * `providedIn: 'root'`.
 */
@Injectable()
export class ExportFolderFacade {
  private readonly port = inject(ExportFolderPort);

  private readonly folderState = signal<string | null | undefined>(undefined);
  private readonly errorState = signal<string | null>(null);
  private readonly workingState = signal(false);

  /** `undefined` until loaded, `null` while exporting is off, else the absolute folder. */
  readonly folder = this.folderState.asReadonly();
  /** Message of the last failed load or write; `null` once the next one starts. */
  readonly error = this.errorState.asReadonly();
  /** True while a load or write is in flight. */
  readonly working = this.workingState.asReadonly();

  load(): Promise<void> {
    return this.run(async () => {
      this.folderState.set(await this.port.get());
    });
  }

  /** Sets the folder, or turns exporting off with `null`. */
  choose(folder: string | null): Promise<void> {
    return this.run(async () => {
      await this.port.set(folder);
      // Read back: the backend stores the trimmed, validated form.
      this.folderState.set(await this.port.get());
    });
  }

  private async run(action: () => Promise<void>): Promise<void> {
    this.workingState.set(true);
    this.errorState.set(null);
    try {
      await action();
    } catch (error) {
      this.errorState.set(error instanceof Error ? error.message : String(error));
    } finally {
      this.workingState.set(false);
    }
  }
}
