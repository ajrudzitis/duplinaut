use duplicity_core::source::{Cancellation, CheckedReader};
use std::io::{Cursor, Read};

#[test]
fn checked_stream_requires_exact_size_and_real_eof() {
    for (bytes, expected, succeeds) in [(b"abc".as_slice(), 3, true), (b"ab", 3, false), (b"abcd", 3, false), (b"", 0, true), (b"a", 0, false)] {
        let mut reader = CheckedReader::new(Cursor::new(bytes), expected, 3, Cancellation::default()).unwrap();
        let mut output = Vec::new();
        assert_eq!(reader.read_to_end(&mut output).is_ok(), succeeds);
        assert!(output.len() <= expected as usize);
    }
}

#[test]
fn cancellation_and_size_limits_fail_before_delivery() {
    let cancel = Cancellation::default();
    assert!(CheckedReader::new(Cursor::new(b"abcd"), 4, 3, cancel.clone()).is_err());
    let mut reader = CheckedReader::new(Cursor::new(b"abc"), 3, 3, cancel.clone()).unwrap();
    let mut byte = [0];
    assert_eq!(reader.read(&mut byte).unwrap(), 1);
    cancel.cancel();
    assert!(reader.read(&mut byte).is_err());
    assert!(CheckedReader::new(Cursor::new(b"abc"), 3, 3, cancel).is_err());
}

#[test]
fn terminal_failure_cannot_be_hidden_by_another_eof_probe() {
    let mut reader = CheckedReader::new(Cursor::new(b"abcd"), 3, 3, Cancellation::default()).unwrap();
    let mut output = Vec::new();
    assert!(reader.read_to_end(&mut output).is_err());
    assert_eq!(output, b"abc");
    assert!(reader.read_to_end(&mut output).is_err());
    assert!(reader.read(&mut [0]).is_err());

    struct BadCount;
    impl Read for BadCount {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> { Ok(usize::MAX) }
    }
    let mut reader = CheckedReader::new(BadCount, 3, 3, Cancellation::default()).unwrap();
    assert!(reader.read(&mut [0]).is_err());
    assert!(reader.read(&mut [0]).is_err());
}
