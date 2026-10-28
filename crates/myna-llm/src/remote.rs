//! Remote summary backend: an Ollama server run by the user on their own
//! network (ADR 0019).
//!
//! This is the one place where meeting text leaves the machine, so it is
//! deliberately narrow:
//!
//! - **Private networks only.** The configured address must resolve to a
//!   loopback, private, link-local or shared (CGNAT/VPN) address; anything
//!   else is refused *before* a connection is attempted, so a typo can never
//!   send a transcript to the public internet.
//! - **Plain `http://` only.** There is no TLS stack in this crate, and none
//!   is added for a LAN feature. `https://` is rejected with a clear message.
//! - **Text only.** Only the rendered summary prompt is sent, never audio.
//! - **No new dependency.** A small HTTP/1.1 client over [`std::net`] keeps
//!   the supply chain unchanged and lets a cancel close the socket at once,
//!   which makes Ollama stop generating.
//!
//! The wire format is Ollama's native `/api/chat`, not the OpenAI-compatible
//! route, because only the native route can pin `num_ctx` (Ollama's default
//! context silently truncates long prompts) and switch off the "thinking"
//! phase of Qwen3-class models with `think: false`.

use std::io::{self, BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::engine::{context_budget, fits_in_context, split_transcript, MAX_REDUCE_DEPTH};
use crate::error::LlmError;
use crate::template::{RenderContext, Template};

/// Port Ollama listens on unless configured otherwise.
pub const DEFAULT_OLLAMA_PORT: u16 = 11434;

/// How long to wait for the TCP connection itself.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Granularity of the read loop: how often a blocked read re-checks the
/// cancel flag and the idle deadline.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Default longest silence tolerated while waiting for a generation. Generous
/// on purpose: a server that just woke from sleep has to load the model and
/// prefill the whole prompt before the first token appears.
const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

/// Longest silence tolerated while listing models.
const LIST_IDLE_TIMEOUT: Duration = Duration::from_secs(15);

/// Largest error body read from the server.
const MAX_ERROR_BODY: usize = 64 * 1024;

/// Largest model-list body read from the server.
const MAX_LIST_BODY: usize = 4 * 1024 * 1024;

/// Largest single header or NDJSON line accepted.
const MAX_LINE: usize = 4 * 1024 * 1024;

/// Characters per token assumed when sizing prompts without a tokenizer.
/// Deliberately pessimistic for French and English text, so the estimate
/// errs towards splitting a long meeting rather than overflowing `num_ctx`.
const CHARS_PER_TOKEN: usize = 3;

fn remote(message: impl Into<String>) -> LlmError {
    LlmError::Remote(message.into())
}

/// A validated `host:port` for the summary server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    host: String,
    port: u16,
}

impl Endpoint {
    /// Parses `http://host[:port]` (a bare `host[:port]` is accepted too).
    /// Any path, query or fragment is ignored.
    pub fn parse(raw: &str) -> Result<Self, LlmError> {
        let trimmed = raw.trim();
        let lower = trimmed.to_ascii_lowercase();
        let rest = if lower.starts_with("https://") {
            return Err(remote(
                "https is not supported: use an http:// address on your local network",
            ));
        } else if lower.starts_with("http://") {
            &trimmed["http://".len()..]
        } else {
            trimmed
        };

        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        if authority.is_empty() {
            return Err(remote("the server address is empty"));
        }
        if authority.contains('@') {
            return Err(remote(
                "credentials in the server address are not supported",
            ));
        }

        let (host, port) = if let Some(bracketed) = authority.strip_prefix('[') {
            let (host, after) = bracketed
                .split_once(']')
                .ok_or_else(|| remote("malformed IPv6 address in the server address"))?;
            let port = match after.strip_prefix(':') {
                Some(port) => parse_port(port)?,
                None if after.is_empty() => DEFAULT_OLLAMA_PORT,
                None => return Err(remote("malformed IPv6 address in the server address")),
            };
            (host, port)
        } else {
            match authority.rsplit_once(':') {
                Some((host, port)) if !host.contains(':') => (host, parse_port(port)?),
                Some(_) => {
                    return Err(remote("an IPv6 address must be written between [ and ]"));
                }
                None => (authority, DEFAULT_OLLAMA_PORT),
            }
        };
        if host.is_empty() {
            return Err(remote("the server address has no host"));
        }
        Ok(Self {
            host: host.to_string(),
            port,
        })
    }

