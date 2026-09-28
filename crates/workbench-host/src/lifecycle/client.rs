//! 안내 파일의 서버를 확인하는 루프백 클라이언트(044 research R5). **순서가 계약이다**:
//! 1. 인증 없는 `/v1/system/identify`로 신원 증명을 받아, 안내 파일의 `ownerToken`으로 검증한다. 틀리면 그 끝점에
//!    자격 증명을 보내지 않는다.
//! 2. 맞을 때만 bearer로 handshake(인스턴스·프로토콜·저장 형식)와 준비 상태를 확인한다.
//!
//! 루프백 전용의 작은 HTTP/1.1 클라이언트다(프록시·리다이렉트 없음, `connection: close`).
//!
//! Codex r9: 요청 하나는 **전체** 시간 상한([`REQUEST_TIMEOUT`], 또는 호출자의 더 이른 deadline)과 응답 크기 상한
//! ([`MAX_RESPONSE_BYTES`])을 가진다. 읽기 대기마다가 아니라 연결·쓰기·읽기 전체가 한 deadline 안에서 끝난다 — 남은
//! 안내 파일의 포트를 다른 프로세스가 차지하고 끝없이 조금씩 보내도 멈추지 않는다. 읽기는 HTTP 메시지 길이
//! (`content-length`, chunked의 마지막 chunk)에서 끝나고, 둘 다 없을 때만 EOF까지 읽는다.

use std::{
    io::{ErrorKind, Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use super::descriptor::Descriptor;

/// 요청 하나의 전체 시간 상한(연결·쓰기·읽기 합).
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
/// 응답 하나(헤더 + 본문)의 크기 상한. lifecycle 응답(신원·handshake·준비·operation 출력)은 이보다 훨씬 작다.
pub const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// 끝점에 닿지 못했다(죽은 서버의 남은 안내 파일 등).
    Unreachable(String),
    /// 신원 증명이 없거나 틀렸다 — 자격 증명을 보내지 않았다.
    Identity(String),
    /// 프로토콜·저장 형식·인스턴스가 맞지 않는다.
    Incompatible(String),
    /// 준비 상태가 아니다.
    NotReady(String),
    /// 신원·호환은 맞지만 서빙 중이 아니다(비우는 중·정지 중) — 붙을 대상이 아니다(contracts §3).
    NotServing(String),
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(reason) => write!(f, "server unreachable: {reason}"),
            Self::Identity(reason) => write!(f, "server identity proof failed: {reason}"),
            Self::Incompatible(reason) => write!(f, "server is incompatible: {reason}"),
            Self::NotReady(reason) => write!(f, "server is not ready: {reason}"),
            Self::NotServing(state) => write!(f, "server is not serving (state {state})"),
        }
    }
}

impl std::error::Error for VerifyError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    pub instance_id: String,
    pub server_epoch: String,
    pub base_url: String,
}

/// 안내 파일의 서버를 확인한다(신원 증명 → 자격 증명 사용). 요청마다 [`REQUEST_TIMEOUT`] 상한.
pub fn verify(descriptor: &Descriptor) -> Result<Verified, VerifyError> {
    verify_by(descriptor, None)
}

