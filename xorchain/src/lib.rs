//! Streaming prefix-XOR encoder/decoder.
//!
//! The on-disk format is:
//!
//! ```text
//! R0 | (R0 xor S0) | (R0 xor S0 xor S1) | ...
//! ```
//!
//! where `R0` is an 8-byte random value and `S0..Sn` are consecutive
//! chunks of at most 8 bytes from the input.  The final chunk keeps its
//! original length, so the output length is always `8 + input length`.
//!
//! This is a reversible data transform, **not encryption**: R0 is stored
//! in the output and the transform is linear.

use std::io::{self, ErrorKind, Read, Write};

/// Size of R0 and of every full input block.
pub const BLOCK_SIZE: usize = 8;

/// Encode `src` into `dst`, generating a fresh random 8-byte R0.
///
/// The function streams the input and uses O(1) memory.
pub fn encode<R: Read, W: Write>(src: &mut R, dst: &mut W) -> io::Result<()> {
    let mut r0 = [0u8; BLOCK_SIZE];
    getrandom::getrandom(&mut r0)
        .map_err(|err| io::Error::other(format!("failed to generate R0: {err}")))?;
    encode_with_r0(src, dst, &r0)
}

/// Decode `src` into `dst`.
///
/// The first 8 bytes of `src` are R0, followed by the encoded payload.
/// An input shorter than 8 bytes is rejected with `UnexpectedEof`.
pub fn decode<R: Read, W: Write>(src: &mut R, dst: &mut W) -> io::Result<()> {
    let mut r0 = [0u8; BLOCK_SIZE];
    let read = read_block(src, &mut r0)?;

    if read != BLOCK_SIZE {
        return Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            "input is shorter than the 8-byte R0 header",
        ));
    }

    decode_payload(src, dst, &r0)
}

/// Encode with a caller-provided R0.  Primarily useful for tests and for
/// callers that need deterministic output.
pub fn encode_with_r0<R: Read, W: Write>(
    src: &mut R,
    dst: &mut W,
    r0: &[u8; BLOCK_SIZE],
) -> io::Result<()> {
    dst.write_all(r0)?;

    let mut state = *r0;
    let mut block = [0u8; BLOCK_SIZE];

    loop {
        let read = read_block(src, &mut block)?;
        if read == 0 {
            break;
        }

        for i in 0..read {
            state[i] ^= block[i];
        }
        dst.write_all(&state[..read])?;

        if read < BLOCK_SIZE {
            break;
        }
    }

    Ok(())
}

/// Decode a payload whose R0 header has already been consumed.
fn decode_payload<R: Read, W: Write>(
    src: &mut R,
    dst: &mut W,
    r0: &[u8; BLOCK_SIZE],
) -> io::Result<()> {
    let mut previous_ciphertext = *r0;
    let mut block = [0u8; BLOCK_SIZE];

    loop {
        let read = read_block(src, &mut block)?;
        if read == 0 {
            break;
        }

        // Keep the ciphertext before overwriting `block` with plaintext.
        let mut ciphertext = [0u8; BLOCK_SIZE];
        ciphertext[..read].copy_from_slice(&block[..read]);

        for i in 0..read {
            block[i] ^= previous_ciphertext[i];
        }
        dst.write_all(&block[..read])?;

        // For the common full-block case this is the next chain value.
        // For a short final block there is no next block.
        previous_ciphertext = ciphertext;

        if read < BLOCK_SIZE {
            break;
        }
    }

    Ok(())
}

/// Read up to 8 bytes, tolerating short reads and `Interrupted`.
///
/// Returns the number of bytes read.  A value smaller than 8 means EOF
/// was reached; `0` means EOF before any byte was read.
fn read_block<R: Read>(src: &mut R, buf: &mut [u8; BLOCK_SIZE]) -> io::Result<usize> {
    let mut read = 0;

    while read < BLOCK_SIZE {
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

    const TEST_R0: [u8; BLOCK_SIZE] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];

    fn deterministic_encode(data: &[u8]) -> Vec<u8> {
        let mut encoded = Vec::new();
        encode_with_r0(&mut Cursor::new(data), &mut encoded, &TEST_R0).unwrap();
        encoded
    }

    fn data_of_len(len: usize) -> Vec<u8> {
        (0..len)
            .map(|i| (i.wrapping_mul(31).wrapping_add(7)) as u8)
            .collect()
    }

    #[test]
    fn output_starts_with_r0_and_uses_prefix_xor() {
        let data = b"ABCDEFGHIJKLMNOPQRST"; // 20 bytes: 8 + 8 + 4
        let encoded = deterministic_encode(data);

        assert_eq!(&encoded[..BLOCK_SIZE], &TEST_R0);
        assert_eq!(encoded.len(), data.len() + BLOCK_SIZE);

        let mut state = TEST_R0;
        for (index, chunk) in data.chunks(BLOCK_SIZE).enumerate() {
            for i in 0..chunk.len() {
                state[i] ^= chunk[i];
            }
            let start = BLOCK_SIZE + index * BLOCK_SIZE;
            assert_eq!(&encoded[start..start + chunk.len()], &state[..chunk.len()]);
        }
    }

    #[test]
    fn roundtrip_various_lengths() {
        for len in [0, 1, 2, 7, 8, 9, 15, 16, 17, 31, 32, 33, 4096, 65537] {
            let data = data_of_len(len);

            let mut encoded = Vec::new();
            encode_with_r0(&mut Cursor::new(data.as_slice()), &mut encoded, &TEST_R0).unwrap();
            assert_eq!(encoded.len(), data.len() + BLOCK_SIZE, "len={len}");

            let mut decoded = Vec::new();
            decode(&mut Cursor::new(encoded.as_slice()), &mut decoded).unwrap();
            assert_eq!(decoded, data, "len={len}");
        }
    }

    #[test]
    fn random_r0_encode_roundtrips() {
        let data = data_of_len(12345);
        let mut encoded = Vec::new();
        encode(&mut Cursor::new(data.as_slice()), &mut encoded).unwrap();

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
        encode_with_r0(
            &mut OneByteReader(Cursor::new(data.as_slice())),
            &mut encoded,
            &TEST_R0,
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
    fn empty_input_produces_only_r0() {
        let encoded = deterministic_encode(b"");
        assert_eq!(encoded, TEST_R0);

        let mut decoded = Vec::new();
        decode(&mut Cursor::new(encoded.as_slice()), &mut decoded).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn rejects_input_shorter_than_r0() {
        for len in 0..BLOCK_SIZE {
            let mut decoded = Vec::new();
            let err = decode(&mut Cursor::new(vec![0u8; len]), &mut decoded).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::UnexpectedEof, "len={len}");
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
