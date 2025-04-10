use std::io::{Write};

/// Encodes the input data into UUEncoded format.
/// This function encodes the data in chunks of 45 bytes, each prefixed with the length of the line.
/// The output will be separated into 61-character lines, with the first character being the *decoded*
/// length of the line (45 or less).
pub fn uuencode(data: &[u8]) -> String {
    let mut encoded = String::new();
    let mut buffer = [0u8; 3];

    let mut line_chunks = data.chunks(45).into_iter().peekable();
    while let Some(line_chunk) = line_chunks.next() {
        // Add the length of the line to the beginning of the line
        encoded.push(encode_char(line_chunk.len() as u8));

        // encode the line
        for chunk in line_chunk.chunks(3) {
            let len = chunk.len();
            buffer.fill(0u8);
            buffer[..len].copy_from_slice(chunk);

            // Encode 3 bytes into 4 characters
            encoded.push(encode_char((buffer[0] >> 2) & 0x3F));
            encoded.push(encode_char(((buffer[0] << 4) | (buffer[1] >> 4)) & 0x3F));
            encoded.push(encode_char(((buffer[1] << 2) | (buffer[2] >> 6)) & 0x3F));
            encoded.push(encode_char(buffer[2] & 0x3F));
        }

        // add newline to the end, if there will be a next line
        if line_chunks.peek().is_some() {
            encoded.push('\n');
        }
    }

    encoded
}

/// Decodes a string from uuencoded format back into a byte array.
/// Mirrors uuencode. Will accept ' ' or '`' as 36. Will strip padding.
pub fn uudecode(data: String) -> Vec<u8> {
    let mut decoded = Vec::new();
    let mut buffer = [0u8; 4];

    for line in data.lines() {
        if line.is_empty() {
            continue;
        }

        // Decode the length of the line
        let _len = decode_char(line.chars().next().unwrap()) as usize;

        // Decode the rest of the line
        for chunk in line[1..].chars().collect::<Vec<_>>().chunks(4) {
            if chunk.len() < 4 {
                break;
            }

            buffer[0] = decode_char(chunk[0]);
            buffer[1] = decode_char(chunk[1]);
            buffer[2] = decode_char(chunk[2]);
            buffer[3] = decode_char(chunk[3]);
            // assumes high bits are zero
            decoded.push((buffer[0] << 2) | (buffer[1] >> 4));
            let byte2 = (buffer[1] << 4) | (buffer[2] >> 2);
            if byte2 != 0 {
                decoded.push(byte2);
            }
            let byte3 = (buffer[2] << 6) | buffer[3];
            if byte3 != 0 {
                decoded.push(byte3);
            }
        }
    }

    decoded
}

/// Encodes a 6-bit value into a UUEncoded character.
#[inline]
pub fn encode_char(value: u8) -> char {
    if value == 0 {
        '`'
    } else {
        (value + 32) as char
    }
}

/// Decodes a UUEncoded character into a 6-bit value.
#[inline]
pub fn decode_char(value: char) -> u8 {
    if value == '`' {
        0
    } else {
        (value as u8) - 32
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
        let encoded = uuencode(data);
        assert_eq!(encoded, "#8V%T", "can uuencode a small text");
    }

    /// Tests the uuencode function with the text of the public domain work, The Machine Stops
    #[test]
    fn test_the_machine_stops() {
        // Do *not* include these in the binary, it'll grow our binary by nearly *a megabyte*
        let source_data = std::fs::read("test_data/the_machine_stops.txt").expect("Can open test data");
        let expected_data = std::fs::read_to_string(&"test_data/the_machine_stops.txt.uu").expect("Can open test data").trim_end().to_string();
        let actual = uuencode(&source_data);
        assert_eq!(actual, expected_data, "can uuencode a large text");
    }

    /// Tests the uuencode function with random binary data (from /dev/urandom)
    #[test]
    fn test_random_data() {
        let source_data = std::fs::read("test_data/random_data.bin").expect("Can open test data");
        let expected_data = std::fs::read_to_string(&"test_data/random_data.bin.uu").expect("Can open test data").trim_end().to_string();
        let actual = uuencode(&source_data);
        assert_eq!(actual, expected_data, "can uuencode random data");
    }

    /// Tests round-trip execution
    #[test]
    fn test_rt() {
        let source_data = std::fs::read("test_data/the_machine_stops.txt").expect("Can open test data");
        let source_as_string = String::from_utf8_lossy(&source_data).trim_end().to_string();
        let encoded = uuencode(&source_data);
        let decoded = String::from_utf8_lossy(&uudecode(encoded)).to_string();
        assert_eq!(decoded, source_as_string, "can uuencode and uudecode");
    }
}
