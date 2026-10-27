//! Safe WASAPI loopback capture for Windows.
//!
//! This library is the Windows counterpart to `myna-coreaudio-tap`'s role on
//! macOS: every `unsafe` COM/WASAPI call in Myna's Windows system-audio path
//! lives here, behind the safe [`LoopbackCapture`] surface and the status
//! helpers below, so `myna-audio` keeps the workspace-wide
//! `unsafe_code = "forbid"` intact. (This package already carried that local
//! `allow` for the `myna-wasapi-probe` binary's throwaway probe; the surface
//! here reuses that probe's proven open sequence — default render
//! `eRender`/`eConsole` endpoint, shared-mode `LOOPBACK` initialize,
//! `Start`/`Stop` on the audio client, unaligned-safe `SubFormat` reads.)
//!
//! The non-Windows stub at the bottom keeps this library compiling on other
//! hosts with identical signatures; it always reports unavailable.

#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "windows")]
use std::sync::mpsc;
#[cfg(target_os = "windows")]
use std::sync::{Arc, Mutex};
#[cfg(target_os = "windows")]
use std::thread::{self, JoinHandle};
#[cfg(target_os = "windows")]
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use windows::core::HRESULT;
#[cfg(target_os = "windows")]
use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
#[cfg(target_os = "windows")]
use windows::Win32::Media::Audio::{eConsole, eRender};
#[cfg(target_os = "windows")]
use windows::Win32::Media::Audio::{
    IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_E_DEVICE_INVALIDATED, AUDCLNT_E_SERVICE_NOT_RUNNING,
    AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX, WAVE_FORMAT_PCM,
};
#[cfg(target_os = "windows")]
use windows::Win32::Media::KernelStreaming::{KSDATAFORMAT_SUBTYPE_PCM, WAVE_FORMAT_EXTENSIBLE};
#[cfg(target_os = "windows")]
use windows::Win32::Media::Multimedia::{KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, WAVE_FORMAT_IEEE_FLOAT};
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
    COINIT_MULTITHREADED,
};

/// How long [`LoopbackCapture::start`] waits for the capture thread to report
/// its mix format before giving up.
#[cfg(target_os = "windows")]
const STARTUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Engine buffer requested at initialize time, in 100ns units
/// (10_000_000 == 1s) — the probe's proven value.
#[cfg(target_os = "windows")]
const ENGINE_BUFFER_100NS: i64 = 10_000_000;

/// Idle sleep between packet-size polls when the engine has no packet ready.
#[cfg(target_os = "windows")]
const POLL_IDLE_SLEEP: Duration = Duration::from_millis(10);

/// How often the capture thread checks that the default render endpoint is
/// still the one it opened. Shared-mode loopback keeps capturing the old
/// device after the default changes, so without this check a newly plugged
/// headset would be missed until the old device disappears.
#[cfg(target_os = "windows")]
const DEVICE_CHECK_INTERVAL: Duration = Duration::from_secs(1);

/// Reopen attempts after the endpoint changed or was invalidated, spaced by
/// [`REOPEN_RETRY_DELAY`] (about 5s in total) so a device that is still
/// settling does not end the capture.
#[cfg(target_os = "windows")]
const REOPEN_ATTEMPTS: u32 = 20;

#[cfg(target_os = "windows")]
const REOPEN_RETRY_DELAY: Duration = Duration::from_millis(250);

/// Peak-meter level counted as genuine render activity rather than meter
/// floor noise.
#[cfg(target_os = "windows")]
const RENDERING_PEAK_THRESHOLD: f32 = 1e-6;

#[cfg(target_os = "windows")]
struct ComGuard;

#[cfg(target_os = "windows")]
impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum SampleLayout {
    Float32,
    Pcm16,
    Pcm32,
}

