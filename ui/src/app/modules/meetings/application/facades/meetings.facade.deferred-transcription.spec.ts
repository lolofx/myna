import { TestBed } from '@angular/core/testing';
import { vi } from 'vitest';

import type { Meeting } from '../../core/models/meeting.model';
import { toMeetingId } from '../../core/models/meeting.model';
import { AppInfoPort } from '../../core/ports/app-info.port';
import { AudioImportPort } from '../../core/ports/audio-import.port';
import { FileDialogPort } from '../../core/ports/file-dialog.port';
import { MeetingRepositoryPort } from '../../core/ports/meeting-repository.port';
import { ModelsStatusPort } from '../../core/ports/models-status.port';
import { PreferencesPort } from '../../core/ports/preferences.port';
import { RecorderPort } from '../../core/ports/recorder.port';
import { SummarizerPort } from '../../core/ports/summarizer.port';
import { TemplateRepositoryPort } from '../../core/ports/template-repository.port';
import { TranscriberPort } from '../../core/ports/transcriber.port';
import { provideMeetings } from '../../meetings.providers';
import { InMemoryAppInfoFake } from '../testing/in-memory-app-info.fake';
import { InMemoryAudioImportFake } from '../testing/in-memory-audio-import.fake';
import { InMemoryFileDialogFake } from '../testing/in-memory-file-dialog.fake';
import { InMemoryMeetingRepositoryFake } from '../testing/in-memory-meeting-repository.fake';
import { InMemoryModelsStatusFake } from '../testing/in-memory-models-status.fake';
import { InMemoryPreferencesFake } from '../testing/in-memory-preferences.fake';
import { InMemoryRecorderFake } from '../testing/in-memory-recorder.fake';
import { InMemorySummarizerFake } from '../testing/in-memory-summarizer.fake';
import { InMemoryTemplateRepositoryFake } from '../testing/in-memory-template-repository.fake';
import { InMemoryTranscriberFake } from '../testing/in-memory-transcriber.fake';
import { MeetingsFacade } from './meetings.facade';

/** The id the recorder fake gives the first meeting it starts. */
const RECORDED_ID = toMeetingId('meeting-1');

const STOPPED: Meeting = {
  id: RECORDED_ID,
  title: 'Just recorded',
  createdAt: new Date(),
  durationSec: 120,
  summaries: [],
  archived: false,
  hasAudio: true,
  hasSystemTrack: true,
  droppedAudioChunks: 0,
};

/**
 * ADR 0017: a capture-only recording stops with `transcriptionPending`, and
 * the facade then transcribes it through the re-transcribe path. A live
 * recording is left alone.
 */
describe('MeetingsFacade deferred transcription', () => {
  let facade: MeetingsFacade;
  let recorder: InMemoryRecorderFake;
  let audioImport: InMemoryAudioImportFake;

  beforeEach(() => {
    vi.useFakeTimers();
    TestBed.configureTestingModule({
      providers: [
        provideMeetings(),
        { provide: MeetingRepositoryPort, useClass: InMemoryMeetingRepositoryFake },
        { provide: RecorderPort, useClass: InMemoryRecorderFake },
        { provide: SummarizerPort, useClass: InMemorySummarizerFake },
        { provide: TranscriberPort, useClass: InMemoryTranscriberFake },
        { provide: TemplateRepositoryPort, useClass: InMemoryTemplateRepositoryFake },
        { provide: ModelsStatusPort, useClass: InMemoryModelsStatusFake },
        { provide: FileDialogPort, useClass: InMemoryFileDialogFake },
        { provide: PreferencesPort, useClass: InMemoryPreferencesFake },
        { provide: AppInfoPort, useClass: InMemoryAppInfoFake },
        { provide: AudioImportPort, useClass: InMemoryAudioImportFake },
      ],
    });
    facade = TestBed.inject(MeetingsFacade);
    recorder = TestBed.inject(RecorderPort) as InMemoryRecorderFake;
    audioImport = TestBed.inject(AudioImportPort) as InMemoryAudioImportFake;
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  const recordAndStop = async (stopOverrides: Partial<Meeting>): Promise<void> => {
    recorder.seedStopResult(stopOverrides);
    await facade.startRecording('Just recorded');
    await facade.stopRecording();
  };

  it('transcribes a capture-only recording right after it stops', async () => {
    const transcribed: Meeting = { ...STOPPED, durationSec: 121 };
    audioImport.seed(transcribed);

    await recordAndStop({ transcriptionPending: true });

    expect(audioImport.getLastRetranscribedId()).toBe(RECORDED_ID);
    expect(audioImport.getLastRetranscribedPath()).toBeUndefined();
    expect(facade.selectedMeeting()?.durationSec).toBe(121);
    expect(facade.selectedMeeting()?.transcriptionPending).toBeUndefined();
  });

  it('does not re-transcribe a live recording', async () => {
    await recordAndStop({});

    expect(audioImport.getLastRetranscribedId()).toBeUndefined();
    expect(facade.selectedMeeting()?.id).toBe(RECORDED_ID);
  });

  it('keeps the saved meeting and reports the error when the transcription fails', async () => {
    audioImport.seedError(new Error('model missing'));

    await recordAndStop({ transcriptionPending: true });

    expect(facade.error()).toBeDefined();
    expect(facade.selectedMeeting()?.id).toBe(RECORDED_ID);
    expect(facade.importing()).toBe(false);
  });
});
