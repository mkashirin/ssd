fn main() {
    if let Err(err) = osul::run() {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
