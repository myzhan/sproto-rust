//! sproto-codegen: Generate Rust structs from sproto schema files.
//!
//! Usage:
//!     sproto-codegen [OPTIONS] <INPUT...>
//!
//! Options:
//!     -m, --mode <MODE>   Generation mode: derive, serde, both (default: both)
//!     -o, --output <FILE> Output file (default: stdout)
//!     -h, --help          Print help

use std::fs;
use std::process;

use sproto::codegen::{generate, CodegenMode};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut mode = CodegenMode::Both;
    let mut output_path: Option<String> = None;
    let mut input_paths: Vec<String> = Vec::new();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            "-m" | "--mode" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --mode requires a value");
                    process::exit(1);
                }
                mode = match args[i].as_str() {
                    "derive" => CodegenMode::Derive,
                    "serde" => CodegenMode::Serde,
                    "both" => CodegenMode::Both,
                    other => {
                        eprintln!("error: unknown mode '{other}', expected: derive, serde, both");
                        process::exit(1);
                    }
                };
            }
            "-o" | "--output" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --output requires a value");
                    process::exit(1);
                }
                output_path = Some(args[i].clone());
            }
            arg if arg.starts_with('-') => {
                eprintln!("error: unknown option '{arg}'");
                print_usage();
                process::exit(1);
            }
            _ => {
                input_paths.push(args[i].clone());
            }
        }
        i += 1;
    }

    if input_paths.is_empty() {
        eprintln!("error: missing input file(s)");
        print_usage();
        process::exit(1);
    }

    let mut schema_texts: Vec<String> = Vec::new();
    for path in &input_paths {
        match fs::read_to_string(path) {
            Ok(s) => schema_texts.push(s),
            Err(e) => {
                eprintln!("error: cannot read '{path}': {e}");
                process::exit(1);
            }
        }
    }

    let refs: Vec<&str> = schema_texts.iter().map(|s| s.as_str()).collect();
    let code = match generate(&refs, mode) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    match output_path {
        Some(path) => {
            if let Err(e) = fs::write(&path, &code) {
                eprintln!("error: cannot write '{path}': {e}");
                process::exit(1);
            }
            eprintln!("wrote {path}");
        }
        None => {
            print!("{code}");
        }
    }
}

fn print_usage() {
    eprintln!(
        "\
sproto-codegen: Generate Rust structs from sproto schema files.

Usage:
    sproto-codegen [OPTIONS] <INPUT...>

Options:
    -m, --mode <MODE>   Generation mode: derive, serde, both (default: both)
    -o, --output <FILE> Output file (default: stdout)
    -h, --help          Print help

Examples:
    sproto-codegen schema.sproto
    sproto-codegen -m derive base.sproto ext.sproto -o generated.rs
    sproto-codegen --mode serde a.sproto b.sproto c.sproto"
    );
}
