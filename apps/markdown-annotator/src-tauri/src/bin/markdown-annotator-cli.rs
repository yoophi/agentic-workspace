use markdown_annotator_lib::cli_launcher::{self, CliMode};

fn main() {
    if let Err(error) = cli_launcher::run("markdown-annotator-cli", CliMode::Release) {
        eprintln!("markdown-annotator-cli: {error}");
        std::process::exit(1);
    }
}
