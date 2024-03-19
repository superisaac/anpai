//#![feature(assert_matches)]

use clap::*;
use feel_lang::eval;
use feel_lang::parse;
use feel_lang::scan::TextPosition;
use fileinput::FileInput;
use std::io::{BufRead, BufReader};

#[derive(Parser, Debug)]
#[clap(
    name = "wfkit",
    about = "workflow kits and tools",
    rename_all = "kebab-case"
)]
enum FEELCommands {
    #[clap(name = "feel", about = "run feel language intepreter")]
    Feel {
        #[arg(short, long, help = "dump AST node instead of evaluating")]
        ast: bool,

        #[arg(short, long, help = "output format is JSON")]
        json: bool,

        #[arg(short, long, help = "given input as string instead of from files")]
        code: Option<String>,

        files: Vec<String>,
    },
}

impl FEELCommands {
    fn parse_and_eval(
        &self,
        code: &str,
        dump_ast: bool,
        json_format: bool,
    ) -> Result<(), (eval::EvalError, TextPosition)> {
        let n = match parse::parse(code) {
            Ok(v) => v,
            Err((err, pos)) => return Err((eval::EvalError::from(err), pos)),
        };
        if dump_ast {
            if json_format {
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
        match self {
            Self::Feel {
                ast,
                json,
                code,
                files,
            } => {
                let input = if let Some(code) = code.clone() {
                    //self.parse_and_eval(code.as_str())
                    code
                } else {
                    let filenames: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
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
                match self.parse_and_eval(input.as_str(), *ast, *json) {
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
            }
        }

        ()
    }
}

fn main() {
    //let cmd = commands::FEELCommands::parse();
    let args = FEELCommands::parse();
    args.execute()
}
