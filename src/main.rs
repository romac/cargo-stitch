fn main() {
    if let Err(e) = cargo_stitch::run() {
        eprintln!("cargo-stitch: {e}");
        std::process::exit(1);
    }
}
