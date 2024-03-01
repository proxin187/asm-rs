mod preprocessor;
mod codegen;
mod parser;
mod log;

use codegen::Codegen;
use clap::{Parser, Subcommand};

use std::process;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, short, action)]
    debug: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Assemble { file: String },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Assemble { file } => {
            log::info(&format!("assembling `{}`", file));

            let mut codegen = match Codegen::new(&file) {
                Ok(codegen) => codegen,
                Err(err) => {
                    println!("{}", err);
                    process::exit(1);
                },
            };

            if let Err(err) = codegen.emit(&file) {
                log::error(&format!("{}:{}", file, codegen.line), &err.to_string());
                process::exit(1);
            }

            if args.debug {
                for (section, address) in codegen.preprocessor.offsets {
                    log::info(&format!("{} -> {:#x?}", section, address));
                }
            }

            log::info("done");
        },
    }
}

