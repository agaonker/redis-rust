use crate::protocol::RespValue;

pub fn handle_ping(args: &[Vec<u8>]) -> RespValue {
    match args.len() {
        0 => RespValue::SimpleString("PONG".into()),
        1 => RespValue::BulkString(Some(args[0].clone())),
        _ => RespValue::Error("ERR wrong number of arguments for 'ping' command".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_args() {
        assert_eq!(handle_ping(&[]), RespValue::SimpleString("PONG".into()));
    }

    #[test]
    fn one_arg() {
        let args = vec![b"hello".to_vec()];
        assert_eq!(handle_ping(&args), RespValue::BulkString(Some(b"hello".to_vec())));
    }

    #[test]
    fn too_many_args() {
        let args = vec![b"a".to_vec(), b"b".to_vec()];
        assert!(matches!(handle_ping(&args), RespValue::Error(_)));
    }
}
