//! Read boundaries and partial-message deadline only. Tungstenite validates RFC6455.
//! Capping reads at frame boundaries prevents a completed message from clearing a
//! deadline for an already-buffered partial next message. Idle sockets have no timer.
use std::future::Future;
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    time::Sleep,
};
#[derive(Debug, thiserror::Error)]
#[error("frame quota exceeded")]
pub(super) struct FrameQuota;
pub(super) struct FrameIo<S> {
    inner: S,
    deadline: Duration,
    maximum: usize,
    timer: Option<Pin<Box<Sleep>>>,
    header: [u8; 14],
    read: usize,
    needed: usize,
    payload: u64,
    fragmented: bool,
    finishes_fragment: bool,
}
impl<S> FrameIo<S> {
    pub(super) fn new(inner: S, deadline: Duration, maximum: usize) -> Self {
        Self {
            inner,
            deadline,
            maximum,
            timer: None,
            header: [0; 14],
            read: 0,
            needed: 2,
            payload: 0,
            fragmented: false,
            finishes_fragment: false,
        }
    }
    pub(super) fn message_complete(&mut self) {
        if !self.fragmented {
            self.timer = None;
        }
    }
    fn frame_complete(&mut self) {
        if self.finishes_fragment {
            self.fragmented = false;
        }
        self.read = 0;
        self.needed = 2;
        self.finishes_fragment = false;
    }
    fn header_complete(&mut self) -> io::Result<()> {
        let code = self.header[1] & 127;
        let length = match code {
            126 => u16::from_be_bytes(self.header[2..4].try_into().unwrap()) as u64,
            127 => u64::from_be_bytes(self.header[2..10].try_into().unwrap()),
            n => n as u64,
        };
        if length > self.maximum as u64 {
            return Err(io::Error::other(FrameQuota));
        }
        self.payload = length;
        let opcode = self.header[0] & 15;
        let fin = self.header[0] & 128 != 0;
        if matches!(opcode, 1 | 2) && !fin {
            self.fragmented = true;
        }
        self.finishes_fragment = opcode == 0 && fin;
        if length == 0 {
            self.frame_complete();
        }
        Ok(())
    }
}
impl<S: AsyncRead + Unpin> AsyncRead for FrameIo<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        if let Some(timer) = &mut this.timer {
            if timer.as_mut().poll(cx).is_ready() {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "partial message deadline",
                )));
            }
        }
        let header = this.payload == 0;
        let cap = if header {
            this.needed - this.read
        } else {
            this.payload.min(usize::MAX as u64) as usize
        }
        .min(buf.remaining());
        let mut limited = ReadBuf::new(buf.initialize_unfilled_to(cap));
        match Pin::new(&mut this.inner).poll_read(cx, &mut limited) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(())) => {
                let n = limited.filled().len();
                if n > 0 {
                    if this.timer.is_none() {
                        let mut timer = Box::pin(tokio::time::sleep(this.deadline));
                        let _ = timer.as_mut().poll(cx);
                        this.timer = Some(timer);
                    }
                    if header {
                        this.header[this.read..this.read + n].copy_from_slice(limited.filled());
                        this.read += n;
                        if this.read == 2 && this.needed == 2 {
                            this.needed =
                                2 + match this.header[1] & 127 {
                                    126 => 2,
                                    127 => 8,
                                    _ => 0,
                                } + if this.header[1] & 128 != 0 { 4 } else { 0 };
                        }
                        if this.read == this.needed {
                            if let Err(e) = this.header_complete() {
                                return Poll::Ready(Err(e));
                            }
                        }
                    } else {
                        this.payload -= n as u64;
                        if this.payload == 0 {
                            this.frame_complete();
                        }
                    }
                }
                buf.advance(n);
                Poll::Ready(Ok(()))
            }
        }
    }
}
impl<S: AsyncWrite + Unpin> AsyncWrite for FrameIo<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}
