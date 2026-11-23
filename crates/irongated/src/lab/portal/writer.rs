//! Ordered, acknowledged journal writes; packet processing never waits for disk.
use super::journal::Record;
use std::{
    fs::{File, OpenOptions},
    io::{self, Seek, SeekFrom, Write},
    os::unix::{
        fs::{DirBuilderExt, OpenOptionsExt},
        net::UnixDatagram,
    },
    sync::mpsc::{Receiver, SyncSender},
};

pub(super) fn open() -> io::Result<File> {
    open_at(std::path::Path::new("/var/lib/irongate-lab"))
}

pub(super) fn open_at(directory: &std::path::Path) -> io::Result<File> {
    match std::fs::DirBuilder::new().mode(0o700).create(directory) {
        Ok(()) => File::open(
            directory
                .parent()
                .ok_or_else(|| io::Error::other("journal parent"))?,
        )?
        .sync_all()?,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
        Err(error) => return Err(error),
    }
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(directory.join("local-records.jsonl"))?;
    File::open(directory)?.sync_all()?;
    Ok(file)
}

pub(super) fn run(
    mut file: File,
    records: Receiver<Record>,
    acks: SyncSender<(usize, bool)>,
    wake: UnixDatagram,
    run_id: String,
) {
    let mut committed = 0;
    while let Ok(record) = records.recv() {
        let mut line=serde_json::to_vec(&serde_json::json!({"schema":1,"run_id":run_id,"id":record.id,
            "kind":format!("{:?}",record.kind),"at_monotonic_seconds":record.at,
            "network":record.session.network().0,"session_generation":record.session.generation(),
            "lease_generation":record.lease.generation(),"submitted_ip_bytes_total":record.input_output_ip_bytes})).expect("fixed JSON record");
        line.push(b'\n');
        let success = append(&mut file, &mut committed, &line).is_ok();
        if acks.send((record.id, success)).is_err() {
            break;
        }
        let _ = wake.send(&[1]);
    }
}
pub(super) fn append(file: &mut File, committed: &mut u64, line: &[u8]) -> io::Result<()> {
    let next = committed
        .checked_add(line.len() as u64)
        .ok_or_else(|| io::Error::other("record limit"))?;
    if next > 65536 {
        return Err(io::Error::other("record limit"));
    }
    // Remove an unacknowledged partial tail before retrying; crash recovery is
    // outside this fresh-run writer. Only a successfully synced prefix is acked.
    file.set_len(*committed)?;
    file.seek(SeekFrom::Start(*committed))?;
    file.write_all(line)?;
    file.sync_data()?;
    *committed = next;
    Ok(())
}
