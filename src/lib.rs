
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;

use inotify::{Inotify, WatchDescriptor as WD, WatchMask};

mod errors;

use errors::{PatrolError as PE, PatrolResultU};



#[derive(Debug, Clone)]
pub struct Target<T: Send + Clone> {
    pub path: PathBuf,
    pub is_file: bool,
    pub data: T,
}


impl<T: Send + Clone> Target<T> {
    pub fn new(path: PathBuf, data: T) -> Target<T> {
        let is_file = path.is_file();
        Target { path, is_file, data }
    }

    fn watching_path(&self) -> &Path {
        if self.is_file {
            self.path.parent().expect("Invalid path found")
        } else {
            self.path.as_ref()
        }
    }
}


#[allow(dead_code)]
pub struct Event<T: Send + Clone> {
    pub data: T
}





pub fn spawn<T: Send + Clone + 'static>(targets: Vec<Target<T>>) -> Receiver<Event<T>> {
    let (tx, rx) = channel();
    thread::spawn(move || start(&targets, &tx));
    rx
}


pub fn start<T: Send + Clone>(targets: &[Target<T>], sender: &Sender<Event<T>>) -> PatrolResultU {
    let target_events: WatchMask  = WatchMask::CREATE | WatchMask::MODIFY | WatchMask::DELETE;

    let mut ino = Inotify::init()?;

    let mut watched  = HashMap::<PathBuf, WD>::new();

    let mut directories = HashMap::<WD, &T>::new();
    let mut files = HashMap::<WD, HashMap<String, &T>>::new();

    for target in targets {
        let watching_path = target.watching_path().to_path_buf();
        let wd = watched.entry(watching_path.clone()).or_insert_with(|| {
            ino.add_watch(watching_path.as_path(), target_events).unwrap() // FIXME
        });
        if target.is_file {
            let files = files.entry(wd.clone()).or_insert_with(|| HashMap::new());
            files.insert(
                target.path.file_name().ok_or(PE::NoFilename)?.to_str().ok_or(PE::FilepathEncoding)?.to_string(),
                &target.data);
        } else {
            directories.insert(wd.clone(), &target.data);
        }
    }

    loop {
        let mut buffer = [0; 1024];
        let events = ino.read_events_blocking(&mut buffer)?;

        for event in events {
            let event: inotify::Event<_> = event;
            if let Some(name) = event.name {
                let wd = event.wd;
                if let Some(data) = directories.get(&wd).cloned().cloned() {
                    sender.send(Event { data })?;
                } else if let Some(files) = files.get(&wd) {
                    if let Some(data) = files.get(name.to_str().ok_or(PE::FilepathEncoding)?) {
                        sender.send(Event { data: data.to_owned().to_owned() })?;
                    }
                }
            }
        }
    }
}
