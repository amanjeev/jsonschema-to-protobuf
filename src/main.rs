use clap::Parser;
use jsonschema_to_protobuf::{convert, Options};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "jsonschema-to-proto")]
#[command(about = "Convert JSON Schema to Protocol Buffer definitions")]
#[command(version)]
struct Args {
    /// Input JSON Schema file (reads from stdin if not provided)
    #[arg(value_name = "INPUT")]
    input: Option<PathBuf>,

    /// Output .proto file (writes to stdout if not provided)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Package name for the generated proto file
    #[arg(short, long)]
    package: Option<String>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Read input
    let json = match &args.input {
        Some(path) => fs::read_to_string(path)?,
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };

    // Convert
    let options = Options {
        package: args.package,
    };

    let proto = convert(&json, options)?;
    let output = proto.to_string();

    // Write output
    match &args.output {
        Some(path) => {
            fs::write(path, output)?;
            eprintln!("Written to {}", path.display());
        }
        None => {
            io::stdout().write_all(output.as_bytes())?;
        }
    }

    Ok(())
}
