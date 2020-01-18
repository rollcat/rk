// Prevent editors from interpreting "#!" as a shebang and adding +x
// #![deny(warnings)]
#![deny(unused_imports)]
extern crate termion;

use std::boxed::Box;
use std::env;
use std::error::Error;
use std::io;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::path::Path;
use std::result::Result;

mod editor;
mod keys;
mod tests;
mod tty;
mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if !termion::is_tty(&io::stdin()) {
        return Err("Standard input is not a TTY.".into());
    }
    if !termion::is_tty(&io::stdout()) {
        return Err("Standard output is not a TTY.".into());
    }

    let t = tty::Terminal::new(io::stdin(), io::stdout())?;
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
    e.deinit()?;
    if let Err(err) = r {
        eprintln!("e: {:?}", e);
        resume_unwind(err);
    };
    Ok(())
}
