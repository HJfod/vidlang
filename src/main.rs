#![feature(new_range_api)]

use clap::Parser;
use crate::{ast::expr::ParseArgs, pools::{codebase::{Codebase, PackageAddError}, exprs::Exprs, messages::Messages, names::Names}};

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
    
    let mut codebase = Codebase::new();
    match codebase.add_package("project".into(), &dir) {
        Ok(c) => c,
        Err(e) => match e {
            PackageAddError::UnableToReadFile(p, e) => panic!("unable to read file {}: {e}", p.display()),
            PackageAddError::UnableToReadDir(p, e) => panic!("unable to read directory {}: {e}", p.display()),
            PackageAddError::DuplicateNamedPackage(e) => panic!("multiple packages with the same name found: {e}"),
        }
    };
    codebase.parse_all(ParseArgs::default());

    codebase.messages.release(&codebase, |msg| println!("{}", msg));
    let (errors, warnings) = codebase.messages.counts();
    println!("Finished with {errors} errors and {warnings} warnings");
}