/// [`verify`]와 같되 모든 요청이 `deadline`(있으면) 전에 끝난다 — `ensure`의 시작 제한 시간이 확인을 포함한다.
pub fn verify_by(
    descriptor: &Descriptor,
    deadline: Option<Instant>,
) -> Result<Verified, VerifyError> {
    let identity = descriptor.identity();
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let (status, body) = request_by(
        &descriptor.base_url,
        "POST",
        "/v1/system/identify",
        Some(&json!({ "nonce": nonce })),
        None,
        None,
        deadline,
    )
    .map_err(VerifyError::Unreachable)?;
    if status != 200 {
        return Err(VerifyError::Identity(format!("identify answered {status}")));
    }
    let instance = body["instanceId"].as_str().unwrap_or_default();
    let proof = body["proof"].as_str().unwrap_or_default();
    if instance != descriptor.instance_id || proof != identity.proof(&nonce, instance) {
        return Err(VerifyError::Identity(
            "the endpoint is not the descriptor's server instance".into(),
        ));
    }

    let token = descriptor.owner_token.as_str();
    let (status, handshake) = request_by(
        &descriptor.base_url,
        "POST",
        "/v1/system/handshake",
        Some(&json!({
            "supportedProtocolVersions": [workbench_protocol::PROTOCOL_VERSION],
            "client": { "name": "agentic-workbench-server", "version": env!("CARGO_PKG_VERSION") },
        })),
        Some(token),
        None,
        deadline,
    )
    .map_err(VerifyError::Unreachable)?;
    if status != 200 {
        return Err(VerifyError::Incompatible(format!(
            "handshake answered {status}"
        )));
    }
    if handshake["instanceId"].as_str() != Some(descriptor.instance_id.as_str()) {
        return Err(VerifyError::Incompatible(
            "instance changed during handshake".into(),
        ));
    }
    let schema = handshake["storageSchemaVersion"]
        .as_i64()
        .unwrap_or_default();
    if schema != workbench_core::infrastructure::sqlite_ledger::SCHEMA_VERSION {
        return Err(VerifyError::Incompatible(format!(
            "storage schema {schema} is not supported"
        )));
    }

    let (status, ready) = request_by(
        &descriptor.base_url,
        "GET",
        "/health/ready",
        None,
        Some(token),
        None,
        deadline,
    )
    .map_err(VerifyError::Unreachable)?;
    if status != 200 || ready["ready"].as_bool() != Some(true) {
        return Err(VerifyError::NotReady(format!("ready answered {status}")));
    }
    Ok(Verified {
        instance_id: descriptor.instance_id.clone(),
        server_epoch: handshake["serverEpoch"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        base_url: descriptor.base_url.clone(),
    })
}

/// 확인된 서버가 서빙 중인지(contracts/server-lifecycle.md §3 — 소유자 토큰으로 `server.status`). `verify`를 통과한 안내에만
/// 부른다(신원 증명 뒤에만 자격 증명을 보낸다).
pub fn require_serving(descriptor: &Descriptor) -> Result<(), VerifyError> {
    require_serving_by(descriptor, None)
}

/// [`require_serving`]와 같되 요청이 `deadline`(있으면) 전에 끝난다.
pub fn require_serving_by(
    descriptor: &Descriptor,
    deadline: Option<Instant>,
) -> Result<(), VerifyError> {
    let status = super::calls::call_by(
        &descriptor.base_url,
        &descriptor.owner_token,
        None,
        "server.status",
        json!({}),
        false,
        deadline,
    )
    .map_err(|error| VerifyError::Unreachable(error.to_string()))?;
    match status["state"].as_str() {
        Some("serving") => Ok(()),
        other => Err(VerifyError::NotServing(
            other.unwrap_or("unknown").to_owned(),
        )),
    }
}

/// 루프백 JSON 요청. `(status, body)`. body가 JSON이 아니면 `Null`.
pub fn request(
    base_url: &str,
    method: &str,
    path: &str,
    body: Option<&Value>,
    bearer: Option<&str>,
) -> Result<(u16, Value), String> {
    request_with_origin(base_url, method, path, body, bearer, None)
}

/// `request`와 같되 `Origin` 헤더를 싣는다(044 T029: 데스크톱이 창 토큰으로 부를 때 — 창 토큰은 WebView 출처에 묶인다).
pub fn request_with_origin(
    base_url: &str,
    method: &str,
    path: &str,
    body: Option<&Value>,
    bearer: Option<&str>,
    origin: Option<&str>,
) -> Result<(u16, Value), String> {
    request_by(base_url, method, path, body, bearer, origin, None)
}

/// 요청 하나. 전체 deadline은 `min(지금 + REQUEST_TIMEOUT, deadline)`이다.
pub fn request_by(
    base_url: &str,
    method: &str,
    path: &str,
    body: Option<&Value>,
    bearer: Option<&str>,
    origin: Option<&str>,
    deadline: Option<Instant>,
) -> Result<(u16, Value), String> {
    let own = Instant::now() + REQUEST_TIMEOUT;
    let deadline = deadline.map_or(own, |caller| caller.min(own));
    let authority = base_url
        .strip_prefix("http://")
        .ok_or_else(|| format!("unsupported base url {base_url}"))?;
    let host = authority.split(':').next().unwrap_or_default();
    if host != "127.0.0.1" && host != "localhost" {
        return Err(format!("refusing a non-loopback server address {base_url}"));
    }
    let payload = body.map(|value| value.to_string()).unwrap_or_default();
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nhost: {authority}\r\nconnection: close\r\naccept: application/json\r\n"
    );
    if body.is_some() {
        head.push_str(&format!(
            "content-type: application/json\r\ncontent-length: {}\r\n",
            payload.len()
        ));
    }
    if let Some(token) = bearer {
        head.push_str(&format!("authorization: Bearer {token}\r\n"));
    }
    if let Some(origin) = origin {
        head.push_str(&format!("origin: {origin}\r\n"));
    }
    head.push_str("\r\n");

    let address = authority
        .parse::<std::net::SocketAddr>()
        .or_else(|_| {
            std::net::ToSocketAddrs::to_socket_addrs(authority)
                .map_err(|error| error.to_string())?
                .next()
                .ok_or_else(|| "no address".to_owned())
        })
        .map_err(|error| error.to_string())?;
    let mut stream =
        TcpStream::connect_timeout(&address, remaining(deadline)?).map_err(|e| e.to_string())?;
    head.push_str(&payload);
    write_message(&mut stream, head.as_bytes(), deadline)?;
    let raw = read_message(&mut stream, deadline, MAX_RESPONSE_BYTES)?;
    parse_response(&raw)
}

