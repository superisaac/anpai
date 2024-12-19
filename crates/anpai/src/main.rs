use clap::*;

use feel::eval;
use feel::parse as feel_parse;

use dmn::eval as dmn_eval;
use dmn::types::DmnError;

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
enum AnpaiCommands {
    #[clap(name = "feel", about = "Run FEEL language intepretor")]
    Feel {
        #[arg(long, help = "Dump AST node instead of evaluating")]
        ast: bool,

        #[arg(long, help = "Output format is JSON")]
        json: bool,

        #[arg(long, help = "Context variable file")]
        varsfile: Option<String>,

        #[arg(long, help = "Context variables")]
        vars: Option<String>,

        #[arg(short, long, help = "Given input as string instead of from files")]
        code: Option<String>,

        #[arg(short, long, help = "Parse top mode")]
        top: Option<feel_parse::ParseTop>,

        files: Vec<String>,
    },

    #[clap(name = "dmn", about = "DMN parser and evaluator")]
    Dmn {
        #[arg(long, help = "Context variable file")]
        varsfile: Option<String>,

        #[arg(long, help = "Context variables")]
        vars: Option<String>,

        #[arg(long, short = 's', help = "Start decision id")]
        start_decision_id: Option<String>,

        file: String,
    },
}

impl AnpaiCommands {
    fn parse_and_eval_feel(
        &self,
        code: &str,
        varsfile: Option<String>,
        vars: Option<String>,
        top: Option<feel_parse::ParseTop>,
        dump_ast: bool,
        json_format: bool,
    ) -> Result<(), eval::EvalError> {
        let mut eng = Box::new(eval::Engine::new());
        // read context vars
        if let Some(context_varsfile) = varsfile {
            let mut data_file = File::open(context_varsfile.as_str()).unwrap();
            let mut content = String::new();
            data_file.read_to_string(&mut content).unwrap();
            eng.load_context_string(&content)?;
        }

        if let Some(context_vars) = vars {
            eng.load_context_string(&context_vars)?;
        }

        let n = feel_parse::parse(code, eng.clone(), top.unwrap_or_default())?;

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

    fn parse_and_eval_dmn(
        &self,
        varsfile: Option<String>,
        vars: Option<String>,
        start_decision_id: Option<String>,
        file: String,
    ) -> Result<(), DmnError> {
        let mut eng = Box::new(eval::Engine::new());
        // read context vars
        if let Some(context_varsfile) = varsfile {
            let mut data_file = File::open(context_varsfile.as_str()).unwrap();
            let mut content = String::new();
            data_file.read_to_string(&mut content).unwrap();
            match eng.load_context_string(&content) {
                Ok(_) => (),
                Err(err) => {
                    return Err(DmnError::FEELEval(err, "context-file".to_owned(), content))
                }
            }
        }

        if let Some(context_vars) = vars {
            //eng.load_context(&context_vars)?;
            match eng.load_context_string(&context_vars) {
                Ok(_) => (),
                Err(err) => {
                    return Err(DmnError::FEELEval(
                        err,
                        "context-vars".to_owned(),
                        context_vars,
                    ))
                }
            }
        }

        //dmn_parse::parse_file(file.as_str());
        let v = dmn_eval::eval_file(&mut eng, file.as_str(), start_decision_id)?;
        println!("{}", v);
        Ok(())
    }

    fn execute(&self) -> () {
        match self {
            Self::Feel {
                ast,
                json,
                varsfile,
                vars,
                top,
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
                match self.parse_and_eval_feel(
                    input.as_str(),
                    varsfile.clone(),
                    vars.clone(),
                    top.clone(),
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
            Self::Dmn {
                varsfile,
                vars,
                start_decision_id,
                file,
            } => match self.parse_and_eval_dmn(
                varsfile.clone(),
                vars.clone(),
                start_decision_id.clone(),
                file.clone(),
            ) {
                Ok(_) => (),
                Err(DmnError::FEELEval(err, path, code)) => {
                    eprintln!(
                        "Path: {}\n{}\nPosition: {}\n\n{}",
                        path,
                        err.kind,
                        err.pos,
                        err.pos.line_pointers(code.as_str()),
                    );
                }
                Err(err) => {
                    eprintln!("Error {}", err);
                }
            },
        }

        ()
    }
}

fn main() {
    let args = AnpaiCommands::parse();
    args.execute()
}
