
#![crate_name = "patrol"]
#![crate_type = "lib"]

extern crate inotify;

use std::collections::{HashSet, HashMap};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use inotify::INotify;
use inotify::ffi::*;


#[derive(Debug)]
pub struct Target {
    pub path: PathBuf,
    pub is_file: bool,
}


impl Target {
    pub fn new(path: &str) -> Target {
        let path = PathBuf::from(path);
        let is_file = path.is_file();
        Target { path: path, is_file: is_file }
    }

    fn watch_path(&self) -> PathBuf {
        if self.is_file {
            self.path.parent().expect("Invalid path found").to_path_buf()
        } else {
            self.path.clone()
        }
    }
}


const EVENTS: u32 = IN_CREATE | IN_MODIFY | IN_DELETE;


pub fn make_targets(targets: &[&str]) -> Vec<Target> {
    targets.iter().map(|it| Target::new(it)).collect()
}


pub fn start(targets: Vec<Target>, sender: Sender<()>) {
    let mut ino = INotify::init().unwrap();

    let mut watched: HashMap<PathBuf, i32> = HashMap::new();

    let mut directories: HashSet<i32> = HashSet::new();
    let mut files: HashMap<i32, HashSet<String>> = HashMap::new();

    for target in targets {
        let watch_path = target.watch_path();
        let wd = watched.entry(watch_path.clone()).or_insert_with(|| {
            ino.add_watch(watch_path.as_path(), EVENTS).unwrap()
        });
        if target.is_file {
            let mut files = files.entry(*wd).or_insert_with(|| {
                HashSet::new()
            });
            files.insert(target.path.file_name().unwrap().to_str().unwrap().to_string());
        } else {
            directories.insert(*wd);
        }
    }

    loop {
        let events = ino.wait_for_events().unwrap();

        for event in events.iter() {
            if !event.is_dir() {
                let wd = event.wd;
                if directories.contains(&wd) {
                    sender.send(()).unwrap();
                } else if let Some(files) = files.get_mut(&wd) {
                    if files.contains(event.name.to_str().unwrap()) {
                        sender.send(()).unwrap();
                    }
                }
            }
        }
    }
}
