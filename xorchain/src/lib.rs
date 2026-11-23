//! Streaming prefix-XOR encoder/decoder with a periodic random key.
//!
//! The on-disk format is:
//!
//! ```text
//! n (u32, little endian) | K0 .. K(n-1) | C0 | C1 | C2 | ...
//! ```
//!
//! where `K0..K(n-1)` is the random key and `S0..Sm` are the input bytes.
//! Byte `i` belongs to column `i mod n` and is chained with the byte `n`
//! positions before it:
//!
//! ```text
//! C_i = K(i mod n) xor S_i xor S(i-n) xor S(i-2n) xor ...
//! ```
//!
//! so the stream is split into groups of `n` bytes and each column of those
//! groups forms its own prefix-XOR chain.  With `n = 3` the first two groups
//! of output bytes are
//!
//! ```text
//! K0 xor S0,           K1 xor S1,           K2 xor S2,
//! K0 xor S0 xor S3,    K1 xor S1 xor S4,    K2 xor S2 xor S5, ...
//! ```
//!
//! and the output length is always `4 + n + input length`.
//!
//! This is a reversible data transform, **not encryption**: the key is
//! stored in the output and the transform is linear.

use std::io::{self, ErrorKind, Read, Write};

/// Default key period in bytes.
///
/// A prime keeps the groups from aligning with power-of-two record sizes in
/// the input.  4099 bytes (a prime) is small enough that the header overhead
/// stays modest even for short files.
pub const DEFAULT_PERIOD: u32 = 4099;

/// Largest key period accepted, to bound the header allocation.
pub const MAX_PERIOD: u32 = 1 << 20;

/// Size of the streaming buffer.
const BUFFER_SIZE: usize = 64 * 1024;

/// Encode `src` into `dst` using a fresh random key of `period` bytes.
///
/// The function streams the input and uses O(period) memory.
pub fn encode<R: Read, W: Write>(src: &mut R, dst: &mut W, period: u32) -> io::Result<()> {
    let period = check_period(period, ErrorKind::InvalidInput)?;

    let mut key = vec![0u8; period];
    getrandom::getrandom(&mut key)
        .map_err(|err| io::Error::other(format!("failed to generate the key: {err}")))?;

    encode_with_key(src, dst, &key)
}

/// Decode `src` into `dst`.
///
/// The period and the key are read from the header, so the caller does not
/// need to know them.  Input shorter than `4 + n` bytes is rejected with
/// `UnexpectedEof`.
pub fn decode<R: Read, W: Write>(src: &mut R, dst: &mut W) -> io::Result<()> {
    let mut header = [0u8; 4];
    let read = read_up_to(src, &mut header)?;
    if read < header.len() {
        return Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            "input is shorter than the 4-byte period header",
        ));
    }

    let period = check_period(u32::from_le_bytes(header), ErrorKind::InvalidData)?;

    let mut key = vec![0u8; period];
    let read = read_up_to(src, &mut key)?;
    if read < period {
        return Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            "input is shorter than the key described by the header",
        ));
    }

    decode_payload(src, dst, &key)
}

/// Encode with a caller-provided key.  Primarily useful for tests and for
/// callers that need deterministic output.
///
/// The key length is the period written to the header, so it must be in
/// `1..=MAX_PERIOD`.
pub fn encode_with_key<R: Read, W: Write>(src: &mut R, dst: &mut W, key: &[u8]) -> io::Result<()> {
    let period = check_period(len_as_period(key.len()), ErrorKind::InvalidInput)?;
    debug_assert_eq!(period, key.len());

    dst.write_all(&(period as u32).to_le_bytes())?;
    dst.write_all(key)?;

    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut column_state = vec![0u8; period];
    let mut column = 0usize;

    loop {
        let read = read_up_to(src, &mut buffer)?;
        if read == 0 {
            break;
        }

        for byte in &mut buffer[..read] {
            column_state[column] ^= *byte;
            *byte = column_state[column] ^ key[column];

            column += 1;
            if column == period {
                column = 0;
            }
        }

        dst.write_all(&buffer[..read])?;
    }

    Ok(())
}

