//! Read-only probes used to decide whether a target can satisfy the 045
//! containment contract before any production consumer is migrated.

use std::{collections::BTreeMap, io, process::Child};

/// Evidence gathered from the running target. A `false` capability is a design
/// blocker, not a reason to weaken the process-tree contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnixCapabilityReport {
    pub target: &'static str,
    pub same_uid_environment_readable: bool,
    pub reusable_signal_handle: bool,
    pub env_clear_descendant_trackable: bool,
    pub ordinary_deployment_permissions: bool,
    pub blockers: Vec<&'static str>,
}

impl UnixCapabilityReport {
    #[must_use]
    pub fn supports_required_containment(&self) -> bool {
        self.same_uid_environment_readable
            && self.reusable_signal_handle
            && self.env_clear_descendant_trackable
            && self.ordinary_deployment_permissions
    }
}

/// A start identity can reject a stale PID observation, but it does not make a
/// later `kill(pid, ..)` atomic with that observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessStartIdentity {
    pub pid: u32,
    pub started_seconds: u64,
    pub started_microseconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentProbeEvidence {
    pub returned_size: usize,
    pub sysctl_errno: Option<i32>,
    pub raw_marker_present: bool,
    pub parsed_key_present: bool,
    pub parsed_value_matches: bool,
}

pub fn process_start_identity(pid: u32) -> io::Result<ProcessStartIdentity> {
    process_start_identity_impl(pid)
}

pub fn process_environment(pid: u32) -> io::Result<BTreeMap<Vec<u8>, Vec<u8>>> {
    process_environment_impl(pid)
}

pub fn probe_environment_marker(pid: u32, key: &[u8], value: &[u8]) -> EnvironmentProbeEvidence {
    probe_environment_marker_impl(pid, key, value)
}

pub fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(target_os = "macos")]
#[must_use]
pub fn capability_report(environment_probe_succeeded: bool) -> UnixCapabilityReport {
    UnixCapabilityReport {
        target: "macos",
        same_uid_environment_readable: environment_probe_succeeded,
        reusable_signal_handle: false,
        env_clear_descendant_trackable: false,
        ordinary_deployment_permissions: environment_probe_succeeded,
        blockers: vec![
            "macOS exposes PID/start metadata but no ordinary-app reusable process handle for atomic signal delivery",
            "a descendant that clears the launch nonce and reparents cannot be attributed by nonce or parent lineage",
        ],
    }
}

