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
    stream
        .set_write_timeout(Some(remaining(deadline)?))
        .map_err(|e| e.to_string())?;
    head.push_str(&payload);
    stream
        .write_all(head.as_bytes())
        .map_err(|e| e.to_string())?;
    let raw = read_message(&mut stream, deadline, MAX_RESPONSE_BYTES)?;
    parse_response(&raw)
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
/// `max` 바이트를 넘으면 거절한다. 헤더 끝은 새로 받은 부분만 찾고 헤더는 한 번만 해석한다(큰 응답에서도 선형).
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
            return Err(format!("server response too large (over {max} bytes)"));
        }
        let scan_from = raw.len().saturating_sub(3);
        raw.extend_from_slice(&buffer[..read]);
        if framing.is_none() {
            framing = raw[scan_from..]
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|at| {
                    let body_start = scan_from + at + 4;
                    (body_start, Framing::of(&raw[..body_start - 4]))
                });
        }
        if let Some((body_start, framing)) = &framing
            && framing.complete(&raw[*body_start..])
        {
            return Ok(raw);
        }
    }
}

/// 응답 본문의 끝을 아는 방법.
enum Framing {
    Length(usize),
    Chunked,
    /// 길이 정보 없음: EOF까지 읽는다.
    UntilClose,
}

impl Framing {
    fn of(head: &[u8]) -> Self {
        let head = String::from_utf8_lossy(head).to_ascii_lowercase();
        let header = |name: &str| {
            head.lines()
                .skip(1)
                .find_map(|line| line.split_once(':').filter(|(key, _)| key.trim() == name))
                .map(|(_, value)| value.trim().to_owned())
        };
        if header("transfer-encoding").is_some_and(|value| value.contains("chunked")) {
            return Self::Chunked;
        }
        match header("content-length").and_then(|value| value.parse::<usize>().ok()) {
            Some(length) => Self::Length(length),
            None => Self::UntilClose,
        }
    }

    fn complete(&self, body: &[u8]) -> bool {
        match self {
            Self::Length(length) => body.len() >= *length,
            // 마지막 chunk(`0\r\n\r\n`)는 항상 빈 줄로 끝난다: 그때만 풀어 본다.
            Self::Chunked => body.ends_with(b"\r\n\r\n") && dechunk(body).is_ok(),
            Self::UntilClose => false,
        }
    }
}

fn parse_response(raw: &[u8]) -> Result<(u16, Value), String> {
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "malformed http response".to_owned())?;
    let head = String::from_utf8_lossy(&raw[..split]);
    let mut body = raw[split + 4..].to_vec();
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| "malformed status line".to_owned())?;
    let chunked = head
        .lines()
        .any(|line| line.to_ascii_lowercase().replace(' ', "") == "transfer-encoding:chunked");
    if chunked {
        body = dechunk(&body)?;
    }
    Ok((status, serde_json::from_slice(&body).unwrap_or(Value::Null)))
}

fn dechunk(mut body: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let line_end = body
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| "malformed chunk".to_owned())?;
        let size = usize::from_str_radix(
            String::from_utf8_lossy(&body[..line_end])
                .split(';')
                .next()
                .unwrap_or("")
                .trim(),
            16,
        )
        .map_err(|error| error.to_string())?;
        body = &body[line_end + 2..];
        if size == 0 {
            return Ok(out);
        }
        if body.len() < size + 2 {
            return Err("truncated chunk".into());
        }
        out.extend_from_slice(&body[..size]);
        body = &body[size + 2..];
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