/// 요청 하나를 쓴다(Codex r11): 부분 쓰기마다 대기 상한을 `deadline`까지 남은 시간으로 다시 잡는다 — 끝점이 천천히 읽어
/// 쓰기가 조금씩만 진행돼도(backpressure) 전체가 `deadline` 안에서 끝난다. 0바이트 쓰기는 끝점이 닫힌 것이다.
fn write_message(stream: &mut TcpStream, bytes: &[u8], deadline: Instant) -> Result<(), String> {
    let mut written = 0;
    while written < bytes.len() {
        stream
            .set_write_timeout(Some(remaining(deadline)?))
            .map_err(|e| e.to_string())?;
        match stream.write(&bytes[written..]) {
            Ok(0) => return Err("the server closed the connection during the request".to_owned()),
            Ok(count) => written += count,
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                return Err(MESSAGE_TIMED_OUT.to_owned());
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

const MESSAGE_TIMED_OUT: &str = "timed out waiting for the server response";

/// deadline까지 남은 시간(지났으면 시간 초과 오류).
fn remaining(deadline: Instant) -> Result<Duration, String> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|left| !left.is_zero())
        .ok_or_else(|| MESSAGE_TIMED_OUT.to_owned())
}

/// 응답 하나를 읽는다: 메시지가 끝나면(길이·마지막 chunk) 멈추고, 길이 정보가 없으면 EOF까지. 전체가 `deadline` 안이고
/// `max` 바이트를 넘으면 거절한다. 헤더 끝은 새로 받은 부분만 찾고 헤더는 한 번만 해석한다. chunked 본문은 한 번 지나간
/// 곳을 다시 보지 않는 증분 검사로 끝을 찾는다(큰 응답에서도 선형, Codex r9·r10).
///
/// 응답 틀은 신뢰하지 않는 입력이다(Codex r10): 신원 확인 전에 아무 프로세스가 보낼 수 있다. 길이 선언은 모두 검사한
/// 산술로 다루고, 상한을 넘는 선언은 본문을 기다리지 않고 곧바로, 잘못된 틀은 곧바로 오류로 끝난다(panic 없음).
fn read_message(stream: &mut TcpStream, deadline: Instant, max: usize) -> Result<Vec<u8>, String> {
    let mut raw = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut framing: Option<(usize, Framing)> = None;
    loop {
        stream
            .set_read_timeout(Some(remaining(deadline)?))
            .map_err(|e| e.to_string())?;
        let read = match stream.read(&mut buffer) {
            Ok(0) => return Ok(raw),
            Ok(read) => read,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                return Err(MESSAGE_TIMED_OUT.to_owned());
            }
            Err(error) => return Err(error.to_string()),
        };
        if raw.len() + read > max {
            return Err(too_large(max));
        }
        let scan_from = raw.len().saturating_sub(3);
        raw.extend_from_slice(&buffer[..read]);
        if framing.is_none()
            && let Some(at) = raw[scan_from..]
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
        {
            let body_start = scan_from + at + 4;
            framing = Some((body_start, Framing::of(&raw[..body_start - 4], max)?));
        }
        if let Some((body_start, framing)) = &mut framing
            && framing.complete(&raw[*body_start..], max)?
        {
            return Ok(raw);
        }
    }
}

