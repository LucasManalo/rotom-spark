use std::{env, process::ExitCode};

const HELP: &str = "Rotom Spark development CLI\n\nUsage: rotom [--help | --version]";

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);

    match (arguments.next(), arguments.next()) {
        (None, None) => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        (Some(ref argument), None) if argument == "--help" || argument == "-h" => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        (Some(ref argument), None) if argument == "--version" || argument == "-V" => {
            println!("rotom {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        (Some(argument), trailing) => {
            let trailing_note =
                trailing.map_or(String::new(), |_| " and trailing arguments".to_owned());
            eprintln!("unrecognized argument {argument:?}{trailing_note}\n\n{HELP}");
            ExitCode::from(2)
        }
        (None, Some(_)) => unreachable!("a second argument cannot exist without a first"),
    }
}
