use super::RespValue;

pub fn serialize(value: &RespValue) -> Vec<u8> {
    let mut buf = Vec::new();
    write_value(value, &mut buf);
    buf
}

fn write_value(value: &RespValue, buf: &mut Vec<u8>) {
    match value {
        RespValue::SimpleString(s) => {
            buf.push(b'+');
            buf.extend_from_slice(s.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
        RespValue::Error(s) => {
            buf.push(b'-');
            buf.extend_from_slice(s.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
        RespValue::Integer(n) => {
            buf.push(b':');
            buf.extend_from_slice(n.to_string().as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
        RespValue::BulkString(None) => {
            buf.extend_from_slice(b"$-1\r\n");
        }
        RespValue::BulkString(Some(data)) => {
            buf.push(b'$');
            buf.extend_from_slice(data.len().to_string().as_bytes());
            buf.extend_from_slice(b"\r\n");
            buf.extend_from_slice(data);
            buf.extend_from_slice(b"\r\n");
        }
        RespValue::Array(None) => {
            buf.extend_from_slice(b"*-1\r\n");
        }
        RespValue::Array(Some(elements)) => {
            buf.push(b'*');
            buf.extend_from_slice(elements.len().to_string().as_bytes());
            buf.extend_from_slice(b"\r\n");
            for elem in elements {
                write_value(elem, buf);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_string() {
        assert_eq!(serialize(&RespValue::SimpleString("OK".into())), b"+OK\r\n");
    }

    #[test]
    fn error() {
        assert_eq!(serialize(&RespValue::Error("ERR bad".into())), b"-ERR bad\r\n");
    }

    #[test]
    fn integer() {
        assert_eq!(serialize(&RespValue::Integer(42)), b":42\r\n");
        assert_eq!(serialize(&RespValue::Integer(-1)), b":-1\r\n");
        assert_eq!(serialize(&RespValue::Integer(0)), b":0\r\n");
    }

    #[test]
    fn bulk_string_null() {
        assert_eq!(serialize(&RespValue::BulkString(None)), b"$-1\r\n");
    }

    #[test]
    fn bulk_string_empty() {
        assert_eq!(serialize(&RespValue::BulkString(Some(vec![]))), b"$0\r\n\r\n");
    }

    #[test]
    fn bulk_string_data() {
        assert_eq!(
            serialize(&RespValue::BulkString(Some(b"foo".to_vec()))),
            b"$3\r\nfoo\r\n"
        );
    }

    #[test]
    fn array_null() {
        assert_eq!(serialize(&RespValue::Array(None)), b"*-1\r\n");
    }

    #[test]
    fn array_empty() {
        assert_eq!(serialize(&RespValue::Array(Some(vec![]))), b"*0\r\n");
    }

    #[test]
    fn array_two_elements() {
        let v = RespValue::Array(Some(vec![
            RespValue::BulkString(Some(b"foo".to_vec())),
            RespValue::BulkString(Some(b"bar".to_vec())),
        ]));
        assert_eq!(serialize(&v), b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
    }

    #[test]
    fn nested_array() {
        let inner = RespValue::Array(Some(vec![RespValue::Integer(1)]));
        let outer = RespValue::Array(Some(vec![inner, RespValue::SimpleString("OK".into())]));
        assert_eq!(serialize(&outer), b"*2\r\n*1\r\n:1\r\n+OK\r\n");
    }
}
