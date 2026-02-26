#![feature(new_range_api)]

use clap::Parser;
use crate::{entities::{messages::Messages, names::Names, codebase::Codebase}};

mod ast;
mod entities;
mod tokens;
mod lookahead_iter;

#[derive(Debug, Parser)]
struct CliArgs {
}

fn main() {
    let dir = std::env::current_dir().expect("Unable to get current directory");
    let messages = Messages::new();

    let mut codebase = Codebase::new();
    codebase.add_dir(&dir, messages.clone());

    let names = Names::new();
    codebase.parse_all(names.clone(), messages.clone());

    messages.release(&codebase, |msg| println!("{}", msg));
    let (errors, warnings) = messages.counts();
    println!("Finished with {errors} errors and {warnings} warnings");
}