    /// Resolves the host and refuses anything that is not on a private
    /// network. Returns the addresses to try, in resolution order.
    fn socket_addrs(&self) -> Result<Vec<SocketAddr>, LlmError> {
        let addrs: Vec<SocketAddr> = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|err| remote(format!("cannot resolve '{}': {err}", self.host)))?
            .collect();
        if addrs.is_empty() {
            return Err(remote(format!("'{}' resolves to no address", self.host)));
        }
        if let Some(public) = addrs.iter().find(|addr| !is_private_ip(addr.ip())) {
            return Err(remote(format!(
                "refusing to send meeting text to a non-local address ({}): \
                 only loopback, private, link-local and VPN addresses are allowed",
                public.ip()
            )));
        }
        Ok(addrs)
    }

    /// `Host` header value: IPv6 literals need brackets.
    fn host_header(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

fn parse_port(raw: &str) -> Result<u16, LlmError> {
    raw.parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| remote(format!("'{raw}' is not a valid port")))
}

/// True for addresses that cannot be on the public internet: loopback,
/// RFC 1918 private ranges, link-local, unique-local IPv6, and the
/// `100.64.0.0/10` shared range used by CGNAT and mesh VPNs.
pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_private_v4(mapped);
            }
            let first = v6.segments()[0];
            v6.is_loopback() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80
        }
    }
}

fn is_private_v4(ip: Ipv4Addr) -> bool {
    let shared = ip.octets()[0] == 100 && (ip.octets()[1] & 0xc0) == 64;
    ip.is_loopback() || ip.is_private() || ip.is_link_local() || shared
}

/// How the response body is delimited.
enum Framing {
    Chunked,
    Length(usize),
    UntilClose,
    Done,
}

/// One HTTP/1.1 exchange over a blocking socket that wakes up every
/// [`POLL_INTERVAL`] to honour cancellation and the idle deadline.
struct Connection<'a> {
    reader: BufReader<TcpStream>,
    cancel: &'a AtomicBool,
    idle_timeout: Duration,
    last_activity: Instant,
    framing: Framing,
}

