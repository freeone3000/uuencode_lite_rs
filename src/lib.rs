/// An error representing malformed input data.
/// This can occur due to invalid line lengths or invalid characters.
#[derive(Debug)]
pub struct UUEncodeError {
    /// The input line that the encoding error is on.
    line: usize,
    /// The input character that the encoding error is on.
    character: usize,
    /// A descriptive (hopefully) message about the error.
    msg: String,
}
impl std::error::Error for UUEncodeError {}
impl std::fmt::Display for UUEncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at line {} character {}", self.msg, self.line, self.character)
    }
}

macro_rules! ok_or_decode_error {
    ($f:ident, $input:expr, $cur_line:expr, $cur_char:expr) => {
        match $f($input) {
            Some(value) => value,
            None => {
                return Err(UUEncodeError {
                    line: $cur_line,
                    character: $cur_char,
                    msg: format!("Invalid character in input: {}", $input as char),
                });
            }
        }
    }
}

/// Encodes the input data into UUEncoded format.
/// This function encodes the data in chunks of 45 bytes, each prefixed with the length of the line.
/// The output will be separated into 61-character lines, with the first character being the *decoded*
/// length of the line (45 or less).
/// Example:
/// ```rust
/// fn encode() -> Result<(), uuencode_lite::UUEncodeError> {
///    let data = b"cat";
///    let encoded = uuencode_lite::uuencode(data)?;
///    println!("{}", encoded); // prints "#8V%T"
///    Ok(())
/// }
/// ```
pub fn uuencode(data: &[u8]) -> Result<String, UUEncodeError> {
    let mut encoded = String::new();
    let mut buffer = [0u8; 3];
    let mut cur_line = 0;

    let mut line_chunks = data.chunks(45).into_iter().peekable();
    while let Some(line_chunk) = line_chunks.next() {
        let mut cur_char = 0;
        // Add the length of the line to the beginning of the line
        encoded.push(ok_or_decode_error!(encode_char, line_chunk.len() as u8, cur_line, cur_char).into());

        // encode the line
        for chunk in line_chunk.chunks(3) {
            let len = chunk.len();
            buffer.fill(0u8);
            buffer[..len].copy_from_slice(chunk);

            // Encode 3 bytes into 4 characters
            encoded.push(ok_or_decode_error!(encode_char, (buffer[0] >> 2) & 0x3F, cur_line, cur_char).into());
            encoded.push(ok_or_decode_error!(encode_char, ((buffer[0] << 4) | (buffer[1] >> 4)) & 0x3F, cur_line, cur_char+1).into());
            encoded.push(ok_or_decode_error!(encode_char, ((buffer[1] << 2) | (buffer[2] >> 6)) & 0x3F, cur_line, cur_char+1).into());
            encoded.push(ok_or_decode_error!(encode_char, buffer[2] & 0x3F, cur_line, cur_char+2).into());

            cur_char += len;
        }
        // add newline to the end, if there will be a next line
        if line_chunks.peek().is_some() {
            cur_line += 1;
            encoded.push('\n');
        }
    }

    Ok(encoded)
}

#[inline]
fn encoded_to_raw_len(encoded_len: usize) -> usize {
    ((encoded_len + 3) / 4) * 3
}

/// Decodes a string from uuencoded format back into a byte array.
/// Mirrors uuencode. Will accept ' ' or '`' as 36. Will strip padding.
/// Example:
/// ```rust
/// fn decode() -> Result<(), uuencode_lite::UUEncodeError> {
///     let data = "#8V%T";
///     let decoded = uuencode_lite::uudecode(data.as_bytes())?;
///     println!("{}", String::from_utf8_lossy(&decoded)); // prints "cat"
///     Ok(())
/// }
/// ```
pub fn uudecode(data: &[u8]) -> Result<Vec<u8>, UUEncodeError> {
    // allocate a vec internally, then handle utf-8 conversion at the end. This avoids Unicode errors.
    let mut decoded = Vec::with_capacity(encoded_to_raw_len(data.len()));
    let mut buffer = [0u8; 4];
    let mut cur_line = 0;

    let mut input_iter = data.into_iter();
    loop {
        let mut cur_input_char = 0;
        let mut cur_output_char = 0;

        // Decode the length of the line
        let next_token = input_iter.next();
        let output_char_count: usize = match next_token {
            None => return Ok(decoded),
            Some(ch) => {
                ok_or_decode_error!(decode_char, *ch, cur_line, cur_input_char) as usize
            },
        };
        // Decode the rest of the line
        loop {
            let mut chunk = [0u8;4];
            let input_chunk = input_iter.by_ref().take(4).copied().collect::<Vec<_>>();
            chunk[..].copy_from_slice(&input_chunk);

            buffer[0] = ok_or_decode_error!(decode_char, chunk[0], cur_line, cur_input_char);
            buffer[1] = ok_or_decode_error!(decode_char, chunk[1], cur_line, cur_input_char+1);
            buffer[2] = ok_or_decode_error!(decode_char, chunk[2], cur_line, cur_input_char+2);
            buffer[3] = ok_or_decode_error!(decode_char, chunk[3], cur_line, cur_input_char+3);
            // assumes high bits are zero
            decoded.push(((buffer[0] << 2) | (buffer[1] >> 4)).into());
            let byte2 = (buffer[1] << 4) | (buffer[2] >> 2);
            if cur_output_char+1 < output_char_count {
                decoded.push(byte2.into());
            }
            let byte3 = (buffer[2] << 6) | buffer[3];
            if cur_output_char+2 < output_char_count {
                decoded.push(byte3.into());
            }

            cur_output_char += 3;
            cur_input_char += 4;
            if cur_output_char >= output_char_count {
                break;
            }
        }
        input_iter.next(); // discard newline
        cur_line += 1;
    }
}