#[cfg(target_os = "linux")]
#[must_use]
pub fn capability_report(environment_probe_succeeded: bool) -> UnixCapabilityReport {
    let pidfd_available = pidfd_open(std::process::id()).is_ok();
    UnixCapabilityReport {
        target: "linux",
        same_uid_environment_readable: environment_probe_succeeded,
        reusable_signal_handle: pidfd_available,
        env_clear_descendant_trackable: false,
        ordinary_deployment_permissions: environment_probe_succeeded,
        blockers: vec![
            "pidfd protects a known process identity but does not discover descendants that clear the nonce",
            "cgroup-v2 delegation for an ordinary desktop/server process is not yet established",
        ],
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
#[must_use]
pub fn capability_report(environment_probe_succeeded: bool) -> UnixCapabilityReport {
    UnixCapabilityReport {
        target: "unsupported-unix",
        same_uid_environment_readable: environment_probe_succeeded,
        reusable_signal_handle: false,
        env_clear_descendant_trackable: false,
        ordinary_deployment_permissions: false,
        blockers: vec!["target has no reviewed containment design"],
    }
}

#[cfg(target_os = "macos")]
fn process_start_identity_impl(pid: u32) -> io::Result<ProcessStartIdentity> {
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::zeroed();
    let size = std::mem::size_of::<libc::proc_bsdinfo>();
    // SAFETY: `info` points to writable storage of the exact size passed to
    // libproc. `proc_pidinfo` initializes the structure on a full-size result.
    let read = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            size as libc::c_int,
        )
    };
    if read != size as libc::c_int {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a full structure was returned above.
    let info = unsafe { info.assume_init() };
    Ok(ProcessStartIdentity {
        pid,
        started_seconds: info.pbi_start_tvsec,
        started_microseconds: info.pbi_start_tvusec,
    })
}

#[cfg(target_os = "macos")]
fn process_environment_impl(pid: u32) -> io::Result<BTreeMap<Vec<u8>, Vec<u8>>> {
    let bytes = macos_procargs(pid)?;
    Ok(parse_macos_procargs_environment(&bytes))
}

#[cfg(target_os = "macos")]
fn macos_procargs(pid: u32) -> io::Result<Vec<u8>> {
    let mut argmax: libc::c_int = 0;
    let mut argmax_size = std::mem::size_of::<libc::c_int>();
    let mut argmax_mib = [libc::CTL_KERN, libc::KERN_ARGMAX];
    // SAFETY: MIB and destination point to valid fixed-size buffers.
    if unsafe {
        libc::sysctl(
            argmax_mib.as_mut_ptr(),
            argmax_mib.len() as libc::c_uint,
            (&mut argmax as *mut libc::c_int).cast(),
            &mut argmax_size,
            std::ptr::null_mut(),
            0,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }

    let mut bytes = vec![0_u8; usize::try_from(argmax).unwrap_or(0)];
    let mut size = bytes.len();
    let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid as libc::c_int];
    // SAFETY: MIB and output buffer remain valid for the duration of sysctl.
    if unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as libc::c_uint,
            bytes.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    bytes.truncate(size);
    Ok(bytes)
}

#[cfg(target_os = "macos")]
fn parse_macos_procargs_environment(bytes: &[u8]) -> BTreeMap<Vec<u8>, Vec<u8>> {
    if bytes.len() < std::mem::size_of::<libc::c_int>() {
        return BTreeMap::new();
    }
    let argc = libc::c_int::from_ne_bytes(bytes[..4].try_into().unwrap_or([0; 4])).max(0) as usize;
    let mut cursor = 4;
    skip_c_string(bytes, &mut cursor);
    while bytes.get(cursor) == Some(&0) {
        cursor += 1;
    }
    for _ in 0..argc {
        skip_c_string(bytes, &mut cursor);
    }
    while bytes.get(cursor) == Some(&0) {
        cursor += 1;
    }
    let mut environment = BTreeMap::new();
    while cursor < bytes.len() {
        let end = bytes[cursor..]
            .iter()
            .position(|byte| *byte == 0)
            .map_or(bytes.len(), |relative| cursor + relative);
        let field = &bytes[cursor..end];
        if field.is_empty() {
            break;
        }
        if let Some(index) = field.iter().position(|byte| *byte == b'=') {
            environment.insert(field[..index].to_vec(), field[index + 1..].to_vec());
        }
        cursor = end.saturating_add(1);
    }
    environment
}

#[cfg(target_os = "macos")]
fn skip_c_string(bytes: &[u8], cursor: &mut usize) {
    while *cursor < bytes.len() && bytes[*cursor] != 0 {
        *cursor += 1;
    }
    if *cursor < bytes.len() {
        *cursor += 1;
    }
}

#[cfg(target_os = "macos")]
fn probe_environment_marker_impl(pid: u32, key: &[u8], value: &[u8]) -> EnvironmentProbeEvidence {
    match macos_procargs(pid) {
        Ok(bytes) => {
            let parsed = parse_macos_procargs_environment(&bytes);
            let mut raw_marker = Vec::with_capacity(key.len() + value.len() + 1);
            raw_marker.extend_from_slice(key);
            raw_marker.push(b'=');
            raw_marker.extend_from_slice(value);
            EnvironmentProbeEvidence {
                returned_size: bytes.len(),
                sysctl_errno: None,
                raw_marker_present: bytes
                    .windows(raw_marker.len())
                    .any(|window| window == raw_marker),
                parsed_key_present: parsed.contains_key(key),
                parsed_value_matches: parsed.get(key) == Some(&value.to_vec()),
            }
        }
        Err(error) => EnvironmentProbeEvidence {
            returned_size: 0,
            sysctl_errno: error.raw_os_error(),
            raw_marker_present: false,
            parsed_key_present: false,
            parsed_value_matches: false,
        },
    }
}

#[cfg(target_os = "linux")]
fn process_start_identity_impl(pid: u32) -> io::Result<ProcessStartIdentity> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))?;
    let close = stat
        .rfind(')')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing comm terminator"))?;
    let fields: Vec<&str> = stat[close + 1..].split_whitespace().collect();
    let ticks = fields
        .get(19)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing starttime"))?
        .parse::<u64>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(ProcessStartIdentity {
        pid,
        started_seconds: ticks,
        started_microseconds: 0,
    })
}

