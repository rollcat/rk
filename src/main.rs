extern crate libc;
extern crate regex;
extern crate termios;

use std::env;
use std::io;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::path::Path;

mod editor;
mod keys;
mod tty;
mod utils;

fn main() {
    let args: Vec<String> = env::args().collect();
    let t = tty::Terminal::new(io::stdin(), io::stdout());
    let mut e = editor::Editor::new(t);

    let r = catch_unwind(AssertUnwindSafe(|| {
        if args.len() == 2 {
            let path = Path::new(&args[1]);
            e.open(path).expect("open");
        }
        e.init().unwrap();
        loop {
            match e.update().unwrap() {
                Some(editor::Exit) => break,
                None => continue,
            }
        }
    }));
    e.deinit().unwrap();
    eprintln!("e: {:?}", e);
    if let Err(err) = r {
        resume_unwind(err);
    };
}