/// Decode a payload whose header has already been consumed.
fn decode_payload<R: Read, W: Write>(src: &mut R, dst: &mut W, key: &[u8]) -> io::Result<()> {
    let period = key.len();
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut column_state = vec![0u8; period];
    let mut column = 0usize;

    loop {
        let read = read_up_to(src, &mut buffer)?;
        if read == 0 {
            break;
        }

        for byte in &mut buffer[..read] {
            // P_i is the running XOR of column i mod n, C_i = K(i mod n) xor P_i.
            let prefix = *byte ^ key[column];
            *byte = prefix ^ column_state[column];
            column_state[column] = prefix;

            column += 1;
            if column == period {
                column = 0;
            }
        }

        dst.write_all(&buffer[..read])?;
    }

    Ok(())
}

fn len_as_period(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

fn check_period(period: u32, kind: ErrorKind) -> io::Result<usize> {
    if period == 0 {
        return Err(io::Error::new(kind, "key period must be at least 1 byte"));
    }
    if period > MAX_PERIOD {
        return Err(io::Error::new(
            kind,
            format!("key period {period} exceeds the limit of {MAX_PERIOD} bytes"),
        ));
    }

    Ok(period as usize)
}

/// Fill `buf` as far as the input allows, tolerating short reads and
/// `Interrupted`.
///
/// Returns the number of bytes read.  A value smaller than `buf.len()`
/// means EOF was reached; `0` means EOF before any byte was read.
fn read_up_to<R: Read>(src: &mut R, buf: &mut [u8]) -> io::Result<usize> {
    let mut read = 0;

    while read < buf.len() {
        match src.read(&mut buf[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }

    Ok(read)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const TEST_KEY: [u8; 3] = [0x10, 0x20, 0x30];

    fn deterministic_encode(data: &[u8]) -> Vec<u8> {
        let mut encoded = Vec::new();
        encode_with_key(&mut Cursor::new(data), &mut encoded, &TEST_KEY).unwrap();
        encoded
    }

    fn data_of_len(len: usize) -> Vec<u8> {
        (0..len)
            .map(|i| (i.wrapping_mul(31).wrapping_add(7)) as u8)
            .collect()
    }

    fn roundtrip(data: &[u8], key: &[u8]) {
        let mut encoded = Vec::new();
        encode_with_key(&mut Cursor::new(data), &mut encoded, key).unwrap();
        assert_eq!(
            encoded.len(),
            4 + key.len() + data.len(),
            "data={}",
            data.len()
        );

        let mut decoded = Vec::new();
        decode(&mut Cursor::new(encoded.as_slice()), &mut decoded).unwrap();
        assert_eq!(decoded, data, "data={}", data.len());
    }

    #[test]
    fn header_holds_period_and_key() {
        let encoded = deterministic_encode(b"hello");
        assert_eq!(&encoded[..4], &(TEST_KEY.len() as u32).to_le_bytes());
        assert_eq!(&encoded[4..7], &TEST_KEY);
        assert_eq!(encoded.len(), 4 + TEST_KEY.len() + 5);
    }

    #[test]
    fn ciphertext_chains_within_each_column() {
        let data = b"ABCDEFGH";
        let encoded = deterministic_encode(data);
        let payload = &encoded[4 + TEST_KEY.len()..];

        for (i, byte) in data.iter().enumerate() {
            let mut expected = TEST_KEY[i % TEST_KEY.len()] ^ byte;
            let mut previous = i;
            while previous >= TEST_KEY.len() {
                previous -= TEST_KEY.len();
                expected ^= data[previous];
            }
            assert_eq!(payload[i], expected, "index {i}");
        }
    }

    #[test]
    fn matches_the_documented_n_equals_3_layout() {
        let key = [0xb0u8, 0xb1, 0xb2];
        let data = [1u8, 2, 3, 4, 5, 6];
        let mut encoded = Vec::new();
        encode_with_key(&mut Cursor::new(data.as_slice()), &mut encoded, &key).unwrap();

        let payload = &encoded[4 + key.len()..];

        // Group 1: each byte only carries its own column's prefix.
        assert_eq!(payload[0], 0xb0 ^ 1);
        assert_eq!(payload[1], 0xb1 ^ 2);
        assert_eq!(payload[2], 0xb2 ^ 3);

        // Group 2: each column continues its own chain across the boundary.
        assert_eq!(payload[3], 0xb0 ^ 1 ^ 4);
        assert_eq!(payload[4], 0xb1 ^ 2 ^ 5);
        assert_eq!(payload[5], 0xb2 ^ 3 ^ 6);
    }

    #[test]
    fn roundtrips_various_lengths_and_periods() {
        for period in [1usize, 2, 3, 8, 4099] {
            let key: Vec<u8> = (0..period).map(|i| (i * 17 + 3) as u8).collect();
            for len in [0, 1, 2, 3, 7, 8, 9, 15, 16, 17, 4095, 4096, 65537] {
                roundtrip(&data_of_len(len), &key);
            }
        }
    }

    #[test]
    fn key_longer_than_input_roundtrips() {
        roundtrip(b"abc", &[9u8; 4099]);
    }

    #[test]
    fn random_key_encode_roundtrips() {
        let data = data_of_len(12345);
        let mut encoded = Vec::new();
        encode(
            &mut Cursor::new(data.as_slice()),
            &mut encoded,
            DEFAULT_PERIOD,
        )
        .unwrap();
        assert_eq!(
            &encoded[..4],
            &DEFAULT_PERIOD.to_le_bytes(),
            "header must name the default period"
        );

        let mut decoded = Vec::new();
        decode(&mut Cursor::new(encoded.as_slice()), &mut decoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn handles_one_byte_reads() {
        struct OneByteReader<R>(R);

        impl<R: Read> Read for OneByteReader<R> {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                if buf.is_empty() {
                    return Ok(0);
                }
                self.0.read(&mut buf[..1])
            }
        }

        let data = data_of_len(37);

        let mut encoded = Vec::new();
        encode_with_key(
            &mut OneByteReader(Cursor::new(data.as_slice())),
            &mut encoded,
            &TEST_KEY,
        )
        .unwrap();

        let mut decoded = Vec::new();
        decode(
            &mut OneByteReader(Cursor::new(encoded.as_slice())),
            &mut decoded,
        )
        .unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn empty_input_produces_only_the_header() {
        let encoded = deterministic_encode(b"");
        assert_eq!(encoded.len(), 4 + TEST_KEY.len());

        let mut decoded = Vec::new();
        decode(&mut Cursor::new(encoded.as_slice()), &mut decoded).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn rejects_zero_period() {
        let mut out = Vec::new();
        let err = encode(&mut Cursor::new(b"abc"), &mut out, 0).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);

        let mut decoded = Vec::new();
        let encoded = [0u8; 4];
        let err = decode(&mut Cursor::new(encoded), &mut decoded).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_period_above_limit() {
        let mut out = Vec::new();
        let err = encode(&mut Cursor::new(b"abc"), &mut out, MAX_PERIOD + 1).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);

        let mut decoded = Vec::new();
        let err = decode(
            &mut Cursor::new((MAX_PERIOD + 1).to_le_bytes()),
            &mut decoded,
        )
        .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_input_shorter_than_header() {
        for len in 0..4 {
            let mut decoded = Vec::new();
            let err = decode(&mut Cursor::new(vec![0u8; len]), &mut decoded).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::UnexpectedEof, "len={len}");
        }

        // Period announced but the key bytes are missing.
        let mut truncated = 4u32.to_le_bytes().to_vec();
        truncated.extend_from_slice(&[1, 2]);
        let mut decoded = Vec::new();
        let err = decode(&mut Cursor::new(truncated), &mut decoded).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
    }

    #[test]
    fn corrupted_byte_only_affects_its_pair() {
        // P_i = C_i xor K(i mod n) comes straight from the ciphertext, so a
        // flipped C_i changes S_i and S(i+n) (the next element of the same
        // column) and leaves every other byte alone.
        let data = data_of_len(12);
        let mut encoded = deterministic_encode(&data);
        let flipped = 1;
        encoded[4 + TEST_KEY.len() + flipped] ^= 0xff;

        let mut decoded = Vec::new();
        decode(&mut Cursor::new(encoded.as_slice()), &mut decoded).unwrap();

        for i in 0..data.len() {
            if i == flipped || i == flipped + TEST_KEY.len() {
                assert_ne!(decoded[i], data[i], "byte {i} should change");
            } else {
                assert_eq!(decoded[i], data[i], "byte {i} should be intact");
            }
        }
    }

    #[test]
    fn truncated_payload_decodes_to_shorter_file() {
        // There is no integrity check by design; a truncated payload is just
        // a shorter valid payload.  This test documents that behavior.
        let data = b"hello world";
        let mut encoded = deterministic_encode(data);
        encoded.pop();

        let mut decoded = Vec::new();
        decode(&mut Cursor::new(encoded.as_slice()), &mut decoded).unwrap();
        assert_eq!(decoded, &data[..data.len() - 1]);
    }
}
