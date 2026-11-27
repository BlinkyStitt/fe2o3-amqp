use std::io;

use crate::error::Error;

use super::{private, Read};

/// A reader for IO stream
#[derive(Debug)]
pub struct IoReader<R> {
    // an io reader
    reader: R,
    buf: Vec<u8>,
}

impl<R: io::Read> IoReader<R> {
    /// Creates a new reader over IO stream
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buf: Vec::new(),
        }
    }

    /// Pop the first byte
    pub fn pop_first(&mut self) -> Option<u8> {
        match self.buf.is_empty() {
            true => None,
            false => Some(self.buf.remove(0)),
        }
    }

    /// Fill the internal buffer with the given length
    pub fn fill_buffer(&mut self, len: usize) -> Result<(), Error> {
        self.fill_buffer_io(len).map_err(Into::into)
    }

    fn fill_buffer_io(&mut self, len: usize) -> io::Result<()> {
        let mut filled = self.buf.len();
        if filled >= len {
            return Ok(());
        }
        self.buf.resize(len, 0);
        while filled < len {
            match self.reader.read(&mut self.buf[filled..]) {
                Ok(0) => {
                    self.buf.truncate(filled);
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "failed to fill whole buffer",
                    ));
                }
                Ok(n) => filled += n,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    self.buf.truncate(filled);
                    return Err(error);
                }
            }
        }
        Ok(())
    }
}

impl<R: io::Read> private::Sealed for IoReader<R> {}

impl<'de, R: io::Read + 'de> Read<'de> for IoReader<R> {
    fn peek(&mut self) -> Option<u8> {
        match self.buf.first() {
            Some(b) => Some(*b),
            None => {
                let mut buf = [0u8; 1];
                match self.reader.read_exact(&mut buf) {
                    Ok(_) => {
                        self.buf.push(buf[0]);
                        Some(buf[0])
                    }
                    Err(_) => None, // EOF
                }
            }
        }
    }

    fn peek_bytes(&mut self, n: usize) -> Option<&[u8]> {
        let l = self.buf.len();
        if l < n {
            match self.fill_buffer(n) {
                Ok(_) => Some(&self.buf[..n]),
                Err(_) => None,
            }
        } else {
            Some(&self.buf[..n])
        }
    }

    fn next(&mut self) -> Option<u8> {
        match self.pop_first() {
            Some(b) => Some(b),
            None => {
                let mut buf = [0u8; 1];
                match self.reader.read_exact(&mut buf) {
                    Ok(_) => Some(buf[0]),
                    Err(_) => None, // EOF
                }
            }
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), io::Error> {
        let n = buf.len();
        self.fill_buffer_io(n)?;
        buf.copy_from_slice(&self.buf[..n]);
        self.buf.drain(..n);
        Ok(())
    }

    fn forward_read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.fill_buffer(len)?;
        let result = visitor.visit_bytes(&self.buf[..len]);
        self.buf.drain(..len);
        result
    }

    fn forward_read_str<V>(&mut self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.fill_buffer(len)?;
        let s = std::str::from_utf8(&self.buf[..len])?;
        let result = visitor.visit_str(s);
        self.buf.drain(..len);
        result
    }
}

#[allow(clippy::all)]
#[cfg(test)]
mod tests {
    use crate::read::ioread::IoReader;

    use super::Read;

