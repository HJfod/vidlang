#![feature(new_range_api)]

use clap::Parser;
use crate::{ast::expr::ParseArgs, pools::{codebase::{Codebase, CodebaseCreateError}, exprs::Exprs, messages::Messages, names::Names}};

mod ast;
mod pools;
mod tokens;
mod utils;
mod check;

#[derive(Debug, Parser)]
struct CliArgs {
}

fn main() {
    let dir = std::env::current_dir().expect("Unable to get current directory");
    
    let messages = Messages::new();
    let names = Names::new();
    let exprs = Exprs::new();
    let mut codebase = match Codebase::from_dir(&dir) {
        Ok(c) => c,
        Err(e) => match e {
            CodebaseCreateError::CantFindRoot => panic!("unable to find {}/main.vid!", dir.display()),
            CodebaseCreateError::UnableToReadFile(p, e) => panic!("unable to read file {}: {e}", p.display()),
            CodebaseCreateError::UnableToReadDir(p, e) => panic!("unable to read directory {}: {e}", p.display()),
            CodebaseCreateError::DuplicateNamedModule(e) => panic!("multiple modules with the same name found: {e}"),
        }
    };

    codebase.parse_all(names.clone(), messages.clone(), exprs.clone(), ParseArgs::default());

    messages.release(&codebase, |msg| println!("{}", msg));
    let (errors, warnings) = messages.counts();
    println!("Finished with {errors} errors and {warnings} warnings");
}
