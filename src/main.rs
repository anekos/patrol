
extern crate patrol;

use std::env;
use std::path::Path;

use patrol::{Config, Patrol, Target};



fn main() {
    let targets: Vec<Target<()>> = env::args().map(|it| Target::new(Path::new(&it).to_path_buf(), ())).collect();
    let patrol = Patrol::new(Config { watch_new_directory: true }, targets);
    let rx = patrol.spawn();

    for ev in rx {
        println!("{:?}", ev);
    }
}
