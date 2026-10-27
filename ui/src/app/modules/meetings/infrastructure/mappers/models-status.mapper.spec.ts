import { mapModelSlotDtoToDomain, mapModelsStatusDtoToDomain } from './models-status.mapper';

describe('mapModelSlotDtoToDomain', () => {
  it('maps every field verbatim', () => {
    const slot = mapModelSlotDtoToDomain({
      present: true,
      path: '/models/parakeet-tdt-0.6b-v3-int8',
      expectedFiles: ['encoder.int8.onnx', 'tokens.txt'],
    });

    expect(slot).toEqual({
      present: true,
      path: '/models/parakeet-tdt-0.6b-v3-int8',
      expectedFiles: ['encoder.int8.onnx', 'tokens.txt'],
    });
  });
});

describe('mapModelsStatusDtoToDomain', () => {
  it('maps all three slots plus allPresent', () => {
    const dto = {
      parakeet: { present: true, path: '/models/parakeet', expectedFiles: ['a'] },
      qwen: { present: false, path: '/models/qwen', expectedFiles: ['b'] },
      silero: { present: true, path: '/models/silero', expectedFiles: ['c'] },
      allPresent: false,
    };

    expect(mapModelsStatusDtoToDomain(dto)).toEqual({
      parakeet: { present: true, path: '/models/parakeet', expectedFiles: ['a'] },
      qwen: { present: false, path: '/models/qwen', expectedFiles: ['b'] },
      silero: { present: true, path: '/models/silero', expectedFiles: ['c'] },
      allPresent: false,
    });
  });
});

describe('mapModelsStatusDtoToDomain summary models', () => {
  const base = {
    parakeet: { present: true, path: '/models/parakeet', expectedFiles: ['a'] },
    qwen: { present: true, path: '/models/qwen', expectedFiles: ['b'] },
    silero: { present: true, path: '/models/silero', expectedFiles: ['c'] },
    allPresent: true,
  };

  it('carries the summary model catalog through when the backend sends it', () => {
    const summaryModels = [
      { id: 'qwen2.5-3b', label: 'Qwen2.5 3B', approxSizeMb: 2100, present: true, selected: true },
      { id: 'qwen2.5-7b', label: 'Qwen2.5 7B', approxSizeMb: 4690, present: false, selected: false },
    ];

    const status = mapModelsStatusDtoToDomain({ ...base, summaryModels, selectedSummaryModel: 'qwen2.5-3b' });

    expect(status.summaryModels).toEqual(summaryModels);
  });

  it('omits the catalog for a backend that predates it', () => {
    const status = mapModelsStatusDtoToDomain(base);

    expect(status.summaryModels).toBeUndefined();
  });
});

