
extern crate patrol;

use std::env;
use std::path::Path;

use patrol::*;



fn main() {
    let targets: Vec<Target<()>> = env::args().map(|it| Target::new(Path::new(&it).to_path_buf(), ())).collect();
    let rx = spawn(targets);

    for ev in rx {
        println!("{:?}", ev);
    }
}