impl<'a> Connection<'a> {
    /// Connects, sends `request` and reads the response head. A non-200
    /// answer becomes an error carrying the server's own message.
    fn send(
        endpoint: &Endpoint,
        method: &str,
        path: &str,
        body: Option<&str>,
        cancel: &'a AtomicBool,
        idle_timeout: Duration,
    ) -> Result<Self, LlmError> {
        let addrs = endpoint.socket_addrs()?;
        let mut last_error = None;
        let stream =
            addrs.iter().find_map(
                |addr| match TcpStream::connect_timeout(addr, CONNECT_TIMEOUT) {
                    Ok(stream) => Some(stream),
                    Err(err) => {
                        last_error = Some(err);
                        None
                    }
                },
            );
        let Some(stream) = stream else {
            let reason = last_error.map_or_else(|| "unknown error".to_string(), |e| e.to_string());
            return Err(remote(format!(
                "cannot reach {}:{} ({reason}); is the server running and awake?",
                endpoint.host, endpoint.port
            )));
        };
        let io_err = |err: io::Error| remote(format!("connection error: {err}"));
        stream
            .set_read_timeout(Some(POLL_INTERVAL))
            .map_err(io_err)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .map_err(io_err)?;
        // Best effort: small requests should leave at once.
        let _ = stream.set_nodelay(true);

        let payload = body.unwrap_or("");
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\n\
             Connection: close\r\n",
            endpoint.host_header()
        );
        if body.is_some() {
            request.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                payload.len()
            ));
        }
        request.push_str("\r\n");
        request.push_str(payload);
        let mut writer = &stream;
        writer.write_all(request.as_bytes()).map_err(io_err)?;
        writer.flush().map_err(io_err)?;

        let mut conn = Self {
            reader: BufReader::new(stream),
            cancel,
            idle_timeout,
            last_activity: Instant::now(),
            framing: Framing::UntilClose,
        };
        let status = conn.read_head()?;
        if status != 200 {
            let body = conn.read_all(MAX_ERROR_BODY).unwrap_or_default();
            return Err(http_error(status, &body));
        }
        Ok(conn)
    }

    /// Waits for buffered bytes. An empty slice means the peer closed the
    /// connection. Errors on cancellation or when the idle deadline passes.
    fn fill(&mut self) -> Result<&[u8], LlmError> {
        loop {
            if self.cancel.load(Ordering::SeqCst) {
                return Err(LlmError::Cancelled);
            }
            match self.reader.fill_buf() {
                Ok(buf) => {
                    if !buf.is_empty() {
                        self.last_activity = Instant::now();
                    }
                    break;
                }
                Err(err)
                    if matches!(
                        err.kind(),
                        io::ErrorKind::WouldBlock
                            | io::ErrorKind::TimedOut
                            | io::ErrorKind::Interrupted
                    ) =>
                {
                    if self.last_activity.elapsed() > self.idle_timeout {
                        return Err(remote(format!(
                            "no answer for {} s; the server may be loading the model or asleep",
                            self.idle_timeout.as_secs()
                        )));
                    }
                }
                Err(err) => return Err(remote(format!("connection lost: {err}"))),
            }
        }
        Ok(self.reader.buffer())
    }

    /// Reads one line without its terminator; `None` at end of stream.
    fn read_line(&mut self) -> Result<Option<String>, LlmError> {
        let mut line = Vec::new();
        loop {
            let buf = self.fill()?;
            if buf.is_empty() {
                if line.is_empty() {
                    return Ok(None);
                }
                break;
            }
            if let Some(end) = buf.iter().position(|&b| b == b'\n') {
                line.extend_from_slice(&buf[..end]);
                self.reader.consume(end + 1);
                break;
            }
            let taken = buf.len();
            line.extend_from_slice(buf);
            self.reader.consume(taken);
            if line.len() > MAX_LINE {
                return Err(remote("the server sent an oversized line"));
            }
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        Ok(Some(String::from_utf8_lossy(&line).into_owned()))
    }

    /// Reads exactly `count` bytes.
    fn read_exact_bytes(&mut self, count: usize) -> Result<Vec<u8>, LlmError> {
        let mut out = Vec::with_capacity(count.min(MAX_LINE));
        while out.len() < count {
            let buf = self.fill()?;
            if buf.is_empty() {
                return Err(remote("the server closed the connection mid-answer"));
            }
            let take = buf.len().min(count - out.len());
            out.extend_from_slice(&buf[..take]);
            self.reader.consume(take);
        }
        Ok(out)
    }

    /// Parses the status line and headers, records how the body is framed,
    /// and returns the status code.
    fn read_head(&mut self) -> Result<u16, LlmError> {
        let eof = || remote("the server closed the connection without answering");
        let status_line = self.read_line()?.ok_or_else(eof)?;
        let status = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse::<u16>().ok())
            .ok_or_else(|| remote(format!("malformed HTTP answer: '{status_line}'")))?;

        let mut chunked = false;
        let mut length = None;
        loop {
            let line = self.read_line()?.ok_or_else(eof)?;
            if line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                match name.trim().to_ascii_lowercase().as_str() {
                    "transfer-encoding" => {
                        chunked = value.to_ascii_lowercase().contains("chunked");
                    }
                    "content-length" => length = value.trim().parse::<usize>().ok(),
                    _ => {}
                }
            }
        }
        self.framing = match (chunked, length) {
            (true, _) => Framing::Chunked,
            (false, Some(length)) => Framing::Length(length),
            (false, None) => Framing::UntilClose,
        };
        Ok(status)
    }

    /// Next decoded piece of the body, or `None` once it is complete.
    fn next_piece(&mut self) -> Result<Option<Vec<u8>>, LlmError> {
        match self.framing {
            Framing::Done => Ok(None),
            Framing::Chunked => {
                let size_line = self
                    .read_line()?
                    .ok_or_else(|| remote("the server closed the connection mid-answer"))?;
                let size_text = size_line.split(';').next().unwrap_or("").trim();
                let size = usize::from_str_radix(size_text, 16)
                    .map_err(|_| remote(format!("malformed chunk size '{size_text}'")))?;
                if size == 0 {
                    // Trailers, if any, up to the blank line.
                    while let Some(line) = self.read_line()? {
                        if line.is_empty() {
                            break;
                        }
                    }
                    self.framing = Framing::Done;
                    return Ok(None);
                }
                let data = self.read_exact_bytes(size)?;
                // The CRLF that closes the chunk.
                self.read_line()?;
                Ok(Some(data))
            }
            Framing::Length(remaining) => {
                if remaining == 0 {
                    self.framing = Framing::Done;
                    return Ok(None);
                }
                let buf = self.fill()?;
                if buf.is_empty() {
                    return Err(remote("the server closed the connection mid-answer"));
                }
                let take = buf.len().min(remaining);
                let piece = buf[..take].to_vec();
                self.reader.consume(take);
                self.framing = Framing::Length(remaining - take);
                Ok(Some(piece))
            }
            Framing::UntilClose => {
                let buf = self.fill()?;
                if buf.is_empty() {
                    self.framing = Framing::Done;
                    return Ok(None);
                }
                let piece = buf.to_vec();
                let taken = piece.len();
                self.reader.consume(taken);
                Ok(Some(piece))
            }
        }
    }

    /// Reads the rest of the body, failing if it exceeds `limit` bytes.
    fn read_all(&mut self, limit: usize) -> Result<Vec<u8>, LlmError> {
        let mut out = Vec::new();
        while let Some(piece) = self.next_piece()? {
            out.extend_from_slice(&piece);
            if out.len() > limit {
                return Err(remote("the server answer is too large"));
            }
        }
        Ok(out)
    }
}

