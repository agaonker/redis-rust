use super::RespValue;
use crate::error::RedisError;

/// Result of attempting to parse one RESP value from a byte buffer.
pub enum ParseOutcome {
    /// Parsed a complete value; usize = number of bytes consumed.
    Complete(RespValue, usize),
    /// Buffer contains a partial frame — need more data.
    Incomplete,
    /// Buffer contains data that violates the RESP spec.
    Err(RedisError),
}

/// Attempt to parse one RESP value from `buf`.
/// Returns how many bytes were consumed on success.
pub fn parse(buf: &[u8]) -> ParseOutcome {
    if buf.is_empty() {
        return ParseOutcome::Incomplete;
    }
    match buf[0] {
        b'+' => parse_simple_string(buf),
        b'-' => parse_error(buf),
        b':' => parse_integer(buf),
        b'$' => parse_bulk_string(buf),
        b'*' => parse_array(buf),
        b => ParseOutcome::Err(RedisError::Parse(format!(
            "unknown type byte: 0x{:02x}",
            b
        ))),
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Find the first `\r\n` in `buf` starting at `offset`.
/// Returns the index of `\r` if found.
fn find_crlf(buf: &[u8], offset: usize) -> Option<usize> {
    buf[offset..].windows(2).position(|w| w == b"\r\n").map(|p| p + offset)
}

/// Parse a line starting after the type byte (index 1 .. crlf).
/// Returns `(line_bytes, total_consumed)` or `None` if incomplete.
fn parse_line(buf: &[u8]) -> Option<(&[u8], usize)> {
    let crlf = find_crlf(buf, 1)?;
    Some((&buf[1..crlf], crlf + 2))
}

// ── simple string ─────────────────────────────────────────────────────────────

fn parse_simple_string(buf: &[u8]) -> ParseOutcome {
    match parse_line(buf) {
        None => ParseOutcome::Incomplete,
        Some((line, consumed)) => {
            match std::str::from_utf8(line) {
                Ok(s) => ParseOutcome::Complete(RespValue::SimpleString(s.to_owned()), consumed),
                Err(_) => ParseOutcome::Err(RedisError::Parse("simple string is not valid UTF-8".into())),
            }
        }
    }
}

// ── error ─────────────────────────────────────────────────────────────────────

fn parse_error(buf: &[u8]) -> ParseOutcome {
    match parse_line(buf) {
        None => ParseOutcome::Incomplete,
        Some((line, consumed)) => {
            match std::str::from_utf8(line) {
                Ok(s) => ParseOutcome::Complete(RespValue::Error(s.to_owned()), consumed),
                Err(_) => ParseOutcome::Err(RedisError::Parse("error string is not valid UTF-8".into())),
            }
        }
    }
}

// ── integer ───────────────────────────────────────────────────────────────────

fn parse_integer(buf: &[u8]) -> ParseOutcome {
    match parse_line(buf) {
        None => ParseOutcome::Incomplete,
        Some((line, consumed)) => {
            match std::str::from_utf8(line).ok().and_then(|s| s.parse::<i64>().ok()) {
                Some(n) => ParseOutcome::Complete(RespValue::Integer(n), consumed),
                None => ParseOutcome::Err(RedisError::Parse(format!(
                    "invalid integer: {:?}",
                    line
                ))),
            }
        }
    }
}

// ── bulk string ───────────────────────────────────────────────────────────────

pub(super) fn parse_bulk_string(buf: &[u8]) -> ParseOutcome {
    let (len_line, header_end) = match parse_line(buf) {
        None => return ParseOutcome::Incomplete,
        Some(v) => v,
    };

    let len: i64 = match std::str::from_utf8(len_line).ok().and_then(|s| s.parse().ok()) {
        Some(n) => n,
        None => return ParseOutcome::Err(RedisError::Parse(format!(
            "invalid bulk string length: {:?}", len_line
        ))),
    };

    if len < -1 {
        return ParseOutcome::Err(RedisError::Parse(format!("bulk string length out of range: {}", len)));
    }

    if len == -1 {
        return ParseOutcome::Complete(RespValue::BulkString(None), header_end);
    }

    let len = len as usize;
    let needed = header_end + len + 2; // +2 for trailing \r\n
    if buf.len() < needed {
        return ParseOutcome::Incomplete;
    }

    // Verify trailing \r\n
    if &buf[header_end + len..header_end + len + 2] != b"\r\n" {
        return ParseOutcome::Err(RedisError::Parse("bulk string missing trailing CRLF".into()));
    }

    let data = buf[header_end..header_end + len].to_vec();
    ParseOutcome::Complete(RespValue::BulkString(Some(data)), needed)
}

// ── array ─────────────────────────────────────────────────────────────────────

pub(super) fn parse_array(buf: &[u8]) -> ParseOutcome {
    let (count_line, mut cursor) = match parse_line(buf) {
        None => return ParseOutcome::Incomplete,
        Some(v) => v,
    };

    let count: i64 = match std::str::from_utf8(count_line).ok().and_then(|s| s.parse().ok()) {
        Some(n) => n,
        None => return ParseOutcome::Err(RedisError::Parse(format!(
            "invalid array length: {:?}", count_line
        ))),
    };

    if count < -1 {
        return ParseOutcome::Err(RedisError::Parse(format!("array length out of range: {}", count)));
    }

    if count == -1 {
        return ParseOutcome::Complete(RespValue::Array(None), cursor);
    }

    let count = count as usize;
    let mut elements = Vec::with_capacity(count);

    for _ in 0..count {
        match parse(&buf[cursor..]) {
            ParseOutcome::Complete(val, consumed) => {
                elements.push(val);
                cursor += consumed;
            }
            ParseOutcome::Incomplete => return ParseOutcome::Incomplete,
            ParseOutcome::Err(e) => return ParseOutcome::Err(e),
        }
    }

    ParseOutcome::Complete(RespValue::Array(Some(elements)), cursor)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn complete(outcome: ParseOutcome) -> (RespValue, usize) {
        match outcome {
            ParseOutcome::Complete(v, n) => (v, n),
            ParseOutcome::Incomplete => panic!("expected Complete, got Incomplete"),
            ParseOutcome::Err(e) => panic!("expected Complete, got Err: {}", e),
        }
    }

    fn assert_incomplete(outcome: ParseOutcome) {
        assert!(matches!(outcome, ParseOutcome::Incomplete), "expected Incomplete");
    }

    fn assert_err(outcome: ParseOutcome) {
        assert!(matches!(outcome, ParseOutcome::Err(_)), "expected Err");
    }

    // simple string
    #[test]
    fn simple_string_ok() {
        let (v, n) = complete(parse(b"+OK\r\n"));
        assert_eq!(v, RespValue::SimpleString("OK".into()));
        assert_eq!(n, 5);
    }

    #[test]
    fn simple_string_empty() {
        let (v, n) = complete(parse(b"+\r\n"));
        assert_eq!(v, RespValue::SimpleString("".into()));
        assert_eq!(n, 3);
    }

    #[test]
    fn simple_string_incomplete() {
        assert_incomplete(parse(b"+OK"));
        assert_incomplete(parse(b"+OK\r"));
        assert_incomplete(parse(b""));
    }

    // error
    #[test]
    fn error_ok() {
        let (v, n) = complete(parse(b"-ERR bad\r\n"));
        assert_eq!(v, RespValue::Error("ERR bad".into()));
        assert_eq!(n, 10);
    }

    #[test]
    fn error_incomplete() {
        assert_incomplete(parse(b"-ERR"));
    }

    // integer
    #[test]
    fn integer_ok() {
        let (v, n) = complete(parse(b":42\r\n"));
        assert_eq!(v, RespValue::Integer(42));
        assert_eq!(n, 5);
    }

    #[test]
    fn integer_negative() {
        let (v, _) = complete(parse(b":-1\r\n"));
        assert_eq!(v, RespValue::Integer(-1));
    }

    #[test]
    fn integer_zero() {
        let (v, _) = complete(parse(b":0\r\n"));
        assert_eq!(v, RespValue::Integer(0));
    }

    #[test]
    fn integer_incomplete() {
        assert_incomplete(parse(b":42"));
    }

    #[test]
    fn integer_malformed() {
        assert_err(parse(b":abc\r\n"));
    }

    // unknown type byte
    #[test]
    fn unknown_type_byte() {
        assert_err(parse(b"@hello\r\n"));
    }
}
