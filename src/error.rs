#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not locate user home directory")]
    HomeDirNotFound,
}
