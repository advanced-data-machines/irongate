//! Descriptor ownership and readiness, independent of guest topology setup.

use super::profile;
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use std::{
    fs::File,
    io,
    os::{
        fd::AsFd,
        unix::{fs::FileTypeExt, net::UnixDatagram},
    },
};

pub(super) struct Transport {
    pub down: File,
    pub up: File,
    pub control: UnixDatagram,
    pub portal: Option<super::portal::Bridge>,
}

impl Transport {
    pub fn inherited() -> io::Result<Self> {
        // Called only after the owned-guest guard succeeds. No unsafe FD creation.
        let down = File::from(io::stdin().as_fd().try_clone_to_owned()?);
        let up = File::from(io::stdout().as_fd().try_clone_to_owned()?);
        Self::new(down, up, UnixDatagram::bind(profile::CONTROL)?)
    }

    pub fn new(down: File, up: File, control: UnixDatagram) -> io::Result<Self> {
        for packet in [&down, &up] {
            if !packet.metadata()?.file_type().is_socket() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "lab requires inherited packet sockets",
                ));
            }
        }
        control.set_nonblocking(true)?;
        Ok(Self {
            down,
            up,
            control,
            portal: None,
        })
    }

    pub fn wait(&self) -> io::Result<()> {
        let mut descriptors = [
            PollFd::new(&self.down, PollFlags::IN),
            PollFd::new(&self.up, PollFlags::IN),
            PollFd::new(&self.control, PollFlags::IN),
            PollFd::new(
                self.portal
                    .as_ref()
                    .map(|p| &p.wake)
                    .unwrap_or(&self.control),
                PollFlags::IN,
            ),
        ];
        let timeout = Timespec {
            tv_sec: 0,
            tv_nsec: 100_000_000,
        };
        match poll(&mut descriptors, Some(&timeout)) {
            Ok(_) | Err(rustix::io::Errno::INTR) => (),
            Err(error) => return Err(error.into()),
        }
        if descriptors.iter().any(|fd| {
            fd.revents()
                .intersects(PollFlags::ERR | PollFlags::HUP | PollFlags::NVAL)
        }) {
            return Err(io::Error::other(
                "lab descriptor failed; forwarding stopped",
            ));
        }
        Ok(())
    }
}
