pub mod drivers;
pub mod images;
pub mod teams;

pub use drivers::*;
pub use images::*;
pub use teams::*;

pub trait QueryParams<'q> {
    fn get_expands(&'q self) -> Vec<&'q str>;
}
