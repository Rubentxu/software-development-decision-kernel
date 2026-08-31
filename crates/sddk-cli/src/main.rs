//! SDDK command-line entry point.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
fn main() {
    let output = sddk_cli::run_from(std::env::args_os());
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);
    std::process::exit(output.status);
}
