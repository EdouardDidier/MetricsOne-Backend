pub mod driver;
pub mod images;
pub mod meeting;
pub mod session;
pub mod team;

pub use driver::*;
pub use images::*;
pub use meeting::*;
pub use session::*;
pub use team::*;

pub trait QueryParams<'q> {
    fn get_expands(&'q self) -> Vec<&'q str>;
}
