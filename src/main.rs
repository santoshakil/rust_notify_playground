use std::{path::Path, time::Duration};

use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent};

fn main() {
    let (sender, receiver) = std::sync::mpsc::channel::<DebouncedEvent>();
    let debouncer = new_debouncer(Duration::from_secs(1), None, move |events| {
        println!("Debouncer: {:?}\n", events);
        if let Ok(events) = events {
            for event in events {
                _ = sender.send(event);
            }
        }
    });
    if let Ok(mut debouncer) = debouncer {
        let path = Path::new("demo");
        _ = debouncer.watcher().watch(path, RecursiveMode::Recursive);
        loop {
            let event = receiver.recv();
            if let Ok(event) = event {
                println!("Event: {:?}\n", event);
            }
        }
    }
}
