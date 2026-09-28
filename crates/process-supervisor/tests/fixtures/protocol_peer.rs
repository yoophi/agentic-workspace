//! Standalone protocol-output fixture source compiled by output-policy tests.

use std::{env, io::Write, thread, time::Duration};

fn main() {
    match env::args().nth(1).as_deref().unwrap_or("exact") {
        "exact" => emit_line(1024),
        "limit-plus-one" => emit_line(1025),
        "malformed" => println!("{{not-json}}"),
        "mid-frame-eof" => print!("{{\"jsonrpc\":\"2.0\""),
        "slow-loris" => loop {
            print!("x");
            std::io::stdout().flush().expect("flush byte");
            thread::sleep(Duration::from_secs(1));
        },
        "endless-valid" => loop {
            println!("{{\"jsonrpc\":\"2.0\",\"method\":\"tick\"}}");
            std::io::stdout().flush().expect("flush frame");
        },
        other => panic!("unknown protocol fixture mode: {other}"),
    }
}

fn emit_line(size: usize) {
    println!("{}", "x".repeat(size));
}