#[cfg(target_os = "windows")]
fn sample_layout(mix_format: *const WAVEFORMATEX) -> Option<SampleLayout> {
    // SAFETY: `mix_format` is the caller-owned `GetMixFormat` buffer, valid
    // for the duration of this call; only read, never written.
    let mix: WAVEFORMATEX = unsafe { *mix_format };
    let tag = u32::from(mix.wFormatTag);
    if tag == WAVE_FORMAT_IEEE_FLOAT && mix.wBitsPerSample == 32 {
        return Some(SampleLayout::Float32);
    }
    if tag == WAVE_FORMAT_PCM {
        return match mix.wBitsPerSample {
            16 => Some(SampleLayout::Pcm16),
            32 => Some(SampleLayout::Pcm32),
            _ => None,
        };
    }
    if tag == WAVE_FORMAT_EXTENSIBLE {
        // WAVEFORMATEXTENSIBLE is `packed(1)`, so `SubFormat` may be
        // unaligned; read it without creating an unaligned reference.
        let sub_format = unsafe {
            std::ptr::addr_of!(
                (*mix_format.cast::<windows::Win32::Media::Audio::WAVEFORMATEXTENSIBLE>())
                    .SubFormat
            )
            .read_unaligned()
        };
        if sub_format == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT && mix.wBitsPerSample == 32 {
            return Some(SampleLayout::Float32);
        }
        if sub_format == KSDATAFORMAT_SUBTYPE_PCM {
            return match mix.wBitsPerSample {
                16 => Some(SampleLayout::Pcm16),
                32 => Some(SampleLayout::Pcm32),
                _ => None,
            };
        }
    }
    None
}

/// Reports whether a default render endpoint exists right now. Each call
/// brackets its own COM init/uninit so callers never manage apartments.
#[cfg(target_os = "windows")]
pub fn default_render_available() -> bool {
    if unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok() }.is_err() {
        return false;
    }
    let _com = ComGuard;
    let endpoint: windows::core::Result<()> = (|| {
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        let _device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole)? };
        Ok(())
    })();
    endpoint.is_ok()
}

/// Current default-render peak-meter level, or 0.0 when no endpoint meters
/// (no device, COM failure). Never fails: meter queries run on the
/// supervisor's worker thread, where an error return would stall teardown.
#[cfg(target_os = "windows")]
pub fn loopback_render_peak() -> f32 {
    if unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok() }.is_err() {
        return 0.0;
    }
    let _com = ComGuard;
    let peak: windows::core::Result<f32> = (|| {
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole)? };
        let meter: IAudioMeterInformation = unsafe { device.Activate(CLSCTX_ALL, None)? };
        unsafe { meter.GetPeakValue() }
    })();
    peak.unwrap_or(0.0)
}

/// Whether the default render endpoint currently meters above silence.
#[cfg(target_os = "windows")]
pub fn is_loopback_rendering() -> bool {
    loopback_render_peak() > RENDERING_PEAK_THRESHOLD
}

/// A running WASAPI loopback capture of the default render mix.
///
/// Owns its polling thread; [`LoopbackCapture::stop`] (or `Drop`)
/// signals it and joins, so no thread outlives its handle.
#[cfg(target_os = "windows")]
pub struct LoopbackCapture {
    stop: Arc<AtomicBool>,
    failure: Arc<Mutex<Option<String>>>,
    thread: Option<JoinHandle<()>>,
}

#[cfg(target_os = "windows")]
impl LoopbackCapture {
    /// Starts looping back the default render mix, invoking `on_frames`
    /// with native interleaved f32 frames plus their channel count per
    /// engine packet (zeros for silent packets, so downstream clocks
    /// never gap) on the owned thread. Returns the handle plus the mix
    /// format's actual sample rate and channel count, discovered — never
    /// assumed — at open time.
    pub fn start(
        on_frames: impl FnMut(&[f32], u16) + Send + 'static,
    ) -> Result<(Self, u32, u16), String> {
        let stop = Arc::new(AtomicBool::new(false));
        let failure = Arc::new(Mutex::new(None));
        let (report_tx, report_rx) = mpsc::channel();
        let thread_stop = Arc::clone(&stop);
        let thread_failure = Arc::clone(&failure);
        let thread = thread::Builder::new()
            .name("wasapi-loopback".to_string())
            .spawn(move || {
                run_loopback(thread_stop, thread_failure, Box::new(on_frames), report_tx);
            })
            .map_err(|error| format!("failed to spawn WASAPI loopback thread: {error}"))?;
        match report_rx.recv_timeout(STARTUP_TIMEOUT) {
            Ok(Ok((rate, channels))) => Ok((
                Self {
                    stop,
                    failure,
                    thread: Some(thread),
                },
                rate,
                channels,
            )),
            Ok(Err(message)) => {
                let _ = thread.join();
                Err(message)
            }
            Err(_) => {
                stop.store(true, Ordering::Release);
                let _ = thread.join();
                Err("WASAPI loopback startup timed out".to_string())
            }
        }
    }

