#[macro_export]
macro_rules! debugln {
    ( $flags:expr, $text:expr) => {
        if $flags.debug
        {
            println!($text);
        }
    };
    ( $flags:expr, $text:expr, $( $args:expr ),*) => {
        if $flags.debug
        {
            println!($text, $($args,)*);
        }
    };
}

#[macro_export]
macro_rules! silentln {
    ( $flags:expr, $text:expr) => {
        if !$flags.silent
        {
            println!($text);
        }
    };
    ( $flags:expr, $text:expr, $( $args:expr ),+) => {
        if !$flags.silent
        {
            println!($text, $($args,)*);
        }
    };
}
