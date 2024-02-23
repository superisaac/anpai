#![feature(assert_matches)]

use clap::Parser;

mod ast;
mod scan;

mod parse;

mod value;

mod eval;

mod helpers;

mod commands;

fn main() {
    let cmd = commands::FEELCommands::parse();
    match cmd.execute() {
        Err(err) => panic!("{}", err),
        _ => (),
    }
}
