#![feature(assert_matches)]

use clap::Parser;
use feel_core::eval;
use feel_core::parse;
use feel_core::scan::TextPosition;
use fileinput::FileInput;
use std::io::{BufRead, BufReader};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct FEELArgs {
    #[arg(short, long, help = "dump AST node instead of evaluating")]
    ast: bool,

    #[arg(short, long, help = "output format is JSON")]
    json: bool,

    #[arg(short, long, help = "given input as string instead of from files")]
    code: Option<String>,

    files: Vec<String>,
}

impl FEELArgs {
    fn parse_and_eval(&self, code: &str) -> Result<(), (eval::EvalError, TextPosition)> {
        let n = match parse::parse(code) {
            Ok(v) => v,
            Err((err, pos)) => return Err((eval::EvalError::from(err), pos)),
        };
        if self.ast {
            if self.json {
                let serialized = serde_json::to_string_pretty(&n).unwrap();
                println!("{}", serialized);
            } else {
                println!("{}", n);
            }
        } else {
            let mut eng = eval::Engine::new();
            let res = match eng.eval(n.clone()) {
                Ok(v) => v,
                Err(err) => return Err((err, n.start_pos)),
            };
            println!("{}", res);
        }
        Ok(())
    }

    fn execute(&self) -> () {
        let input = if let Some(code) = self.code.clone() {
            //self.parse_and_eval(code.as_str())
            code
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
            //self.parse_and_eval(buf.as_str())
            buf
        };
        match self.parse_and_eval(input.as_str()) {
            Ok(_) => (),
            Err((err, pos)) => {
                eprintln!(
                    "{}\nPosition: {}\n\n{}",
                    err,
                    pos,
                    pos.line_pointers(input.as_str())
                );
            }
        }
        ()
    }
}

fn main() {
    //let cmd = commands::FEELCommands::parse();
    let args = FEELArgs::parse();
    args.execute()
}
