use crate::protocol::RespValue;

pub fn handle_echo(args: &[Vec<u8>]) -> RespValue {
    match args.len() {
        1 => RespValue::BulkString(Some(args[0].clone())),
        _ => RespValue::Error("ERR wrong number of arguments for 'echo' command".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_arg() {
        let args = vec![b"hello".to_vec()];
        assert_eq!(handle_echo(&args), RespValue::BulkString(Some(b"hello".to_vec())));
    }

    #[test]
    fn no_args() {
        assert!(matches!(handle_echo(&[]), RespValue::Error(_)));
    }

    #[test]
    fn too_many_args() {
        let args = vec![b"a".to_vec(), b"b".to_vec()];
        assert!(matches!(handle_echo(&args), RespValue::Error(_)));
    }
}
