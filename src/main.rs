use std::process::ExitCode;

fn main() -> ExitCode {
    let status = browser_jr::run_cli(
        std::env::args_os().skip(1),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    ExitCode::from(status.code())
}
