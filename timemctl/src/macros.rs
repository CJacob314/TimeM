#[macro_export]
macro_rules! exit_error {
    ($s:expr) => {
        eprintln!($s);
        std::process::exit(1);
    };
}
