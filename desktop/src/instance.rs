//! One running copy. The app lives on in the tray with its window closed, so starting it
//! again would put a second overlay and a second set of shortcuts next to the first. A
//! second start asks the running copy to show its window instead, and ends.
//!
//! The running copy listens on a loopback port it wrote next to the config file. A port
//! is the one thing all three platforms have without another dependency, and a greeting
//! in both directions tells a copy of this app from whatever else got the port of one
//! that is gone.

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

const KNOCK: &str = "wormholesystems-desktop show\n";
const ANSWER: &str = "wormholesystems-desktop here\n";

/// How long a start waits for the copy named in the port file. One that is alive answers
/// at once; this only bounds a port something else took over.
const PATIENCE: Duration = Duration::from_millis(500);

/// The running copy's side: what later starts ask of it.
#[derive(Debug)]
pub(crate) struct Instance {
    shows: Receiver<()>,
}

impl Instance {
    /// Whether another start asked for the window since the last call.
    pub(crate) fn show_requested(&self) -> bool {
        self.shows.try_iter().count() > 0
    }
}

fn port_file() -> Option<PathBuf> {
    crate::config::path().map(|config| config.with_file_name("desktop.port"))
}

/// Become the running copy, or `None` when there is one already; it has been asked to
/// show its window by then. Where the port cannot be set up, the app starts all the same
/// and later starts will not find it.
pub(crate) fn claim() -> Option<Instance> {
    match port_file() {
        Some(file) => claim_at(&file),
        None => Some(unreachable_instance()),
    }
}

/// Let the next start become the running copy while this one is still on its way out, as
/// when it restarts into an update.
pub(crate) fn release() {
    if let Some(file) = port_file() {
        let _ = std::fs::remove_file(file);
    }
}

fn claim_at(file: &Path) -> Option<Instance> {
    let running = std::fs::read_to_string(file)
        .ok()
        .and_then(|port| port.trim().parse::<u16>().ok());
    if running.is_some_and(knock) {
        return None;
    }
    Some(listen(file).unwrap_or_else(|e| {
        crate::log(format_args!("a second start will not find this one: {e}"));
        unreachable_instance()
    }))
}

fn listen(file: &Path) -> std::io::Result<Instance> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let port = listener.local_addr()?.port();
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(file, port.to_string())?;
    let (asked, shows) = mpsc::channel();
    std::thread::Builder::new()
        .name("instance".into())
        .spawn(move || serve(&listener, &asked))?;
    Ok(Instance { shows })
}

/// An instance nothing can reach.
fn unreachable_instance() -> Instance {
    Instance {
        shows: mpsc::channel().1,
    }
}

/// Ask the copy on `port` for its window. False when nothing answers there as this app.
fn knock(port: u16) -> bool {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, PATIENCE) else {
        return false;
    };
    if patient(&stream).is_err() || stream.write_all(KNOCK.as_bytes()).is_err() {
        return false;
    }
    line(&stream).is_some_and(|answer| answer == ANSWER)
}

fn serve(listener: &TcpListener, asked: &Sender<()>) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        if patient(&stream).is_err() || line(&stream).is_none_or(|knock| knock != KNOCK) {
            continue;
        }
        let _ = stream.write_all(ANSWER.as_bytes());
        if asked.send(()).is_err() {
            return;
        }
    }
}

fn patient(stream: &TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(PATIENCE))?;
    stream.set_write_timeout(Some(PATIENCE))
}

/// One line from the other side, which may be anything at all: no longer than a greeting.
fn line(stream: &TcpStream) -> Option<String> {
    let longest = u64::try_from(KNOCK.len().max(ANSWER.len())).ok()?;
    let mut line = String::new();
    BufReader::new(stream.take(longest))
        .read_line(&mut line)
        .ok()?;
    Some(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_start_asks_the_first_for_its_window() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("wormholesystems").join("desktop.port");

        let first = claim_at(&file).expect("nothing is running yet");
        assert!(!first.show_requested());

        assert!(claim_at(&file).is_none(), "the first copy is running");
        assert!(first.show_requested());
        assert!(!first.show_requested(), "asked once, shown once");
    }

    #[test]
    fn a_port_left_by_a_copy_that_is_gone_does_not_block_the_start() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("desktop.port");
        let gone = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        std::fs::write(&file, gone.local_addr().unwrap().port().to_string()).unwrap();
        drop(gone);

        assert!(claim_at(&file).is_some());
    }

    #[test]
    fn something_else_on_the_port_is_not_taken_for_the_app() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("desktop.port");
        let other = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        std::fs::write(&file, other.local_addr().unwrap().port().to_string()).unwrap();
        let answering = std::thread::spawn(move || {
            let (mut stream, _) = other.accept().unwrap();
            let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n");
        });

        assert!(claim_at(&file).is_some());
        answering.join().unwrap();
    }
}
