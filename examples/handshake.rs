//! Quick end-to-end check that the driver dials criome,
//! exchanges a Handshake, and surfaces engine events.
//!
//! Run criome-daemon first, then:
//!
//! ```text
//! cargo run --example handshake
//! ```

use mentci_lib::connection::driver::{spawn_driver, DaemonRole, DriverCmd};
use std::path::PathBuf;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let socket_path: PathBuf = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/criome.sock".into())
        .into();

    let runtime_handle = tokio::runtime::Handle::current();
    let mut handle = spawn_driver(&runtime_handle, socket_path, DaemonRole::Criome);

    // Drain events for ~2s, printing each.
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(2);
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => break,
            ev = handle.events_rx.recv() => match ev {
                None => break,
                Some(ev) => println!("event: {ev:?}"),
            }
        }
    }

    // Cleanly disconnect.
    let _ = handle.cmds_tx.send(DriverCmd::Disconnect);
    // Give the driver a moment to send its goodbye event.
    let timeout = tokio::time::Instant::now() + tokio::time::Duration::from_millis(200);
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(timeout) => break,
            ev = handle.events_rx.recv() => match ev {
                None => break,
                Some(ev) => println!("event: {ev:?}"),
            }
        }
    }
}
