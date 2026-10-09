fn main() {
    if let Err(e) = pharos::run(std::env::args_os()) {
        eprintln!("{e:?}");
        std::process::exit(1);
    }
}
