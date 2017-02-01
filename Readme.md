# uhttp\_body\_bytes -- Iterator for HTTP request body bytes

[Documentation](https://docs.rs/uhttp_body_bytes)

This crate provides an iterator that yields the bytes in an HTTP request body. In
particular, it provides convenience for the use case where data is read directly from
a `TcpStream` into a fixed-size buffer and, after the first read, the buffer contains
the request headers as well as some initial chunk of the request body.

This iterator can yield the bytes in that partial chunk, then reuse the entire buffer
to read further body chunks and yield the bytes from those. The result can be fed, for
example, into a byte-based parser such as
[serde_json::from_iter](https://docs.serde.rs/serde_json/de/fn.from_iter.html).

## Example

```rust
use uhttp_body_bytes::BodyBytes;
use std::io::{Cursor, Read};

// Create a sample POST request with json payload.
let request = b"POST / HTTP/1.1\r\nHost: w3.org\r\n\r\n{\"k\": 42}";
let mut stream = Cursor::new(&request[..]);

// Simulate reading request-line/headers and partial body into a fixed-size buffer.
let mut buf = [0; 36];
let nbytes = stream.read(&mut buf[..]).unwrap();
assert_eq!(nbytes, 36);
assert_eq!(&buf[..], &b"POST / HTTP/1.1\r\nHost: w3.org\r\n\r\n{\"k"[..]);

// Process the headers (up to byte 33.)
// [...]
let body_start = 33;

// Start reading body after end of headers.
let mut bytes = BodyBytes::new(stream, &mut buf[..], body_start, nbytes);
assert_eq!(bytes.next().unwrap().unwrap(), b'{');
assert_eq!(bytes.next().unwrap().unwrap(), b'"');
assert_eq!(bytes.next().unwrap().unwrap(), b'k');
assert_eq!(bytes.next().unwrap().unwrap(), b'"');
assert_eq!(bytes.next().unwrap().unwrap(), b':');
assert_eq!(bytes.next().unwrap().unwrap(), b' ');
assert_eq!(bytes.next().unwrap().unwrap(), b'4');
assert_eq!(bytes.next().unwrap().unwrap(), b'2');
assert_eq!(bytes.next().unwrap().unwrap(), b'}');
assert!(bytes.next().is_none());
```

## Usage

This [crate](https://crates.io/crates/uhttp_body_bytes) can be used through cargo by
adding it as a dependency in `Cargo.toml`:

```toml
[dependencies]
uhttp_body_bytes = "0.5.2"
```
and importing it in the crate root:

```rust
extern crate uhttp_body_bytes;
```
