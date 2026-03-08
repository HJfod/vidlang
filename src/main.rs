#![feature(new_range_api)]

use std::path::Path;

use clap::Parser;
use crate::{
    ast::expr::ParseArgs, 
    codebase::{AddPackageError, Codebase}, pools::modules::AddModuleError,
};

mod ast;
mod pools;
mod tokens;
mod utils;
mod check;
mod codebase;

#[derive(Debug, Parser)]
struct CliArgs {
}

fn add_pkg_or_panic(codebase: &mut Codebase, path: &Path) {
    match codebase.add_package(path) {
        Ok(_) => (),
        Err(e) => match e {
            AddPackageError::NoVidToml => panic!("{}: missing vid.toml", path.display()),
            AddPackageError::CantReadVidToml(e) => panic!("{}: can't read vid.toml: {e}", path.display()),
            AddPackageError::BadVidToml(e) => panic!("{}: bad vid.toml: {e}", path.display()),
            AddPackageError::DuplicateName(e) => panic!("{}: multiple packages with the same name found: {e}", path.display()),
            AddPackageError::ModuleError(e) => match e {
                AddModuleError::UnableToReadFile(p, e) => panic!("unable to read file {}: {e}", p.display()),
                AddModuleError::UnableToReadDir(p, e) => panic!("unable to read directory {}: {e}", p.display()),
            }
        }
    }
}

fn main() {
    let dir = std::env::current_dir().expect("Unable to get current directory");
    
    let mut codebase = Codebase::new();
    add_pkg_or_panic(&mut codebase, &dir.join("std"));
    add_pkg_or_panic(&mut codebase, &dir.join("examples"));
    codebase.parse_all(ParseArgs::default());

    codebase.messages.release(&codebase, |msg| println!("{}", msg));
    let (errors, warnings) = codebase.messages.counts();
    println!("Finished with {errors} errors and {warnings} warnings");
}

#[test]
fn compile_examples() {
    let dir = std::env::current_dir().expect("Unable to get current directory");
    
    let mut codebase = Codebase::new();
    add_pkg_or_panic(&mut codebase, &dir.join("std"));
    add_pkg_or_panic(&mut codebase, &dir.join("examples"));
    codebase.parse_all(ParseArgs::default());

    assert!(codebase.messages.count_total() == 0, "{:?}", codebase.messages);
}
