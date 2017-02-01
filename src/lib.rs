//! This crate provides an iterator that yields the bytes in an HTTP request body. In
//! particular, it provides convenience for the use case where data is read directly from
//! a `TcpStream` into a fixed-size buffer and, after the first read, the buffer contains
//! the request headers as well as some initial chunk of the request body.
//!
//! This iterator can yield the bytes in that partial chunk, then reuse the entire buffer
//! to read further body chunks and yield the bytes from those. The result can be fed, for
//! example, into a byte-based parser such as
//! [serde_json::from_iter](https://docs.serde.rs/serde_json/de/fn.from_iter.html).
//!
//! ## Example
//!
//! ```rust
//! use uhttp_body_bytes::BodyBytes;
//! use std::io::{Cursor, Read};
//!
//! // Create a sample POST request with json payload.
//! let request = b"POST / HTTP/1.1\r\nHost: w3.org\r\n\r\n{\"k\": 42}";
//! let mut stream = Cursor::new(&request[..]);
//!
//! // Simulate reading request-line/headers and partial body into a fixed-size buffer.
//! let mut buf = [0; 36];
//! let nbytes = stream.read(&mut buf[..]).unwrap();
//! assert_eq!(nbytes, 36);
//! assert_eq!(&buf[..], &b"POST / HTTP/1.1\r\nHost: w3.org\r\n\r\n{\"k"[..]);
//!
//! // Process the headers (up to byte 33.)
//! // [...]
//! let body_start = 33;
//!
//! // Start reading body after end of headers.
//! let mut bytes = BodyBytes::new(stream, &mut buf[..], body_start, nbytes);
//! assert_eq!(bytes.next().unwrap().unwrap(), b'{');
//! assert_eq!(bytes.next().unwrap().unwrap(), b'"');
//! assert_eq!(bytes.next().unwrap().unwrap(), b'k');
//! assert_eq!(bytes.next().unwrap().unwrap(), b'"');
//! assert_eq!(bytes.next().unwrap().unwrap(), b':');
//! assert_eq!(bytes.next().unwrap().unwrap(), b' ');
//! assert_eq!(bytes.next().unwrap().unwrap(), b'4');
//! assert_eq!(bytes.next().unwrap().unwrap(), b'2');
//! assert_eq!(bytes.next().unwrap().unwrap(), b'}');
//! assert!(bytes.next().is_none());
//! ```

use std::io::Read;

/// Iterator over bytes in a stream using a slice buffer.
#[derive(PartialEq, Eq, Hash, Debug)]
pub struct BodyBytes<'a, R: Read> {
    /// Underlying stream to buffer and read bytes from.
    stream: R,
    /// Buffer for writing TCP chunks into and reading bytes out of.
    buf: &'a mut [u8],
    /// Byte position in buffer to read next.
    pos: usize,
    /// Total number of valid bytes in buffer.
    len: usize,
}

impl<'a, R: Read> BodyBytes<'a, R> {
    /// Create a new `BodyBytes` to read chunks from the given stream into the given
    /// buffer and iterate over the bytes in each chunk.
    ///
    /// Before reading the first chunk from the stream, any remaining bytes in the given
    /// buffer are iterated over starting at the given position out of the given length.
    pub fn new(stream: R, buf: &'a mut [u8], start: usize, len: usize) -> Self {
        BodyBytes {
            buf: buf,
            stream: stream,
            pos: start,
            len: len,
        }
    }
}

impl<'a, R: Read> Iterator for BodyBytes<'a, R> {
    type Item = std::io::Result<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.len {
            let len = match self.stream.read(self.buf) {
                Ok(l) => l,
                Err(e) => return Some(Err(e)),
            };

            if len == 0 {
                return None;
            }

            self.pos = 0;
            self.len = len;
        }

        let b = self.buf[self.pos];
        self.pos += 1;

