use std::io::{self, Cursor, Error, Seek, SeekFrom, Write};

pub struct FullBufWriter<W: Write> {
    data: Cursor<Vec<u8>>,
    writer: W,
    finalized: bool,
}

impl<W: Write> Write for FullBufWriter<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.data.write(buf)
    }
    fn flush(&mut self) -> Result<(), Error> {
        self.data.flush()
    }
}

impl<W: Write> Seek for FullBufWriter<W> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
        self.data.seek(pos)
    }
}

impl<W: Write> FullBufWriter<W> {
    fn copy_content(&mut self) -> Result<u64, Error> {
        self.data.seek(SeekFrom::Start(0))?;
        io::copy(&mut self.data, &mut self.writer)
    }

    pub fn finalize(mut self) -> Result<u64, Error> {
        self.finalized = true;
        self.copy_content()
    }

    pub fn new(writer: W) -> FullBufWriter<W> {
        FullBufWriter {
            data: Cursor::new(vec![]),
            writer,
            finalized: false,
        }
    }
}

impl<W: Write> Drop for FullBufWriter<W> {
    fn drop(&mut self) {
        if !self.finalized {
            self.copy_content().unwrap();
        }
    }
}
