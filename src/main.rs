extern crate termion;

use std::env;
use std::io;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::path::Path;

mod editor;
mod tests;
mod tty;
mod utils;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
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
