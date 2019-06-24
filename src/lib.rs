
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread;

use inotify::{Inotify, WatchDescriptor as WD, WatchMask, EventMask};

mod errors;

use errors::{PatrolError as PE, PatrolResultU};



#[derive(Debug, Clone)]
pub struct Target<T: Send + Clone> {
    pub path: PathBuf,
    pub is_file: bool,
    pub data: T,
}

pub type TargetU = Target<()>;

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
#[derive(Debug)]
pub struct Event<T: Send + Clone> {
    pub data: T,
    pub path: PathBuf,
}

pub struct Patrol<T: Send + Clone> {
    config: Config,
    targets: Vec<Target<T>>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Config {
    pub watch_new_directory: bool,
}


impl<T: Send + Clone + 'static> Patrol<T> {
    pub fn new(config: Config, targets: Vec<Target<T>>) -> Self {
        Self { config, targets }
    }

    pub fn spawn(self) -> Receiver<Event<T>> {
        let (tx, rx) = channel();
        thread::spawn(move || self.start(&tx));
        rx
    }

    pub fn start(self, sender: &Sender<Event<T>>) -> PatrolResultU {
        let target_events: WatchMask  = WatchMask::CREATE | WatchMask::MODIFY | WatchMask::DELETE;

        let mut ino = Inotify::init()?;

        let mut watched  = HashMap::<Rc<PathBuf>, Rc<WD>>::new();
        let mut wd_to_path  = HashMap::<Rc<WD>, Rc<PathBuf>>::new();

        let mut directories = HashMap::<Rc<WD>, &T>::new();
        let mut files = HashMap::<Rc<WD>, HashMap<&OsStr, &T>>::new();

        for target in &self.targets {
            let watching_path = Rc::new(target.watching_path().to_path_buf());
            if !watched.contains_key(&*watching_path) {
                let wd = Rc::new(ino.add_watch(&*watching_path, target_events)?);
                wd_to_path.insert(wd.clone(), watching_path.clone());
                watched.insert(watching_path, wd);
            }
        }

        for target in &self.targets {
            let watching_path = target.watching_path().to_path_buf();
            let wd: Rc<WD> = watched.get(&watching_path).expect(concat!("BUG@", line!())).clone();
            if target.is_file {
                let files = files.entry(wd).or_insert_with(|| HashMap::new());
                files.insert(
                    target.path.file_name().ok_or(PE::NoFilename)?,
                    &target.data);
            } else {
                directories.insert(wd, &target.data);
            }
        }

        loop {
            let mut buffer = [0; 1024];
            let events = ino.read_events_blocking(&mut buffer)?;

            for event in events {
                let event: inotify::Event<_> = event;
                if let Some(name) = event.name {
                    let wd = event.wd;

                    let data: Option<&T> = directories.get(&wd).or_else(|| files.get(&wd).and_then(|it| it.get(name))).cloned();

                    if let Some(data) = data {
                        let mut path = wd_to_path.get(&wd).expect(concat!("BUG@", line!())).to_path_buf();
                        path.push(name);

                        if self.config.watch_new_directory && event.mask == EventMask::CREATE | EventMask::ISDIR {
                            if !watched.contains_key(&path) {
                                let wd = Rc::new(ino.add_watch(&path, target_events)?);
                                let path = Rc::new(path.clone());

                                watched.insert(path.clone(), wd.clone());
                                wd_to_path.insert(wd.clone(), path);
                                directories.insert(wd, data);
                            }
                        }

                        sender.send(Event { data: (*data).clone(), path })?;
                    }
                }
            }
        }
    }
}
