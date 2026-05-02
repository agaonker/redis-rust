#[derive(Debug, PartialEq, Clone)]
pub enum RespValue {
    /// +OK\r\n
    SimpleString(String),
    /// -ERR message\r\n
    Error(String),
    /// :42\r\n
    Integer(i64),
    /// $3\r\nfoo\r\n  or  $-1\r\n (null)
    BulkString(Option<Vec<u8>>),
    /// *2\r\n...  or  *-1\r\n (null)
    Array(Option<Vec<RespValue>>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_string() {
        let v = RespValue::SimpleString("OK".into());
        assert_eq!(v, RespValue::SimpleString("OK".into()));
    }

    #[test]
    fn error() {
        let v = RespValue::Error("ERR bad".into());
        assert_eq!(v, RespValue::Error("ERR bad".into()));
    }

    #[test]
    fn integer() {
        let v = RespValue::Integer(42);
        assert_eq!(v, RespValue::Integer(42));
    }

    #[test]
    fn bulk_string_some() {
        let v = RespValue::BulkString(Some(b"hello".to_vec()));
        assert_eq!(v, RespValue::BulkString(Some(b"hello".to_vec())));
    }

    #[test]
    fn bulk_string_null() {
        let v = RespValue::BulkString(None);
        assert_ne!(v, RespValue::BulkString(Some(b"".to_vec())));
    }

    #[test]
    fn array_some() {
        let v = RespValue::Array(Some(vec![
            RespValue::BulkString(Some(b"GET".to_vec())),
            RespValue::BulkString(Some(b"foo".to_vec())),
        ]));
        assert!(matches!(v, RespValue::Array(Some(_))));
    }

    #[test]
    fn array_null() {
        let v = RespValue::Array(None);
        assert_ne!(v, RespValue::Array(Some(vec![])));
    }
}