    /// Why the capture thread ended on its own, if it did: a WASAPI error
    /// it could not recover from, or a replacement device it could not
    /// follow (for example one with a different sample rate). `None` while
    /// the capture is running or after a normal [`LoopbackCapture::stop`].
    pub fn failure(&self) -> Option<String> {
        self.failure.lock().ok().and_then(|failure| failure.clone())
    }

    /// Stops the capture and joins its thread.
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for LoopbackCapture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(target_os = "windows")]
struct OpenedLoopback {
    client: IAudioClient,
    capture: IAudioCaptureClient,
    device_id: String,
    sample_rate: u32,
    channels: u16,
    layout: SampleLayout,
}

/// Owns the `GetMixFormat` buffer and frees it on drop, so it stays valid
/// through `Initialize` and no error path can leak or double-free it.
#[cfg(target_os = "windows")]
struct MixFormat(*mut WAVEFORMATEX);

#[cfg(target_os = "windows")]
impl Drop for MixFormat {
    fn drop(&mut self) {
        // SAFETY: the pointer came from `GetMixFormat`, which allocates it
        // with `CoTaskMemAlloc`, and nothing else frees it.
        unsafe { CoTaskMemFree(Some(self.0 as _)) };
    }
}

/// Id of the endpoint, as the enumerator reports it, or `None` when it
/// cannot be read.
#[cfg(target_os = "windows")]
fn endpoint_id(device: &windows::Win32::Media::Audio::IMMDevice) -> Option<String> {
    let id = unsafe { device.GetId() }.ok()?;
    // SAFETY: `GetId` returns a NUL-terminated string allocated with
    // `CoTaskMemAlloc`; read it, then free it.
    let text = unsafe { id.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(id.0 as _)) };
    text
}

/// Id of the current default render endpoint, or `None` when there is none.
/// The calling thread must already have COM initialized.
#[cfg(target_os = "windows")]
fn default_render_id() -> Option<String> {
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }.ok()?;
    let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole) }.ok()?;
    endpoint_id(&device)
}

#[cfg(target_os = "windows")]
fn open_loopback() -> Result<OpenedLoopback, String> {
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
            .map_err(|error| format!("no audio device enumerator: {error}"))?;
    let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole) }
        .map_err(|error| format!("no default output device for loopback capture: {error}"))?;
    let device_id = endpoint_id(&device).unwrap_or_default();
    let client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None) }
        .map_err(|error| format!("WASAPI client activation failed: {error}"))?;
    let mix_format = MixFormat(
        unsafe { client.GetMixFormat() }
            .map_err(|error| format!("WASAPI mix format query failed: {error}"))?,
    );
    // SAFETY: `GetMixFormat` buffer, owned by `mix_format` and valid until
    // it drops at the end of this function, after `Initialize` has used it.
    let mix: WAVEFORMATEX = unsafe { *mix_format.0 };
    let layout = sample_layout(mix_format.0);
    // `WAVEFORMATEX` is packed, so its fields cannot be borrowed; copy the
    // scalars out before formatting.
    let tag = mix.wFormatTag;
    let bits = mix.wBitsPerSample;
    let layout = layout
        .ok_or_else(|| format!("unsupported loopback mix format (tag {tag}, {bits} bits)"))?;
    if mix.nChannels == 0 {
        return Err("loopback mix format reports zero channels".to_string());
    }
    unsafe {
        client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            ENGINE_BUFFER_100NS,
            0,
            mix_format.0,
            None,
        )
    }
    .map_err(|error| format!("WASAPI loopback initialize failed: {error}"))?;
    let capture: IAudioCaptureClient = unsafe { client.GetService() }
        .map_err(|error| format!("WASAPI capture service failed: {error}"))?;
    Ok(OpenedLoopback {
        client,
        capture,
        device_id,
        sample_rate: mix.nSamplesPerSec,
        channels: mix.nChannels,
        layout,
    })
}

/// Whether a WASAPI error means the endpoint went away or the audio service
/// restarted, so reopening on the current default is worth trying. Any other
/// error ends the capture.
#[cfg(target_os = "windows")]
fn is_recoverable(code: HRESULT) -> bool {
    code == AUDCLNT_E_DEVICE_INVALIDATED || code == AUDCLNT_E_SERVICE_NOT_RUNNING
}

