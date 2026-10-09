fn main() {
    if let Err(e) = pharos::run(std::env::args_os()) {
        eprintln!("Error: {e:?}");
        std::process::exit(1);
    }
}