/// Encodes a 6-bit value into a UUEncoded character.
/// Returns None if input is outside of target range.
#[inline]
pub fn encode_char(value: u8) -> Option<u8> {
    if value == 0 {
        Some(b'`')
    } else {
        value.checked_add(32)
    }
}

/// Decodes a UUEncoded character into a 6-bit value.
#[inline]
pub fn decode_char(value: u8) -> Option<u8> {
    if value == b'`' || value == 0 {
        Some(0)
    } else {
        value.checked_sub(32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test cases generated with `echo -n ITEM | uuencode -r -`

    /// Tests the uuencode function with the string "cat".
    #[test]
    fn test_cat() {
        let data = b"cat";
        let encoded = uuencode(data).unwrap();
        assert_eq!(encoded, "#8V%T", "can uuencode a small text");
    }

    /// Tests the uuencode function with the text of the public domain work, The Machine Stops
    #[test]
    fn test_the_machine_stops() {
        // Do *not* include these in the binary, it'll grow our binary by nearly *a megabyte*
        let source_data = std::fs::read("test_data/the_machine_stops.txt").expect("Can open test data");
        let expected_data = std::fs::read_to_string(&"test_data/the_machine_stops.txt.uu").expect("Can open test data").trim_end().to_string();
        let actual = uuencode(&source_data).unwrap();
        assert_eq!(actual, expected_data, "can uuencode a large text");
    }

    /// Tests the uuencode function with random binary data (from /dev/urandom)
    #[test]
    fn test_random_data() {
        let source_data = std::fs::read("test_data/random_data.bin").expect("Can open test data");
        let expected_data = std::fs::read_to_string(&"test_data/random_data.bin.uu").expect("Can open test data").trim_end().to_string();
        let actual = uuencode(&source_data).unwrap();
        assert_eq!(actual, expected_data, "can uuencode random data");
    }

    /// Tests decoding cat
    #[test]
    fn test_decode_cat() {
        let data = b"#8V%T";
        let encoded = uudecode(data).unwrap();
        assert_eq!(String::from_utf8_lossy(&encoded), "cat", "can uuencode a small text");
    }

    /// Tests decoding the text of the public domain work, The Machine Stops
    #[test]
    fn test_decode_the_machine_stops() {
        let source_data = std::fs::read("test_data/the_machine_stops.txt.uu").expect("Can open test data");
        let original_data = std::fs::read("test_data/the_machine_stops.txt").expect("Can open test data");
        let orig_as_str = String::from_utf8_lossy(&original_data);
        let decoded = uudecode(&source_data).unwrap();
        assert_eq!(String::from_utf8_lossy(&decoded), orig_as_str, "can uudecode a large text");
    }

    /// Tests decoding just the last line of The Machine Stops, to debug a particular issue.
    #[test]
    fn test_can_decode_last_line() {
        let data = b".;W)K+`HQ.38X*2X*,C4`";
        let encoded = uudecode(data).unwrap();
        assert_eq!(String::from_utf8_lossy(&encoded), "ork,\n1968).\n25", "can uudecode a small text");
    }

    /// Tests round-trip execution
    #[test]
    fn test_rt() {
        let source_data = std::fs::read("test_data/the_machine_stops.txt").expect("Can open test data");
        let source_as_string = String::from_utf8_lossy(&source_data).trim_end().to_string();
        let encoded = uuencode(&source_data).unwrap();
        let decoded = uudecode(encoded.as_bytes()).unwrap();
        assert_eq!(String::from_utf8_lossy(&decoded), source_as_string, "can uuencode and uudecode");
    }
}