#[cfg(target_os = "linux")]
fn process_environment_impl(pid: u32) -> io::Result<BTreeMap<Vec<u8>, Vec<u8>>> {
    let bytes = std::fs::read(format!("/proc/{pid}/environ"))?;
    Ok(bytes
        .split(|byte| *byte == 0)
        .filter_map(|field| {
            let index = field.iter().position(|byte| *byte == b'=')?;
            Some((field[..index].to_vec(), field[index + 1..].to_vec()))
        })
        .collect())
}

#[cfg(target_os = "linux")]
fn probe_environment_marker_impl(pid: u32, key: &[u8], value: &[u8]) -> EnvironmentProbeEvidence {
    match std::fs::read(format!("/proc/{pid}/environ")) {
        Ok(bytes) => {
            let parsed = process_environment_impl(pid).unwrap_or_default();
            let mut raw_marker = Vec::with_capacity(key.len() + value.len() + 1);
            raw_marker.extend_from_slice(key);
            raw_marker.push(b'=');
            raw_marker.extend_from_slice(value);
            EnvironmentProbeEvidence {
                returned_size: bytes.len(),
                sysctl_errno: None,
                raw_marker_present: bytes
                    .windows(raw_marker.len())
                    .any(|window| window == raw_marker),
                parsed_key_present: parsed.contains_key(key),
                parsed_value_matches: parsed.get(key) == Some(&value.to_vec()),
            }
        }
        Err(error) => EnvironmentProbeEvidence {
            returned_size: 0,
            sysctl_errno: error.raw_os_error(),
            raw_marker_present: false,
            parsed_key_present: false,
            parsed_value_matches: false,
        },
    }
}

#[cfg(target_os = "linux")]
fn pidfd_open(pid: u32) -> io::Result<libc::c_int> {
    // SAFETY: pidfd_open has no pointer arguments. The returned descriptor is
    // closed below on success.
    let descriptor = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) as libc::c_int };
    if descriptor < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: descriptor was returned by pidfd_open and is owned here.
        unsafe { libc::close(descriptor) };
        Ok(descriptor)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn process_start_identity_impl(_pid: u32) -> io::Result<ProcessStartIdentity> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "unsupported Unix target",
    ))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn process_environment_impl(_pid: u32) -> io::Result<BTreeMap<Vec<u8>, Vec<u8>>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "unsupported Unix target",
    ))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn probe_environment_marker_impl(
    _pid: u32,
    _key: &[u8],
    _value: &[u8],
) -> EnvironmentProbeEvidence {
    EnvironmentProbeEvidence {
        returned_size: 0,
        sysctl_errno: Some(libc::ENOTSUP),
        raw_marker_present: false,
        parsed_key_present: false,
        parsed_value_matches: false,
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::parse_macos_procargs_environment;

    fn procargs(argc: i32, fields: &[&[u8]]) -> Vec<u8> {
        let mut bytes = argc.to_ne_bytes().to_vec();
        for field in fields {
            bytes.extend_from_slice(field);
            bytes.push(0);
        }
        bytes
    }

    #[test]
    fn parser_preserves_empty_argv_while_locating_environment() {
        let bytes = procargs(
            3,
            &[
                b"/fixture",
                b"",
                b"fixture",
                b"",
                b"tail",
                b"AW_045_NONCE=present",
                b"",
            ],
        );
        let environment = parse_macos_procargs_environment(&bytes);
        assert_eq!(
            environment.get(b"AW_045_NONCE".as_slice()),
            Some(&b"present".to_vec())
        );
    }
}
