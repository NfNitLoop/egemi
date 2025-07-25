use std::fmt::Display;
use std::fmt::Write as _;

/// Like Rust's built-in Join, but works on things that impl Display.
pub trait DisplayJoin {
    /// Join an iterable of Displays.
    fn join(&mut self, joiner: &str) -> String;
}

impl <T, D> DisplayJoin for T
    where T: Iterator<Item = D>,
    D: Display
{
    fn join(&mut self, joiner: &str) -> String {
        let mut out = String::new();
        let Some(first) = self.next() else { 
            return out;
        };
        write!(out, "{first}").expect("writing to string");

        for part in self {
            write!(out, "{joiner}{part}").expect("writing to string");
        }
        out
    }
}



