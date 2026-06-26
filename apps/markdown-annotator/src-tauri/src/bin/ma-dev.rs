use markdown_annotator_lib::cli_launcher::{self, CliMode};

fn main() {
    if let Err(error) = cli_launcher::run("ma-dev", CliMode::Dev) {
        eprintln!("ma-dev: {error}");
        std::process::exit(1);
    }
}
