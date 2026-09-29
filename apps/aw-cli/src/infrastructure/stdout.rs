//! Owns a CLOEXEC stdout duplicate. Pipe/socket/TTY writes use readiness, never a worker task.
use std::{
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{unix::AsyncFd, AsyncWrite};
enum Descriptor {
    Ready(OwnedFd),
    Poll(AsyncFd<OwnedFd>),
}
pub struct StdoutWriter {
    descriptor: Descriptor,
    // This duplicate remains valid even if AsyncFd registration consumes/closes its FD.
    _flags: Option<FlagLease>,
}
struct FlagLease {
    fd: OwnedFd,
    original: i32,
}
impl Drop for FlagLease {
    fn drop(&mut self) {
        // Restore the shared open-file-description, including the caller's original mode.
        // The owned restoration FD cannot be closed/reused by the caller.
        loop {
            if unsafe { libc::fcntl(self.fd.as_raw_fd(), libc::F_SETFL, self.original) } == 0 {
                break;
            }
            if io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
                break;
            }
        }
    }
}
fn duplicate_owned(fd: RawFd) -> io::Result<OwnedFd> {
    let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 3) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(duplicate) })
}
impl StdoutWriter {
    pub fn stdout() -> io::Result<Self> {
        Self::duplicate(libc::STDOUT_FILENO)
    }
    /// Duplicates an output descriptor; the caller retains its original ownership.
    pub fn duplicate(fd: RawFd) -> io::Result<Self> {
        Self::duplicate_registered(fd, AsyncFd::new)
    }
    fn duplicate_registered(
        fd: RawFd,
        register: impl FnOnce(OwnedFd) -> io::Result<AsyncFd<OwnedFd>>,
    ) -> io::Result<Self> {
        let duplicate = duplicate_owned(fd)?;
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe { libc::fstat(duplicate.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let stat = unsafe { stat.assume_init() };
        let kind = stat.st_mode & libc::S_IFMT;
        let is_null = if kind == libc::S_IFCHR {
            use std::os::unix::fs::MetadataExt;
            let null = std::fs::metadata("/dev/null")?;
            (null.mode() & libc::S_IFMT as u32) == libc::S_IFCHR as u32
                && null.rdev() == stat.st_rdev as u64
        } else {
            false
        };
        if kind == libc::S_IFREG || is_null {
            return Ok(Self {
                descriptor: Descriptor::Ready(duplicate),
                _flags: None,
            });
        }
        // Unknown character devices may block in arbitrary device-specific ways.
        if kind == libc::S_IFCHR && unsafe { libc::isatty(duplicate.as_raw_fd()) } != 1 {
            return Err(io::ErrorKind::Unsupported.into());
        }
        let original = unsafe { libc::fcntl(duplicate.as_raw_fd(), libc::F_GETFL) };
        if original < 0 {
            return Err(io::Error::last_os_error());
        }
        let lease = FlagLease {
            fd: duplicate,
            original,
        };
        if unsafe {
            libc::fcntl(
                lease.fd.as_raw_fd(),
                libc::F_SETFL,
                original | libc::O_NONBLOCK,
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }
        let descriptor = register(duplicate_owned(lease.fd.as_raw_fd())?)?;
        Ok(Self {
            descriptor: Descriptor::Poll(descriptor),
            _flags: Some(lease),
        })
    }
}
fn write(fd: RawFd, bytes: &[u8]) -> io::Result<usize> {
    // Cap the bytes per syscall. Regular-file syscall duration is not bounded by this cap.
    let count = unsafe { libc::write(fd, bytes.as_ptr().cast(), bytes.len().min(64 * 1024)) };
    if count < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(count as usize)
    }
}
impl AsyncWrite for StdoutWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        if bytes.is_empty() {
            return Poll::Ready(Ok(0));
        }
        match &self.descriptor {
            Descriptor::Ready(fd) => Poll::Ready(write(fd.as_raw_fd(), bytes)),
            Descriptor::Poll(fd) => loop {
                let mut ready = match fd.poll_write_ready(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                    Poll::Ready(Ok(ready)) => ready,
                };
                match ready.try_io(|fd| write(fd.as_raw_fd(), bytes)) {
                    Ok(result) => return Poll::Ready(result),
                    Err(_) => continue,
                }
            },
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;
    fn pipe() -> (OwnedFd, OwnedFd) {
        let mut pair = [-1; 2];
        assert_eq!(unsafe { libc::pipe(pair.as_mut_ptr()) }, 0);
        (unsafe { OwnedFd::from_raw_fd(pair[0]) }, unsafe {
            OwnedFd::from_raw_fd(pair[1])
        })
    }
    fn flags(fd: &OwnedFd) -> i32 {
        let flags = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) };
        assert!(flags >= 0);
        flags
    }
    #[tokio::test]
    async fn normal_drop_restores_original_shared_description_flags() {
        for originally_nonblocking in [false, true] {
            let (_read, original) = pipe();
            // Darwin F_GETFL also exposes kernel FWASWRITTEN, which F_SETFL cannot clear.
            // Establish it before capturing the original flags; all flags are then exact-compared.
            assert_eq!(
                unsafe { libc::write(original.as_raw_fd(), b"b".as_ptr().cast(), 1) },
                1
            );
            if originally_nonblocking {
                assert_eq!(
                    unsafe {
                        libc::fcntl(
                            original.as_raw_fd(),
                            libc::F_SETFL,
                            flags(&original) | libc::O_NONBLOCK,
                        )
                    },
                    0
                );
            }
            let before = flags(&original);
            let mut writer = StdoutWriter::duplicate(original.as_raw_fd()).unwrap();
            assert_ne!(flags(&original) & libc::O_NONBLOCK, 0);
            writer.write_all(b"complete\n").await.unwrap();
            drop(writer);
            assert_eq!(flags(&original), before);
        }
    }
    #[tokio::test]
    async fn cancelled_owned_pending_pipe_writer_restores_original_shared_flags() {
        let (_read, original) = pipe();
        assert_eq!(
            unsafe { libc::write(original.as_raw_fd(), b"b".as_ptr().cast(), 1) },
            1
        );
        let before = flags(&original);
        let mut writer = StdoutWriter::duplicate(original.as_raw_fd()).unwrap();
        let bytes = vec![b'x'; 64 * 1024];
        loop {
            let count =
                unsafe { libc::write(original.as_raw_fd(), bytes.as_ptr().cast(), bytes.len()) };
            if count < 0 {
                assert_eq!(io::Error::last_os_error().kind(), io::ErrorKind::WouldBlock);
                break;
            }
        }
        let entered = std::sync::Arc::new(tokio::sync::Notify::new());
        let ready = entered.clone();
        let job = tokio::spawn(async move {
            std::future::poll_fn(|cx| {
                let result = Pin::new(&mut writer).poll_write(cx, b"pending");
                if result.is_pending() {
                    ready.notify_one();
                }
                result
            })
            .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(1), entered.notified())
            .await
            .unwrap();
        job.abort();
        assert!(tokio::time::timeout(std::time::Duration::from_secs(1), job)
            .await
            .unwrap()
            .unwrap_err()
            .is_cancelled());
        assert_eq!(flags(&original), before);
    }
    #[tokio::test]
    async fn registration_failure_restores_flags_after_nonblocking_was_set() {
        let (_read, original) = pipe();
        let before = flags(&original);
        let result = StdoutWriter::duplicate_registered(original.as_raw_fd(), |fd| {
            assert_ne!(flags(&original) & libc::O_NONBLOCK, 0);
            drop(fd); // AsyncFd::new may consume and close this FD on failure.
            Err(io::ErrorKind::Unsupported.into())
        });
        assert!(result.is_err());
        assert_eq!(flags(&original), before);
    }

    #[tokio::test]
    async fn dev_null_writes_without_registration_or_changing_shared_flags() {
        let original: OwnedFd = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .unwrap()
            .into();
        assert_eq!(
            unsafe { libc::write(original.as_raw_fd(), b"b".as_ptr().cast(), 1) },
            1
        );
        let before = flags(&original);
        let mut writer = StdoutWriter::duplicate_registered(original.as_raw_fd(), |_| {
            panic!("null must not be registered in kqueue")
        })
        .unwrap();
        writer
            .write_all(b"discarded complete output\n")
            .await
            .unwrap();
        drop(writer);
        assert_eq!(flags(&original), before);
    }
    #[tokio::test]
    async fn unsupported_character_device_is_rejected_without_blocking_fallback_or_flag_change() {
        let original: OwnedFd = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/zero")
            .unwrap()
            .into();
        let before = flags(&original);
        assert!(
            StdoutWriter::duplicate_registered(original.as_raw_fd(), |_| panic!(
                "unsupported character device must not register"
            ))
            .is_err()
        );
        assert_eq!(flags(&original), before);
    }
}