const MESSAGE_MALFORMED_CHUNK: &str = "malformed chunk";
/// chunk 크기 줄(확장자 포함)·trailer 줄 하나의 길이 상한.
const MAX_CHUNK_LINE: usize = 1024;

fn too_large(max: usize) -> String {
    format!("server response too large (over {max} bytes)")
}

/// 응답 본문의 끝을 아는 방법.
enum Framing {
    Length(usize),
    Chunked(ChunkScan),
    /// 길이 정보 없음: EOF까지 읽는다.
    UntilClose,
}

impl Framing {
    /// 헤더에서 본문 틀을 정한다. `content-length`가 숫자가 아니거나 서로 다른 값으로 여러 번 오면 잘못된 틀, `max`를 넘으면
    /// 상한 초과다(본문을 기다리지 않는다).
    fn of(head: &[u8], max: usize) -> Result<Self, String> {
        let head = String::from_utf8_lossy(head).to_ascii_lowercase();
        let values = |name: &str| -> Vec<String> {
            head.lines()
                .skip(1)
                .filter_map(|line| line.split_once(':'))
                .filter(|(key, _)| key.trim() == name)
                .map(|(_, value)| value.trim().to_owned())
                .collect()
        };
        if values("transfer-encoding")
            .iter()
            .any(|value| value.contains("chunked"))
        {
            return Ok(Self::Chunked(ChunkScan::default()));
        }
        let lengths = values("content-length");
        let Some(first) = lengths.first() else {
            return Ok(Self::UntilClose);
        };
        if lengths.iter().any(|value| value != first) {
            return Err("malformed content-length (conflicting values)".into());
        }
        if first.is_empty() || !first.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(format!("malformed content-length {first:?}"));
        }
        match first.parse::<usize>() {
            Ok(length) if length <= max => Ok(Self::Length(length)),
            // 자릿수가 usize를 넘거나 상한보다 크다.
            _ => Err(too_large(max)),
        }
    }

    /// 본문이 끝났는가(잘못된 chunk 틀이면 오류). chunked는 지난 호출에서 본 곳부터 이어 본다.
    fn complete(&mut self, body: &[u8], max: usize) -> Result<bool, String> {
        match self {
            Self::Length(length) => Ok(body.len() >= *length),
            Self::Chunked(scan) => scan.advance(body, max),
            Self::UntilClose => Ok(false),
        }
    }
}

/// chunked 본문의 증분 검사. `pos`까지는 확인한 완결 단위(크기 줄·데이터·trailer 줄)다.
#[derive(Default)]
struct ChunkScan {
    pos: usize,
    state: ChunkState,
    /// 지금까지 선언된 데이터 합(상한 검사).
    declared: usize,
}

#[derive(Default, Clone, Copy)]
enum ChunkState {
    #[default]
    Size,
    Data(usize),
    Trailer,
}

impl ChunkScan {
    fn advance(&mut self, body: &[u8], max: usize) -> Result<bool, String> {
        loop {
            match self.state {
                ChunkState::Size => {
                    let Some(line_end) = line_end_from(body, self.pos)? else {
                        return Ok(false);
                    };
                    let size = chunk_size(&body[self.pos..line_end], max)?;
                    self.declared = self
                        .declared
                        .checked_add(size)
                        .filter(|total| *total <= max)
                        .ok_or_else(|| too_large(max))?;
                    self.pos = line_end + 2;
                    self.state = if size == 0 {
                        ChunkState::Trailer
                    } else {
                        ChunkState::Data(size)
                    };
                }
                ChunkState::Data(size) => {
                    let data_end = self.pos.checked_add(size).ok_or_else(|| too_large(max))?;
                    // 데이터와 그 뒤 CRLF가 아직 다 오지 않았다.
                    let Some(crlf) = body.get(data_end..data_end.saturating_add(2)) else {
                        return Ok(false);
                    };
                    if crlf != b"\r\n" {
                        return Err(MESSAGE_MALFORMED_CHUNK.into());
                    }
                    self.pos = data_end + 2;
                    self.state = ChunkState::Size;
                }
                ChunkState::Trailer => {
                    let Some(line_end) = line_end_from(body, self.pos)? else {
                        return Ok(false);
                    };
                    if line_end == self.pos {
                        return Ok(true);
                    }
                    // trailer 헤더 줄은 건너뛴다.
                    self.pos = line_end + 2;
                }
            }
        }
    }
}