/// Turns a non-200 answer into an error, preferring Ollama's own
/// `{"error": "..."}` message.
fn http_error(status: u16, body: &[u8]) -> LlmError {
    let message = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| {
            let text = String::from_utf8_lossy(body);
            text.trim().chars().take(200).collect()
        });
    if message.is_empty() {
        remote(format!("the server answered HTTP {status}"))
    } else {
        remote(format!("the server answered HTTP {status}: {message}"))
    }
}

/// Lists the models installed on the server (`GET /api/tags`), sorted by
/// name. Doubles as the "test connection" check: it fails with a readable
/// message when the address is wrong, public, unreachable or not Ollama.
pub fn list_remote_models(base_url: &str) -> Result<Vec<String>, LlmError> {
    let endpoint = Endpoint::parse(base_url)?;
    let never = AtomicBool::new(false);
    let mut conn = Connection::send(
        &endpoint,
        "GET",
        "/api/tags",
        None,
        &never,
        LIST_IDLE_TIMEOUT,
    )?;
    let body = conn.read_all(MAX_LIST_BODY)?;
    let value: Value = serde_json::from_slice(&body).map_err(|err| {
        remote(format!(
            "unexpected answer from the server (is it an Ollama server?): {err}"
        ))
    })?;
    let mut names: Vec<String> = value
        .get("models")
        .and_then(Value::as_array)
        .map(|models| {
            models
                .iter()
                .filter_map(|model| model.get("name").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    Ok(names)
}

/// Generation parameters for the remote backend. Defaults mirror the local
/// engine (`max_tokens`, `temperature`, `top_p`, `seed`); `num_ctx` is
/// smaller than the local 32K because a server model holds its context in
/// the server's memory, and a 16K window covers about an hour of speech.
#[derive(Debug, Clone)]
pub struct RemoteOptions {
    pub num_ctx: u32,
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub seed: u32,
    /// Longest silence tolerated before giving up on the server.
    pub idle_timeout: Duration,
}

impl Default for RemoteOptions {
    fn default() -> Self {
        Self {
            num_ctx: 16_384,
            max_tokens: 1024,
            temperature: 0.3,
            top_p: 0.9,
            seed: 1234,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
        }
    }
}

/// Summarizes transcripts on a remote Ollama server.
pub struct RemoteSummarizer {
    endpoint: Endpoint,
    model: String,
    options: RemoteOptions,
}

impl RemoteSummarizer {
    /// Validates the address and model name. Does not touch the network.
    pub fn new(base_url: &str, model: &str) -> Result<Self, LlmError> {
        let model = model.trim();
        if model.is_empty() {
            return Err(remote("no model is selected for the summary server"));
        }
        Ok(Self {
            endpoint: Endpoint::parse(base_url)?,
            model: model.to_string(),
            options: RemoteOptions::default(),
        })
    }

    #[must_use]
    pub fn with_options(mut self, options: RemoteOptions) -> Self {
        self.options = options;
        self
    }

    /// Renders `template` against `ctx` and summarizes it on the server,
    /// splitting a transcript that cannot fit `num_ctx` into chunks that
    /// are summarized separately and then combined (map-reduce), exactly as
    /// the local engine does. Prompt size is estimated from the character
    /// count, since the server's tokenizer is not available here.
    ///
    /// Only the final pass streams through `on_token`. `cancel` is observed
    /// every [`POLL_INTERVAL`]; cancelling closes the connection, which
    /// stops generation on the server.
    pub fn summarize_transcript(
        &self,
        template: &Template,
        ctx: &RenderContext,
        cancel: &AtomicBool,
        mut on_token: impl FnMut(&str),
    ) -> Result<String, LlmError> {
        self.summarize_at_depth(template, ctx, cancel, &mut on_token, 0)
    }

    fn summarize_at_depth(
        &self,
        template: &Template,
        ctx: &RenderContext,
        cancel: &AtomicBool,
        on_token: &mut dyn FnMut(&str),
        depth: u32,
    ) -> Result<String, LlmError> {
        if cancel.load(Ordering::SeqCst) {
            return Err(LlmError::Cancelled);
        }
        let opts = &self.options;
        let full_prompt = template.render(ctx);
        if fits_in_context(estimate_tokens(&full_prompt), opts.max_tokens, opts.num_ctx) {
            return self.generate(&full_prompt, cancel, on_token);
        }
        if depth >= MAX_REDUCE_DEPTH {
            return Err(LlmError::PromptTooLong(format!(
                "the transcript still does not fit the server context ({} tokens) after \
                 {depth} rounds of map-reduce summarization; giving up",
                opts.num_ctx
            )));
        }

        let wrapper = RenderContext {
            transcript: String::new(),
            ..ctx.clone()
        };
        let overhead = estimate_tokens(&template.render(&wrapper));
        let chunk_budget = context_budget(opts.max_tokens, opts.num_ctx).saturating_sub(overhead);
        if chunk_budget == 0 {
            return Err(LlmError::PromptTooLong(
                "the template and instructions alone fill the server context".to_string(),
            ));
        }
        let chunks = split_transcript(&ctx.transcript, chunk_budget, estimate_tokens);

        let mut partial = Vec::with_capacity(chunks.len());
        for chunk in &chunks {
            if cancel.load(Ordering::SeqCst) {
                return Err(LlmError::Cancelled);
            }
            let chunk_ctx = RenderContext {
                transcript: chunk.clone(),
                ..ctx.clone()
            };
            partial.push(self.generate(&template.render(&chunk_ctx), cancel, &mut |_| {})?);
        }
        let reduced = RenderContext {
            transcript: partial.join("\n\n"),
            ..ctx.clone()
        };
        self.summarize_at_depth(template, &reduced, cancel, on_token, depth + 1)
    }

    /// One `/api/chat` round trip, streamed.
    fn generate(
        &self,
        prompt: &str,
        cancel: &AtomicBool,
        on_token: &mut dyn FnMut(&str),
    ) -> Result<String, LlmError> {
        let opts = &self.options;
        let body = json!({
            "model": self.model,
            "stream": true,
            // Qwen3-class models otherwise spend minutes "thinking" first.
            "think": false,
            "messages": [{ "role": "user", "content": prompt }],
            "options": {
                "num_ctx": opts.num_ctx,
                "num_predict": opts.max_tokens,
                "temperature": opts.temperature,
                "top_p": opts.top_p,
                "seed": opts.seed,
            },
        })
        .to_string();
        let mut conn = Connection::send(
            &self.endpoint,
            "POST",
            "/api/chat",
            Some(&body),
            cancel,
            opts.idle_timeout,
        )?;

        let mut pending: Vec<u8> = Vec::new();
        let mut text = String::new();
        let mut finished = false;
        while !finished {
            let Some(piece) = conn.next_piece()? else {
                break;
            };
            pending.extend_from_slice(&piece);
            while let Some(end) = pending.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = pending.drain(..=end).collect();
                if apply_stream_line(&line, &mut text, on_token)? {
                    finished = true;
                    break;
                }
            }
        }
        if !finished && !pending.iter().all(u8::is_ascii_whitespace) {
            finished = apply_stream_line(&pending, &mut text, on_token)?;
        }
        if !finished {
            return Err(remote(
                "the server ended the stream before the end of the answer",
            ));
        }
        Ok(strip_think_blocks(&text).trim().to_string())
    }
}

/// Applies one NDJSON line of an `/api/chat` stream to `text`. Returns true
/// once the server reports `done`.
fn apply_stream_line(
    line: &[u8],
    text: &mut String,
    on_token: &mut dyn FnMut(&str),
) -> Result<bool, LlmError> {
    if line.iter().all(u8::is_ascii_whitespace) {
        return Ok(false);
    }
    let value: Value = serde_json::from_slice(line)
        .map_err(|err| remote(format!("unreadable answer from the server: {err}")))?;
    if let Some(message) = value.get("error").and_then(Value::as_str) {
        return Err(remote(message));
    }
    if let Some(piece) = value
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .filter(|piece| !piece.is_empty())
    {
        text.push_str(piece);
        on_token(piece);
    }
    Ok(value.get("done").and_then(Value::as_bool) == Some(true))
}

/// Pessimistic token estimate, see [`CHARS_PER_TOKEN`].
fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(CHARS_PER_TOKEN)
}

/// Removes complete `<think>...</think>` blocks that a server may still
/// emit despite `think: false`.
fn strip_think_blocks(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<think>") {
        let Some(len) = rest[start..].find("</think>") else {
            break;
        };
        out.push_str(&rest[..start]);
        rest = &rest[start + len + "</think>".len()..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::atomic::AtomicUsize;
    use std::sync::{Arc, Mutex};
    use std::thread;

    // --- fake server --------------------------------------------------------

    /// Reads one request: returns its header block and body.
    fn read_request(stream: &mut TcpStream) -> (String, String) {
        let mut data = Vec::new();
        let mut byte = [0u8; 1];
        while !data.ends_with(b"\r\n\r\n") {
            if stream.read(&mut byte).unwrap_or(0) == 0 {
                break;
            }
            data.push(byte[0]);
        }
        let head = String::from_utf8_lossy(&data).into_owned();
        let length = head
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|v| v.trim().parse::<usize>().ok())
            })
            .unwrap_or(0);
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).expect("request body");
        (head, String::from_utf8_lossy(&body).into_owned())
    }

    use std::io::Read;

    fn write_chunk(stream: &mut TcpStream, data: &str) {
        write!(stream, "{:x}\r\n{data}\r\n", data.len()).expect("chunk");
        stream.flush().expect("flush");
    }

    const CHUNKED_HEAD: &str =
        "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\n\r\n";

    fn ndjson(piece: &str) -> String {
        format!(
            "{{\"message\":{{\"role\":\"assistant\",\"content\":{}}},\"done\":false}}\n",
            json!(piece)
        )
    }

    const DONE_LINE: &str =
        "{\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true}\n";

    /// Serves `connections` requests, answering each with `respond`.
    fn serve(
        connections: usize,
        respond: impl Fn(usize, &str, &mut TcpStream) + Send + 'static,
    ) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        thread::spawn(move || {
            for index in 0..connections {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let (_, body) = read_request(&mut stream);
                respond(index, &body, &mut stream);
            }
        });
        port
    }

    fn url(port: u16) -> String {
        format!("http://127.0.0.1:{port}")
    }

    fn template() -> Template {
        serde_json::from_str(
            r#"{"name":"t","description":"d","prompt":"Summarize in {language}:\n{transcript}"}"#,
        )
        .expect("template")
    }

    fn ctx(transcript: &str) -> RenderContext {
        RenderContext {
            transcript: transcript.to_string(),
            language: "French".to_string(),
            ..RenderContext::default()
        }
    }

    fn never() -> AtomicBool {
        AtomicBool::new(false)
    }

    // --- address parsing ----------------------------------------------------

    #[test]
    fn parse_accepts_scheme_host_and_port() {
        let endpoint = Endpoint::parse(" http://ollama-host:8080/ ").expect("parse");
        assert_eq!(endpoint.host, "ollama-host");
        assert_eq!(endpoint.port, 8080);
    }

    #[test]
    fn parse_defaults_to_the_ollama_port_and_accepts_a_bare_host() {
        let endpoint = Endpoint::parse("ollama-host").expect("parse");
        assert_eq!(endpoint.port, DEFAULT_OLLAMA_PORT);
        let endpoint = Endpoint::parse("http://ollama-host/api/tags?x=1").expect("parse");
        assert_eq!(
            (endpoint.host.as_str(), endpoint.port),
            ("ollama-host", 11434)
        );
    }

    #[test]
    fn parse_accepts_bracketed_ipv6() {
        let endpoint = Endpoint::parse("http://[::1]:9000").expect("parse");
        assert_eq!((endpoint.host.as_str(), endpoint.port), ("::1", 9000));
        assert_eq!(endpoint.host_header(), "[::1]:9000");
        assert_eq!(Endpoint::parse("[fd00::1]").expect("parse").port, 11434);
    }

    #[test]
    fn parse_rejects_https_with_an_explanation() {
        let err = Endpoint::parse("https://ollama-host").expect_err("https");
        assert!(err.to_string().contains("https is not supported"));
    }

    #[test]
    fn parse_rejects_bad_input() {
        for bad in [
            "",
            "   ",
            "http://",
            "http://user:pw@host",
            "http://host:0",
            "http://host:99999",
            "http://host:abc",
            "http://::1",
            "http://[::1",
            "http://:11434",
        ] {
            assert!(Endpoint::parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn private_ranges_are_recognised() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.1",
            "169.254.1.1",
            "100.64.0.1",
            "100.127.255.255",
            "::1",
            "fd12::1",
            "fe80::1",
            "::ffff:192.168.1.5",
        ] {
            let parsed: IpAddr = ip.parse().expect("ip");
            assert!(is_private_ip(parsed), "{ip} should be private");
        }
    }

    #[test]
    fn public_ranges_are_rejected() {
        for ip in [
            "8.8.8.8",
            "1.1.1.1",
            "172.32.0.1",
            "100.128.0.1",
            "100.63.255.255",
            "2001:4860:4860::8888",
            "::ffff:8.8.8.8",
        ] {
            let parsed: IpAddr = ip.parse().expect("ip");
            assert!(!is_private_ip(parsed), "{ip} should be public");
        }
    }

    #[test]
    fn a_public_address_is_refused_before_any_connection() {
        let err = list_remote_models("http://8.8.8.8:11434").expect_err("public");
        assert!(err.to_string().contains("non-local address"), "{err}");
    }

    #[test]
    fn an_unreachable_server_gets_a_readable_error() {
        // Bind then drop, so the port is closed.
        let port = TcpListener::bind("127.0.0.1:0")
            .expect("bind")
            .local_addr()
            .expect("addr")
            .port();
        let err = list_remote_models(&url(port)).expect_err("closed port");
        assert!(err.to_string().contains("cannot reach"), "{err}");
    }

    // --- model listing ------------------------------------------------------

    #[test]
    fn list_models_returns_sorted_names() {
        let port = serve(1, |_, _, stream| {
            let body = r#"{"models":[{"name":"qwen:9b"},{"name":"llama:3b"}]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .expect("write");
        });

        let names = list_remote_models(&url(port)).expect("list");

        assert_eq!(names, vec!["llama:3b", "qwen:9b"]);
    }

    #[test]
    fn list_models_flags_a_server_that_is_not_ollama() {
        let port = serve(1, |_, _, stream| {
            let body = "<html>hello</html>";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .expect("write");
        });

        let err = list_remote_models(&url(port)).expect_err("not ollama");

        assert!(err.to_string().contains("is it an Ollama server"), "{err}");
    }

    #[test]
    fn a_body_delimited_by_connection_close_is_read_to_the_end() {
        let port = serve(1, |_, _, stream| {
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n{{\"models\":[{{\"name\":\"a\"}}]}}"
            )
            .expect("write");
        });

        assert_eq!(list_remote_models(&url(port)).expect("list"), vec!["a"]);
    }

    // --- streaming ----------------------------------------------------------

    #[test]
    fn tokens_stream_across_chunk_and_line_boundaries() {
        let captured = Arc::new(Mutex::new(String::new()));
        let seen = Arc::clone(&captured);
        let port = serve(1, move |_, body, stream| {
            *seen.lock().expect("lock") = body.to_string();
            stream.write_all(CHUNKED_HEAD.as_bytes()).expect("head");
            let first = ndjson("Bon");
            let second = ndjson("jour é");
            // Cut the second line in the middle, with a pause longer than the
            // poll interval so the client takes a read timeout in between.
            let (a, b) = second.split_at(second.len() / 2);
            write_chunk(stream, &format!("{first}{a}"));
            thread::sleep(Duration::from_millis(350));
            write_chunk(stream, &format!("{b}{DONE_LINE}"));
            stream.write_all(b"0\r\n\r\n").expect("end");
        });
        let summarizer = RemoteSummarizer::new(&url(port), "qwen3.5:9b").expect("new");
        let mut pieces = Vec::new();

        let text = summarizer
            .summarize_transcript(&template(), &ctx("Hello"), &never(), |t| {
                pieces.push(t.to_string());
            })
            .expect("summarize");

        assert_eq!(text, "Bonjour é");
        assert_eq!(pieces, vec!["Bon", "jour é"]);
        let request: Value = serde_json::from_str(&captured.lock().expect("lock")).expect("json");
        assert_eq!(request["model"], "qwen3.5:9b");
        assert_eq!(request["stream"], true);
        assert_eq!(request["think"], false);
        assert_eq!(request["options"]["num_ctx"], 16_384);
        let prompt = request["messages"][0]["content"].as_str().expect("prompt");
        assert!(prompt.contains("Summarize in French:\nHello"), "{prompt}");
    }

    #[test]
    fn think_blocks_are_stripped_from_the_result() {
        let port = serve(1, |_, _, stream| {
            stream.write_all(CHUNKED_HEAD.as_bytes()).expect("head");
            write_chunk(
                stream,
                &format!(
                    "{}{}{DONE_LINE}",
                    ndjson("<think>hmm</think>"),
                    ndjson("\n\nSummary")
                ),
            );
            stream.write_all(b"0\r\n\r\n").expect("end");
        });
        let summarizer = RemoteSummarizer::new(&url(port), "m").expect("new");

        let text = summarizer
            .summarize_transcript(&template(), &ctx("x"), &never(), |_| {})
            .expect("summarize");

        assert_eq!(text, "Summary");
    }

    #[test]
    fn a_missing_model_surfaces_the_servers_own_message() {
        let port = serve(1, |_, _, stream| {
            let body = r#"{"error":"model 'nope' not found"}"#;
            write!(
                stream,
                "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .expect("write");
        });
        let summarizer = RemoteSummarizer::new(&url(port), "nope").expect("new");

        let err = summarizer
            .summarize_transcript(&template(), &ctx("x"), &never(), |_| {})
            .expect_err("404");

        let message = err.to_string();
        assert!(message.contains("HTTP 404"), "{message}");
        assert!(message.contains("model 'nope' not found"), "{message}");
    }

    #[test]
    fn an_error_inside_the_stream_is_reported() {
        let port = serve(1, |_, _, stream| {
            stream.write_all(CHUNKED_HEAD.as_bytes()).expect("head");
            write_chunk(
                stream,
                &format!("{}{{\"error\":\"out of memory\"}}\n", ndjson("par")),
            );
            stream.write_all(b"0\r\n\r\n").expect("end");
        });
        let summarizer = RemoteSummarizer::new(&url(port), "m").expect("new");

        let err = summarizer
            .summarize_transcript(&template(), &ctx("x"), &never(), |_| {})
            .expect_err("stream error");

        assert!(err.to_string().contains("out of memory"), "{err}");
    }

    #[test]
    fn a_stream_cut_before_done_is_an_error_not_a_truncated_summary() {
        let port = serve(1, |_, _, stream| {
            stream.write_all(CHUNKED_HEAD.as_bytes()).expect("head");
            write_chunk(stream, &ndjson("partial"));
            stream.write_all(b"0\r\n\r\n").expect("end");
        });
        let summarizer = RemoteSummarizer::new(&url(port), "m").expect("new");

        let err = summarizer
            .summarize_transcript(&template(), &ctx("x"), &never(), |_| {})
            .expect_err("cut");

        assert!(err.to_string().contains("ended the stream"), "{err}");
    }

    // --- cancellation and timeouts ------------------------------------------

    #[test]
    fn cancel_stops_a_stalled_generation_promptly() {
        let port = serve(1, |_, _, stream| {
            stream.write_all(CHUNKED_HEAD.as_bytes()).expect("head");
            write_chunk(stream, &ndjson("start"));
            thread::sleep(Duration::from_secs(5));
        });
        let summarizer = RemoteSummarizer::new(&url(port), "m").expect("new");
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(400));
            flag.store(true, Ordering::SeqCst);
        });
        let started = Instant::now();

        let result = summarizer.summarize_transcript(&template(), &ctx("x"), &cancel, |_| {});

        assert!(matches!(result, Err(LlmError::Cancelled)), "{result:?}");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn a_silent_server_times_out_with_a_helpful_message() {
        let port = serve(1, |_, _, _stream| thread::sleep(Duration::from_secs(5)));
        let summarizer = RemoteSummarizer::new(&url(port), "m")
            .expect("new")
            .with_options(RemoteOptions {
                idle_timeout: Duration::from_millis(500),
                ..RemoteOptions::default()
            });
        let started = Instant::now();

        let err = summarizer
            .summarize_transcript(&template(), &ctx("x"), &never(), |_| {})
            .expect_err("timeout");

        assert!(
            err.to_string().contains("loading the model or asleep"),
            "{err}"
        );
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    // --- map-reduce ---------------------------------------------------------

    #[test]
    fn a_transcript_that_exceeds_the_context_is_summarized_in_chunks() {
        let requests = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&requests);
        let port = serve(16, move |index, _body, stream| {
            counted.fetch_add(1, Ordering::SeqCst);
            stream.write_all(CHUNKED_HEAD.as_bytes()).expect("head");
            write_chunk(
                stream,
                &format!("{}{DONE_LINE}", ndjson(&format!("S{index}"))),
            );
            stream.write_all(b"0\r\n\r\n").expect("end");
        });
        // Budget: 400 - (100 + 64) = 236 tokens, i.e. about 700 characters.
        let summarizer = RemoteSummarizer::new(&url(port), "m")
            .expect("new")
            .with_options(RemoteOptions {
                num_ctx: 400,
                max_tokens: 100,
                ..RemoteOptions::default()
            });
        let transcript: String = (0..40)
            .map(|n| format!("Me: line number {n} with some words to fill the space\n"))
            .collect();

        let text = summarizer
            .summarize_transcript(&template(), &ctx(&transcript), &never(), |_| {})
            .expect("summarize");

        let total = requests.load(Ordering::SeqCst);
        assert!(
            total >= 3,
            "expected several chunk passes plus a final one, got {total}"
        );
        assert_eq!(
            text,
            format!("S{}", total - 1),
            "the last pass is the answer"
        );
    }

    #[test]
    fn a_short_transcript_needs_a_single_request() {
        let requests = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&requests);
        let port = serve(2, move |_, _, stream| {
            counted.fetch_add(1, Ordering::SeqCst);
            stream.write_all(CHUNKED_HEAD.as_bytes()).expect("head");
            write_chunk(stream, &format!("{}{DONE_LINE}", ndjson("ok")));
            stream.write_all(b"0\r\n\r\n").expect("end");
        });
        let summarizer = RemoteSummarizer::new(&url(port), "m").expect("new");

        summarizer
            .summarize_transcript(&template(), &ctx("short"), &never(), |_| {})
            .expect("summarize");

        assert_eq!(requests.load(Ordering::SeqCst), 1);
    }

    // --- small pure helpers -------------------------------------------------

    #[test]
    fn a_blank_model_name_is_rejected_up_front() {
        assert!(RemoteSummarizer::new("http://127.0.0.1", "  ").is_err());
    }

    #[test]
    fn strip_think_blocks_keeps_unclosed_text_and_removes_closed_blocks() {
        assert_eq!(
            strip_think_blocks("a<think>x</think>b<think>y</think>c"),
            "abc"
        );
        assert_eq!(
            strip_think_blocks("a<think>never closed"),
            "a<think>never closed"
        );
        assert_eq!(strip_think_blocks("plain"), "plain");
    }

    #[test]
    fn token_estimates_round_up() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("ab"), 1);
        assert_eq!(estimate_tokens("abcd"), 2);
    }
}
