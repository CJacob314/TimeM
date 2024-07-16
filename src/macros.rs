#[macro_export]
macro_rules! exit_error {
    ($s:expr) => {
        $crate::logger_init();
        $crate::log::error!($s);
        std::process::exit(1);
    };
}
