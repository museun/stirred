mod token;
pub use token::Token;

mod link;
pub use link::Link;

mod set;
pub use set::Set;

mod brain;
pub use brain::Brain;

pub(self) type Word = Box<[u8]>;
