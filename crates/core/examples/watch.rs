//! Prints every relay event: `cargo run -p rela-core --example watch -- ws://127.0.0.1:4534/ user token salt [secs] [follow]`
use rela_core::sync::{SyncClient, SyncEvent, SyncListener, SyncSnapshot};
use std::sync::Arc;

struct Print;
impl SyncListener for Print {
    fn on_event(&self, event: SyncEvent, snapshot: SyncSnapshot) {
        println!("{event:?} {snapshot:?}");
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 4 {
        eprintln!("usage: watch <relay ws url> <user> <token> <salt> [seconds] [user to follow]");
        std::process::exit(2);
    }
    let c = SyncClient::new(false, false, Arc::new(Print));
    c.start(a[0].clone(), a[1].clone(), a[2].clone(), a[3].clone());
    if let Some(who) = a.get(5) {
        std::thread::sleep(std::time::Duration::from_secs(1));
        c.follow(who);
    }
    std::thread::sleep(std::time::Duration::from_secs(a.get(4).and_then(|s| s.parse().ok()).unwrap_or(10)));
    c.stop(true);
}
