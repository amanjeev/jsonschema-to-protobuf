use jsonschema_to_protobuf::{convert, Options};
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    let input_path = args.get(1).map(|s| s.as_str()).unwrap_or("examples/user.json");

    let json = fs::read_to_string(input_path).expect("Failed to read input file");

    let options = Options {
        package: Some("myapi.v1".to_string()),
    };

    match convert(&json, options) {
        Ok(proto) => {
            println!("{}", proto);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
