pub mod catalog;
pub mod claude;
pub mod credentials;
pub mod error;
pub mod http;
pub mod paths;
pub mod profile;
pub mod providers;
pub mod settings;
pub mod tui;

pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;
