/** Mirrors the Rust `ModelSlot` (`#[serde(rename_all = "camelCase")]`). */
export interface ModelSlotDto {
  readonly present: boolean;
  readonly path: string;
  readonly expectedFiles: readonly string[];
}

/** Mirrors the Rust `SummaryModelSlot` (`#[serde(rename_all = "camelCase")]`). */
export interface SummaryModelSlotDto {
  readonly id: string;
  readonly label: string;
  readonly approxSizeMb: number;
  readonly present: boolean;
  readonly selected: boolean;
}

/** Mirrors the Rust `ModelsStatusDto` (`#[serde(rename_all = "camelCase")]`). */
export interface ModelsStatusDto {
  readonly parakeet: ModelSlotDto;
  readonly qwen: ModelSlotDto;
  readonly silero: ModelSlotDto;
  readonly summaryModels?: readonly SummaryModelSlotDto[];
  readonly selectedSummaryModel?: string | null;
  readonly diarization?: ModelSlotDto;
  readonly allPresent: boolean;
}
