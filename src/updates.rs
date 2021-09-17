extern crate notify;
use crate::log;
use notify::{watcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

pub struct Updates {}

impl Updates {
    pub fn local(path: String) {
        if !Path::new(&path).is_dir() {
            log::error(format!("Root directory {:?} does not exist", path));
            std::process::exit(1);
        }
        // Create a channel to receive the events.
        let (tx, rx) = channel();

        // Create a watcher object, delivering debounced events.
        // The notification back-end is selected based on the platform.
        let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher.watch(path, RecursiveMode::Recursive).unwrap();

        loop {
            match rx.recv() {
                Ok(event) => println!("{:?}", event),
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    }
}
