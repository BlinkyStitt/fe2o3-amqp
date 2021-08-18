use std::io;

use crate::error::Error;

mod private {
    pub trait Sealed {}
}

pub trait Read<'de>: private::Sealed {
    /// Peek the next byte without consuming
    fn peek(&mut self) -> Result<u8, Error>;

    /// Read the next byte
    fn next(&mut self) -> Result<u8, Error>;

    /// Read n bytes
    /// 
    /// Prefered to use this when the size is small and can be stack allocated
    fn read_const_bytes<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        // let mut buf = vec![0; n];
        let mut buf = [0; N];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, Error> {
        let mut buf = vec![0; n];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Read to fill a mutable buffer
    fn read_exact(&mut self, out: &mut [u8]) -> Result<(), io::Error>;
}

pub struct IoReader<R> {
    // an io reader
    reader: R,
    // a temporarty buffer holding the next byte
    next_byte: Option<u8>,
}

impl<R: io::Read> IoReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            next_byte: None,
        }
    }
}

impl<R: io::Read> private::Sealed for IoReader<R> {}

impl<'de, R: io::Read> Read<'de> for IoReader<R> {
    fn peek(&mut self) -> Result<u8, Error> {
        match self.next_byte {
            Some(b) => Ok(b),
            None => {
                let mut buf = [0u8; 1];
                self.reader.read_exact(&mut buf)?;
                Ok(buf[0])
            }
        }
    }

    fn next(&mut self) -> Result<u8, Error> {
        match self.next_byte.take() {
            Some(b) => Ok(b),
            None => {
                let mut buf = [0u8; 1];
                self.reader.read_exact(&mut buf)?;
                Ok(buf[0])
            }
        }
    }

    fn read_exact(&mut self, out: &mut [u8]) -> Result<(), io::Error> {
        let result = match self.next_byte {
            Some(b) => {
                out[0] = b;
                self.reader.read_exact(&mut out[1..])
            }
            None => self.reader.read_exact(out),
        };

        if result.is_ok() {
            self.next_byte.take();
        }
        result
    }
}

#[inline]
fn map_eof_to_none(err: io::Error) -> Option<Result<u8, Error>> {
    if let io::ErrorKind::UnexpectedEof = err.kind() {
        None
    } else {
        Some(Err(err.into()))
    }
}

#[cfg(test)]
mod tests {
    use crate::read::IoReader;

    use super::Read;

    const SHORT_BUFFER: &[u8] = &[0, 1, 2];
    const LONG_BUFFER: &[u8] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    ];

    #[test]
    fn test_peek() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(reader);

        let peek0 = io_reader
            .peek()
            .expect("Should not return error");
        let peek1 = io_reader
            .peek()
            .expect("Should not return error");
        let peek2 = io_reader
            .peek()
            .expect("Should not return error");

        assert_eq!(peek0, reader[0]);
        assert_eq!(peek1, reader[0]);
        assert_eq!(peek2, reader[0]);
    }

    #[test]
    fn test_next() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(reader);

        for i in 0..reader.len() {
            let peek = io_reader
                .peek()
                .expect("Should not return error");
            let next = io_reader
                .next()
                .expect("Should not return error");

            assert_eq!(peek, reader[i]);
            assert_eq!(next, reader[i]);
        }

        let peek_none = io_reader.peek();
        let next_none = io_reader.next();

        assert!(peek_none.is_err());
        assert!(next_none.is_err());
    }

    #[test]
    fn test_read_const_bytes_without_peek() {
        let reader = LONG_BUFFER;
        let mut io_reader = IoReader::new(reader);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader
            .read_const_bytes::<10>()
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
        assert!(bytes.is_err());
    }

    #[test]
    fn test_incomplete_read_const_bytes_without_peek() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(std::io::Cursor::new(reader));

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader.read_const_bytes::<N>();
        assert!(bytes.is_err());

        for i in 0..reader.len() {
            let peek = io_reader
                .peek()
                .expect("Should not return error");
            let next = io_reader
                .next()
                .expect("Should not return error");

            assert_eq!(peek, reader[i]);
            assert_eq!(next, reader[i]);
        }

        let peek_none = io_reader.peek();
        let next_none = io_reader.next();

        assert!(peek_none.is_err());
        assert!(next_none.is_err());
    }

    #[test]
    fn test_read_const_bytes_after_peek() {
        let reader = LONG_BUFFER;
        let mut io_reader = IoReader::new(reader);

        let peek0 = io_reader
            .peek()
            .expect("Should not return error");
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
        assert!(bytes.is_err());
    }

    #[test]
    fn test_incomplete_read_const_bytes_after_peek() {
        let reader = SHORT_BUFFER;
        let mut io_reader = IoReader::new(std::io::Cursor::new(reader));

        let peek0 = io_reader
            .peek()
            .expect("Should not return error");
        assert_eq!(peek0, reader[0]);

        // Read first 10 bytes
        const N: usize = 10;
        let bytes = io_reader.read_const_bytes::<N>();
        assert!(bytes.is_err());

        for i in 0..reader.len() {
            let peek = io_reader
                .peek()
                .expect("Should not return error");
            let next = io_reader
                .next()
                .expect("Should not return error");

            assert_eq!(peek, reader[i]);
            assert_eq!(next, reader[i]);
        }

        let peek_none = io_reader.peek();
        let next_none = io_reader.next();

        assert!(peek_none.is_err());
        assert!(next_none.is_err());
    }
}