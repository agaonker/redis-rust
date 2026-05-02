use thiserror::Error;

#[derive(Debug, Error)]
pub enum RedisError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("WRONGTYPE Operation against a key holding the wrong kind of value")]
    WrongType,

    #[error("ERR unknown command '{0}'")]
    UnknownCommand(String),

    #[error("ERR wrong number of arguments for '{cmd}' command")]
    WrongArity { cmd: String },
}

pub type Result<T> = std::result::Result<T, RedisError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = RedisError::Parse("unexpected byte".into());
        assert!(e.to_string().contains("unexpected byte"));

        let e = RedisError::WrongType;
        assert!(e.to_string().contains("WRONGTYPE"));

        let e = RedisError::UnknownCommand("FOOBAR".into());
        assert!(e.to_string().contains("FOOBAR"));

        let e = RedisError::WrongArity { cmd: "GET".into() };
        assert!(e.to_string().contains("GET"));
    }
}
