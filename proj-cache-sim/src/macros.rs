/// only print if feature `verbose` is enabled
#[macro_export]
macro_rules! verbose {
    ($($arg:tt)*) => {
        if cfg!(feature = "verbose") {
            println!($($arg)*);
        }
    }
}
