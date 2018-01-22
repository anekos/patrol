
#![crate_name = "patrol"]
#![crate_type = "lib"]

extern crate inotify;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;

use inotify::INotify;
use inotify::ffi::*;


#[derive(Debug)]
pub struct Target<T: Send + Clone> {
    pub path: PathBuf,
    pub is_file: bool,
    pub data: T,
}


impl<T: Send + Clone> Target<T> {
    pub fn new<P: AsRef<Path>>(path: &P, data: T) -> Target<T> {
        let path = path.as_ref().to_path_buf();
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
    data: T
}


const EVENTS: u32 = IN_CREATE | IN_MODIFY | IN_DELETE;


pub fn make_targets(targets: &[&str]) -> Vec<Target<()>> {
    targets.iter().map(|it| Target::new(it, ())).collect()
}


pub fn spawn<T: Send + Clone + 'static>(targets: Vec<Target<T>>) -> Receiver<Event<T>> {
    let (tx, rx) = channel();
    thread::spawn(move || start(&targets, &tx));
    rx
}


pub fn start<T: Send + Clone>(targets: &[Target<T>], sender: &Sender<Event<T>>) {
    let mut ino = INotify::init().unwrap();

    let mut watched  = HashMap::<PathBuf, i32>::new();

    let mut directories = HashMap::<i32, &T>::new();
    let mut files = HashMap::<i32, HashMap<String, &T>>::new();

    for target in targets {
        let watching_path = target.watching_path().to_path_buf();
        let wd = watched.entry(watching_path.clone()).or_insert_with(|| {
            ino.add_watch(watching_path.as_path(), EVENTS).unwrap()
        });
        if target.is_file {
            let mut files = files.entry(*wd).or_insert_with(|| {
                HashMap::new()
            });
            files.insert(
                target.path.file_name().unwrap().to_str().unwrap().to_string(),
                &target.data);
        } else {
            directories.insert(*wd, &target.data);
        }
    }

    loop {
        let events = ino.wait_for_events().unwrap();

        for event in events.iter() {
            if !event.is_dir() {
                let wd = event.wd;
                if let Some(data) = directories.get(&wd).cloned().cloned() {
                    sender.send(Event { data }).unwrap();
                } else if let Some(files) = files.get_mut(&wd) {
                    if let Some(data) = files.get(event.name.to_str().unwrap()).cloned().cloned() {
                        sender.send(Event { data }).unwrap();
                    }
                }
            }
        }
    }
}