#[cfg(target_os = "windows")]
fn push_packet_as_f32(
    data: *const u8,
    frames: u32,
    channels: u16,
    layout: SampleLayout,
    silent: bool,
    out: &mut Vec<f32>,
) {
    let count = frames as usize * usize::from(channels);
    out.clear();
    out.reserve(count);
    if silent {
        out.resize(count, 0.0);
        return;
    }
    // SAFETY: `data` is the capture client's buffer holding `count` samples
    // in `layout`, valid until the matching `ReleaseBuffer`; only read.
    match layout {
        SampleLayout::Float32 => {
            out.extend(unsafe { std::slice::from_raw_parts(data.cast::<f32>(), count) });
        }
        SampleLayout::Pcm16 => {
            out.extend(
                unsafe { std::slice::from_raw_parts(data.cast::<i16>(), count) }
                    .iter()
                    .map(|sample| f32::from(*sample) / 32768.0),
            );
        }
        SampleLayout::Pcm32 => {
            out.extend(
                unsafe { std::slice::from_raw_parts(data.cast::<i32>(), count) }
                    .iter()
                    .map(|sample| *sample as f32 / 2_147_483_648.0),
            );
        }
    }
}

/// Why [`pump`] returned.
#[cfg(target_os = "windows")]
enum PumpEnd {
    Stopped,
    /// The default endpoint changed, or the device or audio service went
    /// away: reopen on whatever is the default now.
    Reopen,
    Failed(String),
}

/// Reads engine packets and forwards them until stopped, the default
/// endpoint changes, or WASAPI reports an error.
#[cfg(target_os = "windows")]
fn pump(
    opened: &OpenedLoopback,
    stop: &AtomicBool,
    on_frames: &mut dyn FnMut(&[f32], u16),
) -> PumpEnd {
    let mut scratch = Vec::<f32>::new();
    let mut last_device_check = Instant::now();
    loop {
        if stop.load(Ordering::Relaxed) {
            return PumpEnd::Stopped;
        }
        if last_device_check.elapsed() >= DEVICE_CHECK_INTERVAL {
            last_device_check = Instant::now();
            if !opened.device_id.is_empty()
                && default_render_id().is_some_and(|id| id != opened.device_id)
            {
                return PumpEnd::Reopen;
            }
        }
        let available = match unsafe { opened.capture.GetNextPacketSize() } {
            Ok(frames) => frames,
            Err(error) if is_recoverable(error.code()) => return PumpEnd::Reopen,
            Err(error) => {
                return PumpEnd::Failed(format!("WASAPI packet size query failed: {error}"))
            }
        };
        if available == 0 {
            thread::sleep(POLL_IDLE_SLEEP);
            continue;
        }
        let mut data: *mut u8 = std::ptr::null_mut();
        let mut frames = 0;
        let mut flags = 0;
        match unsafe {
            opened
                .capture
                .GetBuffer(&mut data, &mut frames, &mut flags, None, None)
        } {
            Ok(()) => {}
            Err(error) if is_recoverable(error.code()) => return PumpEnd::Reopen,
            Err(error) => return PumpEnd::Failed(format!("WASAPI buffer read failed: {error}")),
        }
        let silent = (flags & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32)) != 0;
        push_packet_as_f32(
            data,
            frames,
            opened.channels,
            opened.layout,
            silent,
            &mut scratch,
        );
        if !scratch.is_empty() {
            on_frames(&scratch, opened.channels);
        }
        if let Err(error) = unsafe { opened.capture.ReleaseBuffer(frames) } {
            return if is_recoverable(error.code()) {
                PumpEnd::Reopen
            } else {
                PumpEnd::Failed(format!("WASAPI buffer release failed: {error}"))
            };
        }
    }
}

