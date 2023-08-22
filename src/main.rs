use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use notify::{
    event::{ModifyKind, RemoveKind},
    Event, EventKind, RecursiveMode, Watcher,
};
use notify_debouncer_full::{new_debouncer, DebouncedEvent};

#[tokio::main]
async fn main() {
    let (sender, receiver) = std::sync::mpsc::channel::<FSEvent>();
    let debouncer = new_debouncer(
        Duration::from_secs(1),
        None,
        move |events: Result<Vec<DebouncedEvent>, Vec<notify::Error>>| {
            println!("Debouncer: {:?}\n", events);
            if let Ok(events) = events {
                _ = sender.send(events.to_fs_event());
            }
        },
    );
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

#[derive(Debug, PartialEq)]
pub enum FSEvent {
    Create(PathBuf),
    Modify(PathBuf),
    Delete(PathBuf),
    Rename(PathBuf, PathBuf),
    Move(PathBuf, PathBuf),
    Unknown,
}

impl FSEvent {
    pub fn is_unknown(&self) -> bool {
        match self {
            FSEvent::Unknown => true,
            _ => false,
        }
    }

    pub fn path(&self) -> Option<PathBuf> {
        match self {
            FSEvent::Create(path) => Some(path.clone()),
            FSEvent::Modify(path) => Some(path.clone()),
            FSEvent::Delete(path) => Some(path.clone()),
            FSEvent::Rename(path, _) => Some(path.clone()),
            FSEvent::Move(path, _) => Some(path.clone()),
            FSEvent::Unknown => None,
        }
    }

    pub fn path2(&self) -> Option<PathBuf> {
        match self {
            FSEvent::Rename(_, path) => Some(path.clone()),
            FSEvent::Move(_, path) => Some(path.clone()),
            _ => None,
        }
    }
}

pub trait EventExt {
    fn is_remove_any(&self) -> bool;
    fn should_ignore(&self) -> bool;
}

impl EventExt for Event {
    fn is_remove_any(&self) -> bool {
        match self.kind {
            EventKind::Remove(RemoveKind::Any) => true,
            _ => false,
        }
    }

    fn should_ignore(&self) -> bool {
        self.paths.is_empty() || self.paths.iter().any(|path| path.ends_with(".DS_Store"))
    }
}

pub trait EventsExt {
    fn to_fs_event(&self) -> FSEvent;
}

impl EventsExt for Vec<DebouncedEvent> {
    fn to_fs_event(&self) -> FSEvent {
        if self.is_empty() {
            return FSEvent::Unknown;
        }
        for event in self.iter() {
            if event.should_ignore() {
                continue;
            }
            let path = event.paths.first().unwrap();
            let remove_any = self.iter().any(|event| event.is_remove_any());
            match event.kind {
                EventKind::Create(_) if remove_any => {
                    let path1 = self.iter().find(|event| event.is_remove_any()).unwrap();
                    return FSEvent::Move(path1.paths.first().unwrap().clone(), path.clone());
                }
                EventKind::Create(_) => return FSEvent::Create(path.clone()),
                EventKind::Modify(ModifyKind::Data(_)) | EventKind::Modify(ModifyKind::Any) => {
                    return FSEvent::Modify(path.clone())
                }
                EventKind::Modify(ModifyKind::Name(_)) if event.paths.len() == 2 => {
                    if let Some(path2) = event.paths.last() {
                        if path.parent() == path2.parent() {
                            return FSEvent::Rename(path.clone(), path2.clone());
                        } else {
                            return FSEvent::Move(path.clone(), path2.clone());
                        }
                    }
                }
                EventKind::Modify(_) if !path.exists() => return FSEvent::Delete(path.clone()),
                _ => {}
            }
        }
        FSEvent::Unknown
    }
}