        Some(Ok(b))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_body_bytes() {
        let stream = b"dy text";
        let mut buf = [b'#'; 25];
        (&mut buf[..]).copy_from_slice(b"GET / HTTP/1.1\r\n\r\nsome bo");

        let mut r = BodyBytes::new(Cursor::new(&stream[..]), &mut buf[..], 18, 25);

        assert_eq!(r.next().unwrap().unwrap(), b's');
        assert_eq!(r.next().unwrap().unwrap(), b'o');
        assert_eq!(r.next().unwrap().unwrap(), b'm');
        assert_eq!(r.next().unwrap().unwrap(), b'e');
        assert_eq!(r.next().unwrap().unwrap(), b' ');
        assert_eq!(r.next().unwrap().unwrap(), b'b');
        assert_eq!(r.next().unwrap().unwrap(), b'o');
        assert_eq!(r.next().unwrap().unwrap(), b'd');
        assert_eq!(r.next().unwrap().unwrap(), b'y');
        assert_eq!(r.next().unwrap().unwrap(), b' ');
        assert_eq!(r.next().unwrap().unwrap(), b't');
        assert_eq!(r.next().unwrap().unwrap(), b'e');
        assert_eq!(r.next().unwrap().unwrap(), b'x');
        assert_eq!(r.next().unwrap().unwrap(), b't');
        assert!(r.next().is_none());

        let stream = b"abcdef";
        let mut buf = [b'#'; 4];

        let mut r = BodyBytes::new(Cursor::new(&stream[..]), &mut buf[..], 4, 4);

        assert_eq!(r.next().unwrap().unwrap(), b'a');
        assert_eq!(r.next().unwrap().unwrap(), b'b');
        assert_eq!(r.next().unwrap().unwrap(), b'c');
        assert_eq!(r.next().unwrap().unwrap(), b'd');
        assert_eq!(r.next().unwrap().unwrap(), b'e');
        assert_eq!(r.next().unwrap().unwrap(), b'f');
        assert!(r.next().is_none());

        let stream = b"cdefgh";
        let mut buf = [b'#'; 4];
        (&mut buf[..2]).copy_from_slice(b"ab");

        let mut r = BodyBytes::new(Cursor::new(&stream[..]), &mut buf[..], 0, 2);

        assert_eq!(r.next().unwrap().unwrap(), b'a');
        assert_eq!(r.next().unwrap().unwrap(), b'b');
        assert_eq!(r.next().unwrap().unwrap(), b'c');
        assert_eq!(r.next().unwrap().unwrap(), b'd');
        assert_eq!(r.next().unwrap().unwrap(), b'e');
        assert_eq!(r.next().unwrap().unwrap(), b'f');
        assert_eq!(r.next().unwrap().unwrap(), b'g');
        assert_eq!(r.next().unwrap().unwrap(), b'h');
        assert!(r.next().is_none());

        let stream = b" text";
        let mut buf = [b'#'; 25];
        (&mut buf[..22]).copy_from_slice(b"GET / HTTP/1.1\r\n\r\nsome");

        let mut r = BodyBytes::new(Cursor::new(&stream[..]), &mut buf[..], 18, 22);

        assert_eq!(r.next().unwrap().unwrap(), b's');
        assert_eq!(r.next().unwrap().unwrap(), b'o');
        assert_eq!(r.next().unwrap().unwrap(), b'm');
        assert_eq!(r.next().unwrap().unwrap(), b'e');
        assert_eq!(r.next().unwrap().unwrap(), b' ');
        assert_eq!(r.next().unwrap().unwrap(), b't');
        assert_eq!(r.next().unwrap().unwrap(), b'e');
        assert_eq!(r.next().unwrap().unwrap(), b'x');
        assert_eq!(r.next().unwrap().unwrap(), b't');
        assert!(r.next().is_none());

        let stream = b"efghijklm";
        let mut buf = [b'#'; 4];
        (&mut buf[..]).copy_from_slice(b"abcd");

        let mut r = BodyBytes::new(Cursor::new(&stream[..]), &mut buf[..], 2, 4);

        assert_eq!(r.next().unwrap().unwrap(), b'c');
        assert_eq!(r.next().unwrap().unwrap(), b'd');
        assert_eq!(r.next().unwrap().unwrap(), b'e');
        assert_eq!(r.next().unwrap().unwrap(), b'f');
        assert_eq!(r.next().unwrap().unwrap(), b'g');
        assert_eq!(r.next().unwrap().unwrap(), b'h');
        assert_eq!(r.next().unwrap().unwrap(), b'i');
        assert_eq!(r.next().unwrap().unwrap(), b'j');
        assert_eq!(r.next().unwrap().unwrap(), b'k');
        assert_eq!(r.next().unwrap().unwrap(), b'l');
        assert_eq!(r.next().unwrap().unwrap(), b'm');
        assert!(r.next().is_none());
    }
}
