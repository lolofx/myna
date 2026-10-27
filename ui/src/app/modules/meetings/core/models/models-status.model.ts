export interface ModelSlot {
  readonly present: boolean;
  readonly path?: string;
  readonly expectedFiles: readonly string[];
}

/** One selectable summary model and whether it is installed / in use. */
export interface SummaryModelSlot {
  readonly id: string;
  readonly label: string;
  readonly approxSizeMb: number;
  readonly present: boolean;
  /** True for the model a summary would use right now. */
  readonly selected: boolean;
}

export interface ModelsStatus {
  readonly parakeet: ModelSlot;
  /** Whether ANY summary model is installed. Optional for the app: excluded from `allPresent`. */
  readonly qwen: ModelSlot;
  readonly silero: ModelSlot;
  /** Every selectable summary model; absent in fixtures predating the picker. */
  readonly summaryModels?: readonly SummaryModelSlot[];
  /**
   * Presence of the speaker-diarization models (pyannote-3.0 segmentation +
   * NeMo TitaNet embedding). Deliberately EXCLUDED from `allPresent` — a
   * manual-only, optional feature (`./scripts/download-models.sh --only
   * diarization`), never part of the default download. Optional so DTO/
   * fixture literals predating this field keep type-checking.
   */
  readonly diarization?: ModelSlot;
  /** True once the required models (transcription and voice detection) are installed; the summary model never gates it. */
  readonly allPresent: boolean;
}