    const SHORT_BUFFER: &[u8] = &[0, 1, 2];
    const LONG_BUFFER: &[u8] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    ];

    #[test]
    fn test_peek() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(reader);

        let peek0 = io_reader.peek().expect("Should not return error");
        let peek1 = io_reader.peek().expect("Should not return error");
        let peek2 = io_reader.peek().expect("Should not return error");

        assert_eq!(peek0, reader[0]);
        assert_eq!(peek1, reader[0]);
        assert_eq!(peek2, reader[0]);
    }

    #[test]
    fn test_next() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(reader);

        for i in 0..reader.len() {
            let peek = io_reader.peek().expect("Should not return error");
            let next = io_reader.next().expect("Should not return error");

            assert_eq!(peek, reader[i]);
            assert_eq!(next, reader[i]);
        }

        let peek_none = io_reader.peek();
        let next_none = io_reader.next();

        assert!(peek_none.is_none());
        assert!(next_none.is_none());
    }

    #[test]
    fn test_read_const_bytes_without_peek() {
        let reader = LONG_BUFFER;
        let mut io_reader = IoReader::new(reader);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader
            .read_const_bytes::<N>()
            .expect("Should not return error");
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &reader[..N]);

        // Read the second bytes
        let bytes = io_reader
            .read_const_bytes::<N>()
            .expect("Should not return error");
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &reader[(N)..(2 * N)]);

        // Read None
        let bytes = io_reader.read_const_bytes::<N>();
        assert!(bytes.is_none());
    }

    #[test]
    fn test_incomplete_read_const_bytes_without_peek() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(std::io::Cursor::new(reader));

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader.read_const_bytes::<N>();
        assert!(bytes.is_none());

        for i in 0..reader.len() {
            let peek = io_reader.peek().expect("Should not return error");
            let next = io_reader.next().expect("Should not return error");

            assert_eq!(peek, reader[i]);
            assert_eq!(next, reader[i]);
        }

        let peek_none = io_reader.peek();
        let next_none = io_reader.next();

        assert!(peek_none.is_none());
        assert!(next_none.is_none());
    }

    #[test]
    fn test_read_const_bytes_after_peek() {
        let reader = LONG_BUFFER;
        let mut io_reader = IoReader::new(reader);

        let peek0 = io_reader.peek().expect("Should not return error");
        assert_eq!(peek0, reader[0]);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader
            .read_const_bytes::<N>()
            .expect("Should not return error");
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &reader[..N]);

        // Read the second bytes
        let bytes = io_reader
            .read_const_bytes::<N>()
            .expect("Should not return error");
        assert_eq!(bytes.len(), N);
        assert_eq!(&bytes[..], &reader[(N)..(2 * N)]);

        // Read None
        let bytes = io_reader.read_const_bytes::<N>();
        assert!(bytes.is_none());
    }

    #[test]
    fn test_incomplete_read_const_bytes_after_peek() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(std::io::Cursor::new(reader));

        let peek0 = io_reader.peek().expect("Should not return error");
        assert_eq!(peek0, reader[0]);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader.read_const_bytes::<N>();
        assert!(bytes.is_none());

        for i in 0..reader.len() {
            let peek = io_reader.peek().expect("Should not return error");
            let next = io_reader.next().expect("Should not return error");

            assert_eq!(peek, reader[i]);
            assert_eq!(next, reader[i]);
        }

        let peek_err = io_reader.peek();
        let next_err = io_reader.next();

        assert!(peek_err.is_none());
        assert!(next_err.is_none());
    }

    #[test]
    fn test_peek_bytes() {
        let mut reader = IoReader::new(SHORT_BUFFER);
        let peek0 = reader.peek_bytes(2).unwrap().to_vec();
        let peek1 = reader.peek_bytes(2).unwrap();

        assert_eq!(peek0, &SHORT_BUFFER[..2]);
        assert_eq!(peek1, &SHORT_BUFFER[..2]);
    }

    #[test]
    fn failed_peek_preserves_partial_bytes_without_padding() {
        let mut reader = IoReader::new(std::io::Cursor::new(SHORT_BUFFER));
        assert!(reader.peek_bytes(10).is_none());
        for &byte in SHORT_BUFFER {
            assert_eq!(reader.next(), Some(byte));
        }
        assert!(reader.next().is_none());
    }

    #[test]
    fn read_exact_retries_interrupted_reads() {
        struct InterruptedOnce {
            reader: std::io::Cursor<&'static [u8]>,
            interrupted: bool,
        }
        impl std::io::Read for InterruptedOnce {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if !self.interrupted {
                    self.interrupted = true;
                    return Err(std::io::ErrorKind::Interrupted.into());
                }
                let len = buf.len().min(1);
                std::io::Read::read(&mut self.reader, &mut buf[..len])
            }
        }
        let mut reader = IoReader::new(InterruptedOnce {
            reader: std::io::Cursor::new(SHORT_BUFFER),
            interrupted: false,
        });
        assert_eq!(reader.read_const_bytes::<3>(), Some([0, 1, 2]));
        assert!(reader.next().is_none());
    }
}
