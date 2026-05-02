use crate::error::RedisError;
use crate::protocol::RespValue;

#[derive(Debug, Clone)]
pub struct Command {
    pub name: String,
    pub args: Vec<Vec<u8>>,
}

impl TryFrom<RespValue> for Command {
    type Error = RedisError;

    fn try_from(value: RespValue) -> Result<Self, Self::Error> {
        let elements = match value {
            RespValue::Array(Some(elems)) => elems,
            _ => return Err(RedisError::Parse("command must be a RESP array".into())),
        };

        if elements.is_empty() {
            return Err(RedisError::Parse("empty command array".into()));
        }

        // First element is the command name — must be a bulk string
        let name = match &elements[0] {
            RespValue::BulkString(Some(bytes)) => {
                String::from_utf8(bytes.clone())
                    .map_err(|_| RedisError::Parse("command name is not valid UTF-8".into()))?
                    .to_uppercase()
            }
            _ => return Err(RedisError::Parse("command name must be a bulk string".into())),
        };

        // Remaining elements are args — each must be a bulk string
        let mut args = Vec::with_capacity(elements.len() - 1);
        for elem in &elements[1..] {
            match elem {
                RespValue::BulkString(Some(bytes)) => args.push(bytes.clone()),
                _ => return Err(RedisError::Parse("command argument must be a bulk string".into())),
            }
        }

        Ok(Command { name, args })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bulk(s: &str) -> RespValue {
        RespValue::BulkString(Some(s.as_bytes().to_vec()))
    }

    fn array(elems: Vec<RespValue>) -> RespValue {
        RespValue::Array(Some(elems))
    }

    #[test]
    fn simple_command_no_args() {
        let cmd = Command::try_from(array(vec![bulk("PING")])).unwrap();
        assert_eq!(cmd.name, "PING");
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn command_name_uppercased() {
        let cmd = Command::try_from(array(vec![bulk("get")])).unwrap();
        assert_eq!(cmd.name, "GET");
    }

    #[test]
    fn command_with_args() {
        let cmd = Command::try_from(array(vec![bulk("SET"), bulk("foo"), bulk("bar")])).unwrap();
        assert_eq!(cmd.name, "SET");
        assert_eq!(cmd.args, vec![b"foo".to_vec(), b"bar".to_vec()]);
    }

    #[test]
    fn empty_array_is_error() {
        assert!(Command::try_from(array(vec![])).is_err());
    }

    #[test]
    fn non_array_is_error() {
        assert!(Command::try_from(RespValue::SimpleString("PING".into())).is_err());
        assert!(Command::try_from(RespValue::BulkString(Some(b"PING".to_vec()))).is_err());
    }

    #[test]
    fn non_bulk_string_name_is_error() {
        assert!(Command::try_from(array(vec![RespValue::SimpleString("PING".into())])).is_err());
    }
}
