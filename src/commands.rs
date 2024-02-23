use crate::eval::{EvalError, Intepreter};
use crate::parse::parse;
use clap::*;

#[derive(Parser)]
#[clap(
    name = "feel",
    about = "FEEL language toolchain",
    rename_all = "kebab-case"
)]
pub enum FEELCommands {
    #[clap(name = "ast", about = "parse feel codes into AST")]
    Ast {
        #[clap(short, long, help = "script code")]
        script: String,
    },
    #[clap(name = "eval", about = "evaluate feel codes")]
    Eval {
        #[clap(short, long, help = "script code")]
        script: String,
    },
}

impl FEELCommands {
    pub fn execute(self) -> Result<(), EvalError> {
        match self {
            Self::Ast { script } => {
                let n = parse(script.as_str())?;
                println!("{}", n);
                Ok(())
            }
            Self::Eval { script } => {
                let n = parse(script.as_str())?;
                let mut intp = Intepreter::new();
                let res = intp.eval(n)?;
                println!("{}", res);
                Ok(())
            }
        }
    }
}
