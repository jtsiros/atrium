mod cli;

use std::process::ExitCode;


const USAGE: &str = "\
atriumd — Home Assistant bridge for the Atrium Omarchy panel

USAGE:
    atriumd serve                       run the bridge; NDJSON on stdin/stdout
    atriumd probe <url> [--rows]        connect once and print what the panel would show
    atriumd call  <url> <entity> <action> [json]
    atriumd --version

The access token is read from the ATRIUM_TOKEN environment variable, or from
stdin when that is not set. It is never accepted as a command-line argument,
where it would be visible to every process on the machine.
";

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version" | "-V") => {
            println!("atriumd {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("serve") => match atriumd::serve::run().await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("atriumd: {e}");
                ExitCode::FAILURE
            }
        },
        Some("call") => match (args.get(1), args.get(2), args.get(3)) {
            (Some(target), Some(entity), Some(action)) => {
                cli::call(target, entity, action, args.get(4).map(String::as_str)).await
            }
            _ => {
                eprintln!("atriumd call needs <url> <entity_id> <action> [json]");
                ExitCode::FAILURE
            }
        },
        Some("probe") => match args.get(1) {
            Some(target) => cli::probe(target).await,
            None => {
                eprintln!("atriumd probe needs a Home Assistant URL");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprint!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

