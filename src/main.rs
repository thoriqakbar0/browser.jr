use std::process::ExitCode;

fn main() -> ExitCode {
    let mut input = std::io::stdin().lock();
    let status = browser_jr::run_cli_with_input(
        std::env::args_os().skip(1),
        &mut input,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    ExitCode::from(status.code())
}
