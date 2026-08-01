use std::time::Instant;
fn main() {
    let started = Instant::now();
    let mut paths = (0..10_000)
        .map(|index| format!("group-{}/file-{index}.md", index / 100))
        .collect::<Vec<_>>();
    paths.sort_by_key(|path| path.to_ascii_lowercase());
    let matched = paths.iter().filter(|path| path.contains("file-99")).count();
    println!(
        "10,000 path sort/search: {:?}, matches={matched}",
        started.elapsed()
    );
    assert!(started.elapsed().as_secs_f32() < 1.0);
}
