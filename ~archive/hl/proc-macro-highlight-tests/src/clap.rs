use clap::Parser;
#[derive(Parser)]
struct Cli {
    #[clap(long = "input", short = 'x')]
    _x: String,
}