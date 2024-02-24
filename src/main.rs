#![feature(assert_matches)]

use clap::Parser;
use fileinput::FileInput;
use std::io::{BufRead, BufReader};

mod ast;
mod scan;

mod parse;

mod value;

mod eval;

mod helpers;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct FEELArgs {
    #[arg(short, long)]
    ast: bool,

    #[arg(short, long)]
    script: Option<String>,

    files: Vec<String>,
}

impl FEELArgs {
    fn execute(&self) -> Result<(), eval::EvalError> {
        let mut intp = eval::Intepreter::new();
        if let Some(script) = self.script.clone() {
            let n = parse::parse(script.as_str())?;
            if self.ast {
                println!("{}", n);
            } else {
                let res = intp.eval(n)?;
                println!("{}", res);
            }
        } else {
            let filenames: Vec<&str> = self.files.iter().map(|s| s.as_str()).collect();
            let fileinput = FileInput::new(&filenames);
            let reader = BufReader::new(fileinput);

            // read all contents from either files or stdin
            let mut buf: String = String::new();
            for res in reader.lines() {
                let line = res.unwrap();
                buf.push_str(line.as_str());
                buf.push_str("\n");
            }
            let n = parse::parse(buf.as_str())?;
            if self.ast {
                println!("{}", n);
            } else {
                let res = intp.eval(n)?;
                println!("{}", res);
            }
        }
        Ok(())
    }
}

fn main() {
    //let cmd = commands::FEELCommands::parse();
    let args = FEELArgs::parse();
    match args.execute() {
        Ok(_) => (),
        Err(err) => panic!("{}", err),
    }
}
