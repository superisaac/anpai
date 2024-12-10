use clap::*;
use feel::eval;
use feel::parse;
use fileinput::FileInput;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;

#[derive(Parser, Debug)]
#[clap(
    name = "anpai",
    about = "workflow kits and tools",
    rename_all = "kebab-case"
)]
enum FEELCommands {
    #[clap(name = "feel", about = "run feel language intepretor")]
    Feel {
        #[arg(short, long, help = "dump AST node instead of evaluating")]
        ast: bool,

        #[arg(short, long, help = "output format is JSON")]
        json: bool,

        #[arg(long, help = "context variables")]
        varsfile: Option<String>,

        #[arg(long, help = "context variables")]
        vars: Option<String>,

        #[arg(short, long, help = "given input as string instead of from files")]
        code: Option<String>,

        files: Vec<String>,
    },
}

impl FEELCommands {
    fn parse_and_eval(
        &self,
        code: &str,
        varsfile: Option<String>,
        vars: Option<String>,
        dump_ast: bool,
        json_format: bool,
    ) -> Result<(), eval::EvalError> {
        let mut eng = Box::new(eval::Engine::new());
        // read context vars
        if let Some(context_varsfile) = varsfile {
            let mut data_file = File::open(context_varsfile.as_str()).unwrap();
            let mut content = String::new();
            data_file.read_to_string(&mut content).unwrap();
            eng.load_context(&content)?;
        }

        if let Some(context_vars) = vars {
            eng.load_context(&context_vars)?;
        }
        let n = parse::parse(code, eng.clone(), Default::default())?;

        if dump_ast {
            if json_format {
                let serialized = serde_json::to_string_pretty(&n).unwrap();
                println!("{}", serialized);
            } else {
                println!("{}", n);
            }
        } else {
            let res = eng.eval(n.clone())?;
            println!("{}", res);
        }
        Ok(())
    }

    fn execute(&self) -> () {
        match self {
            Self::Feel {
                ast,
                json,
                varsfile,
                vars,
                code,
                files,
            } => {
                let input = if let Some(code) = code.clone() {
                    //self.parse_and_eval(code.as_str())
                    code
                } else {
                    let filenames: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
                    let fileinput = FileInput::new(&filenames);
                    let mut reader = BufReader::new(fileinput);

                    // read all contents from either files or stdin
                    let mut buf: String = String::new();
                    reader.read_to_string(&mut buf).unwrap();
                    buf
                };
                match self.parse_and_eval(
                    input.as_str(),
                    varsfile.clone(),
                    vars.clone(),
                    *ast,
                    *json,
                ) {
                    Ok(_) => (),
                    Err(err) => {
                        eprintln!(
                            "{}\nPosition: {}\n\n{}",
                            err.kind,
                            err.pos,
                            err.pos.line_pointers(input.as_str())
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
