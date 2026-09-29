//! Owner-only, descriptor-relative no-follow lookup. Located is not verified.
use crate::{
    application::admission::CallerProfile,
    domain::attempt::EndpointIdentity,
    ports::{ClientError, Credential, CredentialProvider},
};
use serde::Deserialize;
use std::{
    ffi::CString,
    fs::File,
    io::Read,
    net::SocketAddr,
    os::unix::{
        fs::MetadataExt,
        io::{AsRawFd, FromRawFd},
    },
    path::{Component, Path},
};

pub const DESCRIPTOR_LIMIT: usize = 64 * 1024;
/// Wire/storage compatibility for merged044. Not a migration or readiness proof.
pub const STORAGE_SCHEMA_VERSION: i64 = 2;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Descriptor {
    format_version: u32,
    mode: String,
    instance_id: String,
    server_epoch: String,
    base_url: String,
    protocol_versions: Vec<u16>,
    storage_schema_version: i64,
    owner_token: String,
}
#[derive(Debug)]
pub struct LocatedEndpoint {
    identity: EndpointIdentity,
    address: SocketAddr,
    credential: Credential,
}
impl LocatedEndpoint {
    pub fn identity(&self) -> &EndpointIdentity {
        &self.identity
    }
    pub fn address(&self) -> SocketAddr {
        self.address
    }
}
impl CredentialProvider for LocatedEndpoint {
    fn credential(&self) -> &Credential {
        &self.credential
    }
}

pub fn read_descriptor(
    path: &Path,
    profile: CallerProfile,
) -> Result<LocatedEndpoint, ClientError> {
    if profile != CallerProfile::Owner {
        return Err(ClientError::PrerequisiteUnavailable);
    }
    let file = open_private_file_for_descriptor(path)?;
    let bytes = read_bounded(file, DESCRIPTOR_LIMIT)?;
    let descriptor: Descriptor =
        serde_json::from_slice(&bytes).map_err(|_| ClientError::Identity)?;
    if descriptor.format_version != 1
        || !matches!(descriptor.mode.as_str(), "server" | "embedded")
        || !descriptor
            .protocol_versions
            .contains(&workbench_protocol::PROTOCOL_VERSION)
        || descriptor.storage_schema_version != STORAGE_SCHEMA_VERSION
    {
        return Err(ClientError::Incompatible);
    }
    let identity = EndpointIdentity::new(descriptor.instance_id, descriptor.server_epoch)
        .map_err(|_| ClientError::Identity)?;
    let authority = descriptor
        .base_url
        .strip_prefix("http://")
        .ok_or(ClientError::Identity)?;
    let address: SocketAddr = authority.parse().map_err(|_| ClientError::Identity)?;
    if !address.ip().is_loopback()
        || address.port() == 0
        || !(address.ip() == std::net::Ipv4Addr::LOCALHOST
            || address.ip() == std::net::Ipv6Addr::LOCALHOST)
    {
        return Err(ClientError::Identity);
    }
    Ok(LocatedEndpoint {
        identity,
        address,
        credential: Credential::new(descriptor.owner_token)?,
    })
}

/// Traverse from an owned root FD, refusing links on every component. Shared system
/// ancestors may be root-owned/sticky; the immediate control directory must be private.
fn open_private_file_for_descriptor(path: &Path) -> Result<File, ClientError> {
    open_private_file_by(path, true)
}

fn open_private_file_by(path: &Path, missing_unavailable: bool) -> Result<File, ClientError> {
    let mut components = path.components();
    if components.next() != Some(Component::RootDir) {
        return Err(ClientError::PrivateState);
    }
    let names: Vec<_> = components
        .map(|c| match c {
            Component::Normal(n) => {
                CString::new(n.as_encoded_bytes()).map_err(|_| ClientError::PrivateState)
            }
            _ => Err(ClientError::PrivateState),
        })
        .collect::<Result<_, _>>()?;
    if names.is_empty() {
        return Err(ClientError::PrivateState);
    }
    let mut directory = File::open("/").map_err(|_| ClientError::PrivateState)?;
    let uid = unsafe { libc::geteuid() };
    for (index, name) in names.iter().enumerate() {
        let last = index == names.len() - 1;
        if last {
            let meta = directory
                .metadata()
                .map_err(|_| ClientError::PrivateState)?;
            if meta.uid() != uid || meta.mode() & 0o077 != 0 {
                return Err(ClientError::PrivateState);
            }
        }
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | if last { 0 } else { libc::O_DIRECTORY };
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(
                if missing_unavailable
                    && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound
                {
                    ClientError::Unavailable
                } else {
                    ClientError::PrivateState
                },
            );
        }
        let file = unsafe { File::from_raw_fd(fd) };
        let meta = file.metadata().map_err(|_| ClientError::PrivateState)?;
        if last {
            if !private_metadata_is_valid(&meta, uid) {
                return Err(ClientError::PrivateState);
            }
            return Ok(file);
        }
        if !meta.is_dir()
            || (meta.uid() != uid && meta.uid() != 0)
            || (meta.mode() & 0o022 != 0 && !(meta.uid() == 0 && meta.mode() & 0o1000 != 0))
        {
            return Err(ClientError::PrivateState);
        }
        directory = file;
    }
    Err(ClientError::PrivateState)
}
pub(crate) fn read_bounded(file: File, limit: usize) -> Result<Vec<u8>, ClientError> {
    if file
        .metadata()
        .map_err(|_| ClientError::PrivateState)?
        .len()
        > limit as u64
    {
        return Err(ClientError::PrivateState);
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ClientError::PrivateState)?;
    if bytes.len() > limit {
        return Err(ClientError::PrivateState);
    }
    Ok(bytes)
}

fn private_metadata_is_valid(meta: &std::fs::Metadata, uid: u32) -> bool {
    meta.is_file() && meta.uid() == uid && meta.mode() & 0o777 == 0o600 && meta.nlink() == 1
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};
    #[test]
    fn opened_descriptor_survives_filename_replacement_without_reading_new_source() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().canonicalize().unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        let path = parent.join("server.json");
        fs::write(&path, b"original").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let file = open_private_file_for_descriptor(&path).unwrap();
        let replacement = parent.join("replacement");
        fs::write(&replacement, b"replacement").unwrap();
        fs::set_permissions(&replacement, fs::Permissions::from_mode(0o600)).unwrap();
        fs::rename(&replacement, &path).unwrap();
        assert_eq!(read_bounded(file, 64).unwrap(), b"original");
    }
    #[test]
    fn wrong_owner_and_hardlinks_are_rejected_by_fd_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().canonicalize().unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        let path = parent.join("server.json");
        fs::write(&path, b"data").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let meta = fs::metadata(&path).unwrap();
        assert!(private_metadata_is_valid(&meta, meta.uid()));
        assert!(!private_metadata_is_valid(
            &meta,
            meta.uid().wrapping_add(1)
        ));
        fs::hard_link(&path, parent.join("linked")).unwrap();
        assert!(open_private_file_for_descriptor(&path).is_err());
    }
}
