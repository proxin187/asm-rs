mod preprocessor;
mod codegen;
mod parser;

use codegen::Codegen;

use std::process;

fn main() {
    let mut codegen = match Codegen::new("program.fasm") {
        Ok(codegen) => codegen,
        Err(err) => {
            println!("{}", err);
            process::exit(1);
        },
    };

    if let Err(err) = codegen.emit("program.o") {
        println!("{} {}", codegen.line, err);
        process::exit(1);
    }
}

