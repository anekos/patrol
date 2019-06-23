
use failure::Fail;



// pub type PatrolResult<T> = Result<T, PatrolError>;
pub type PatrolResultU = Result<(), PatrolError>;



#[derive(Fail, Debug)]
pub enum PatrolError {
    #[fail(display = "Channel lost")]
    Channel,
    #[fail(display = "IO Error: {}", 0)]
    Io(std::io::Error),
    #[fail(display = "No filename")]
    NoFilename,
    #[fail(display = "Invalid number: {}", 0)]
    NumberFormat(std::num::ParseIntError),
    #[fail(display = "Filepath encoding is invalid")]
    FilepathEncoding,
    #[fail(display = "Void")]
    Void,
}


macro_rules! define_error {
    ($source:ty, $kind:ident) => {
        impl From<$source> for PatrolError {
            fn from(error: $source) -> PatrolError {
                PatrolError::$kind(error)
            }
        }
    }
}

define_error!(std::io::Error, Io);
define_error!(std::num::ParseIntError, NumberFormat);

impl<T> From<std::sync::mpsc::SendError<T>> for PatrolError {
    fn from(_error: std::sync::mpsc::SendError<T>) -> PatrolError {
        PatrolError::Channel
    }
}