/// Reopens loopback capture on the current default endpoint, retrying while
/// the device settles. `Ok(None)` means a stop was requested while waiting.
/// The sample rate must match the one reported to the caller at start,
/// because downstream buffers are sized from it; a device with another rate
/// ends the capture rather than feeding mismatched audio.
#[cfg(target_os = "windows")]
fn reopen(stop: &AtomicBool, sample_rate: u32) -> Result<Option<OpenedLoopback>, String> {
    let mut last_error = String::new();
    for _ in 0..REOPEN_ATTEMPTS {
        if stop.load(Ordering::Relaxed) {
            return Ok(None);
        }
        match open_loopback() {
            Ok(next) if next.sample_rate != sample_rate => {
                return Err(format!(
                    "default output changed sample rate from {sample_rate} Hz to {} Hz",
                    next.sample_rate
                ));
            }
            Ok(next) => match unsafe { next.client.Start() } {
                Ok(()) => return Ok(Some(next)),
                Err(error) => last_error = format!("WASAPI capture restart failed: {error}"),
            },
            Err(message) => last_error = message,
        }
        thread::sleep(REOPEN_RETRY_DELAY);
    }
    Err(format!("could not reopen loopback capture: {last_error}"))
}

#[cfg(target_os = "windows")]
fn record_failure(failure: &Mutex<Option<String>>, message: String) {
    // Debug-only: release builds keep device and error detail out of logs.
    #[cfg(debug_assertions)]
    eprintln!("myna-wasapi-probe: loopback capture ended: {message}");
    if let Ok(mut slot) = failure.lock() {
        *slot = Some(message);
    }
}

#[cfg(target_os = "windows")]
fn run_loopback(
    stop: Arc<AtomicBool>,
    failure: Arc<Mutex<Option<String>>>,
    mut on_frames: Box<dyn FnMut(&[f32], u16) + Send>,
    report: mpsc::Sender<Result<(u32, u16), String>>,
) {
    if let Err(error) = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok() } {
        let _ = report.send(Err(format!("WASAPI COM init failed: {error}")));
        return;
    }
    let _com = ComGuard;
    let mut opened = match open_loopback() {
        Ok(opened) => opened,
        Err(message) => {
            let _ = report.send(Err(message));
            return;
        }
    };
    if unsafe { opened.client.Start() }.is_err() {
        let _ = report.send(Err("WASAPI capture start failed".to_string()));
        return;
    }
    if report
        .send(Ok((opened.sample_rate, opened.channels)))
        .is_err()
    {
        let _ = unsafe { opened.client.Stop() };
        return;
    }
    let sample_rate = opened.sample_rate;
    loop {
        let end = pump(&opened, &stop, &mut *on_frames);
        let _ = unsafe { opened.client.Stop() };
        match end {
            PumpEnd::Stopped => return,
            PumpEnd::Failed(message) => {
                record_failure(&failure, message);
                return;
            }
            PumpEnd::Reopen => match reopen(&stop, sample_rate) {
                Ok(Some(next)) => opened = next,
                Ok(None) => return,
                Err(message) => {
                    record_failure(&failure, message);
                    return;
                }
            },
        }
    }
}

/// Non-Windows stub: identical signatures, always unavailable.
#[cfg(not(target_os = "windows"))]
pub fn default_render_available() -> bool {
    false
}

/// Non-Windows stub: identical signatures, always silence.
#[cfg(not(target_os = "windows"))]
pub fn loopback_render_peak() -> f32 {
    0.0
}

/// Non-Windows stub: identical signatures, never rendering.
#[cfg(not(target_os = "windows"))]
pub fn is_loopback_rendering() -> bool {
    false
}

/// Non-Windows stub: identical signatures, [`LoopbackCapture::start`]
/// always fails.
#[cfg(not(target_os = "windows"))]
pub struct LoopbackCapture;

#[cfg(not(target_os = "windows"))]
impl LoopbackCapture {
    /// Non-Windows stub: identical signatures, always fails.
    pub fn start(
        _on_frames: impl FnMut(&[f32], u16) + Send + 'static,
    ) -> Result<(Self, u32, u16), String> {
        Err("WASAPI loopback capture is Windows-only".to_string())
    }

    /// Non-Windows stub: identical signatures, no thread to join.
    pub fn stop(self) {}
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use windows::Win32::Media::Audio::AUDCLNT_E_BUFFER_ERROR;

    #[test]
    fn device_loss_and_service_restart_are_recoverable() {
        assert!(is_recoverable(AUDCLNT_E_DEVICE_INVALIDATED));
        assert!(is_recoverable(AUDCLNT_E_SERVICE_NOT_RUNNING));
    }

    #[test]
    fn other_errors_end_the_capture() {
        assert!(!is_recoverable(AUDCLNT_E_BUFFER_ERROR));
        assert!(!is_recoverable(HRESULT(0)));
    }
}