/// `from`부터 첫 CRLF 위치(없으면 아직 모름). 줄이 [`MAX_CHUNK_LINE`]을 넘으면 잘못된 틀.
fn line_end_from(body: &[u8], from: usize) -> Result<Option<usize>, String> {
    let rest = body.get(from..).unwrap_or(&[]);
    match rest.windows(2).position(|window| window == b"\r\n") {
        Some(at) if at <= MAX_CHUNK_LINE => Ok(Some(from + at)),
        Some(_) => Err(MESSAGE_MALFORMED_CHUNK.into()),
        None if rest.len() > MAX_CHUNK_LINE => Err(MESSAGE_MALFORMED_CHUNK.into()),
        None => Ok(None),
    }
}

/// chunk 크기 줄(`<16진 크기>[;확장자]`)을 해석한다. 16진이 아니거나 자릿수가 usize를 넘으면 잘못된 틀, `max`를 넘으면 상한
/// 초과다.
fn chunk_size(line: &[u8], max: usize) -> Result<usize, String> {
    let text = std::str::from_utf8(line).map_err(|_| MESSAGE_MALFORMED_CHUNK.to_owned())?;
    let digits = text.split(';').next().unwrap_or("").trim();
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(MESSAGE_MALFORMED_CHUNK.into());
    }
    let size = usize::from_str_radix(digits, 16).map_err(|_| MESSAGE_MALFORMED_CHUNK.to_owned())?;
    if size > max {
        return Err(too_large(max));
    }
    Ok(size)
}

fn parse_response(raw: &[u8]) -> Result<(u16, Value), String> {
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "malformed http response".to_owned())?;
    let head_bytes = &raw[..split];
    let head = String::from_utf8_lossy(head_bytes);
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| "malformed status line".to_owned())?;
    let body = &raw[split + 4..];
    let body = match Framing::of(head_bytes, MAX_RESPONSE_BYTES)? {
        Framing::Chunked(_) => dechunk(body, MAX_RESPONSE_BYTES)?,
        Framing::Length(length) => body
            .get(..length)
            .ok_or_else(|| "truncated response body".to_owned())?
            .to_vec(),
        Framing::UntilClose => body.to_vec(),
    };
    Ok((status, serde_json::from_slice(&body).unwrap_or(Value::Null)))
}

/// chunked 본문을 푼다(한 번 지나가는 검사, 검사한 산술). 잘못된 틀·잘린 chunk는 오류.
fn dechunk(body: &[u8], max: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    loop {
        let line_end = line_end_from(body, pos)?.ok_or_else(|| "truncated chunk".to_owned())?;
        let size = chunk_size(&body[pos..line_end], max)?;
        pos = line_end + 2;
        if size == 0 {
            return Ok(out);
        }
        let data_end = pos.checked_add(size).ok_or_else(|| too_large(max))?;
        let data = body
            .get(pos..data_end)
            .ok_or_else(|| "truncated chunk".to_owned())?;
        if body.get(data_end..data_end.saturating_add(2)) != Some(b"\r\n".as_slice()) {
            return Err(MESSAGE_MALFORMED_CHUNK.into());
        }
        if out.len() + data.len() > max {
            return Err(too_large(max));
        }
        out.extend_from_slice(data);
        pos = data_end + 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_chunked_json_responses() {
        let plain = b"HTTP/1.1 200 OK\r\ncontent-length: 7\r\n\r\n{\"a\":1}";
        assert_eq!(parse_response(plain).unwrap(), (200, json!({"a": 1})));
        let chunked = b"HTTP/1.1 404 Not Found\r\ntransfer-encoding: chunked\r\n\r\n7\r\n{\"a\":1}\r\n0\r\n\r\n";
        assert_eq!(parse_response(chunked).unwrap(), (404, json!({"a": 1})));
    }

    #[test]
    fn refuses_non_loopback_addresses() {
        assert!(request("http://example.com:80", "GET", "/", None, None).is_err());
    }
}
