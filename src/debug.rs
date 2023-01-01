/// Accepts a vector of T and creates a String
#[macro_export]
macro_rules! bulk_print {
    ($vec:expr, $s:expr) => {
        $vec.iter()
            .map(|x| x.print())
            .collect::<Vec<String>>()
            .join($s)
    };
}
