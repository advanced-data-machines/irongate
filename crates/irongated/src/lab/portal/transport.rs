//! A total phase deadline checked on every underlying TLS read/write.
use rustls::{ServerConnection, Stream};
use std::{
    io::{self, Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};

pub(super) trait RequestStream: Read + Write {
    fn begin_response(&mut self) {}
}
pub(super) trait TimedIo: Read + Write {
    fn timeout(&self, read: bool, duration: Duration) -> io::Result<()>;
}
impl TimedIo for TcpStream {
    fn timeout(&self, read: bool, duration: Duration) -> io::Result<()> {
        if read {
            self.set_read_timeout(Some(duration))
        } else {
            self.set_write_timeout(Some(duration))
        }
    }
}
#[cfg(test)]
impl TimedIo for std::os::unix::net::UnixStream {
    fn timeout(&self, read: bool, duration: Duration) -> io::Result<()> {
        if read {
            self.set_read_timeout(Some(duration))
        } else {
            self.set_write_timeout(Some(duration))
        }
    }
}
pub(super) struct Socket<T: TimedIo = TcpStream> {
    stream: T,
    until: Instant,
}
impl<T: TimedIo> Socket<T> {
    pub fn new(stream: T, until: Instant) -> Self {
        Self { stream, until }
    }
    pub fn phase(&mut self) {
        self.until = Instant::now() + Duration::from_secs(2);
    }
    fn remaining(&self) -> io::Result<Duration> {
        self.until
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .map(|d| d.min(Duration::from_millis(200)))
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "portal phase deadline"))
    }
}
impl<T: TimedIo> Read for Socket<T> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.stream.timeout(true, self.remaining()?)?;
        self.stream.read(out)
    }
}
impl<T: TimedIo> Write for Socket<T> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.stream.timeout(false, self.remaining()?)?;
        self.stream.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}
impl<T: TimedIo> RequestStream for Socket<T> {
    fn begin_response(&mut self) {
        self.phase();
    }
}

pub(super) struct Secure<T: TimedIo = TcpStream> {
    pub conn: ServerConnection,
    pub socket: Socket<T>,
}
impl<T: TimedIo> Secure<T> {
    pub fn handshake(&mut self) -> io::Result<()> {
        self.conn.set_buffer_limit(Some(16384));
        while self.conn.is_handshaking() {
            self.socket.remaining()?;
            match self.conn.complete_io(&mut self.socket) {
                Ok(_) => (),
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock
                            | io::ErrorKind::TimedOut
                            | io::ErrorKind::Interrupted
                    ) =>
                {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        self.socket.phase();
        Ok(())
    }
}
impl<T: TimedIo> Read for Secure<T> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        Stream::new(&mut self.conn, &mut self.socket).read(out)
    }
}
impl<T: TimedIo> Write for Secure<T> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        Stream::new(&mut self.conn, &mut self.socket).write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        Stream::new(&mut self.conn, &mut self.socket).flush()
    }
}
impl<T: TimedIo> RequestStream for Secure<T> {
    fn begin_response(&mut self) {
        self.socket.phase();
    }
}
