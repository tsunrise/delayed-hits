// only print events if feature "verbose-sim" is enabled
#[macro_export]
macro_rules! verbose {
    ($($arg:tt)*) => {
        if cfg!(feature = "verbose-sim") {
            println!($($arg)*);
        }
    }
}