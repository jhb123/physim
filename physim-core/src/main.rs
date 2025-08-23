#![feature(iter_intersperse)]
use std::env;

use physim_core::pipeline::Pipeline;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let mut args = env::args().peekable();
    args.next().ok_or("No configuration provided")?;

    let help_text = include_str!("help.txt");

    let pipeline = if let Some(v) = args.peek() {
        match v.as_str() {
            "-h" | "--help" => {
                println!("{}", help_text);
                return Ok(());
            }
            "-v" | "--version" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "-f" | "--file" => {
                args.next();
                let file = args.next().ok_or("No file provided")?;
                Pipeline::new_from_file(&file)?
            }
            _ => {
                let desc: String = args.intersperse(" ".to_string()).collect();
                Pipeline::new_from_description(&desc)?
            }
        }
    } else {
        println!("{}", help_text);
        return Ok(());
    };
    pipeline.run()
}
