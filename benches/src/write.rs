// Copyright (c) 2026 Moonbase 2090 LLC
// SPDX-License-Identifier: MIT

use std::io::{self, Write};

/// Counts of the calls made through a [`CountingWriter`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WriteStats {
    /// Calls to `write`, including ones that failed.
    pub writes: usize,
    /// Bytes the inner writer accepted.
    pub bytes: usize,
    /// Calls to `flush`.
    pub flushes: usize,
}

/// Wraps a writer and counts the `write()` and `flush()` calls made through it.
///
/// lunatui's goal is one `write()` per frame (PRD §1.3), and this is how
/// benchmarks check it.
///
/// ```
/// use std::io::Write;
/// use lunatui_bench::CountingWriter;
///
/// let mut out = CountingWriter::new(Vec::new());
/// out.write_all(b"frame")?;
/// assert_eq!(out.stats().writes, 1);
/// assert_eq!(out.stats().bytes, 5);
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct CountingWriter<W> {
    inner: W,
    stats: WriteStats,
}

impl<W> CountingWriter<W> {
    /// Wraps `inner` with all counts at zero.
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            stats: WriteStats::default(),
        }
    }

    /// Returns the counts so far.
    pub fn stats(&self) -> WriteStats {
        self.stats
    }

    /// Returns the wrapped writer.
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stats.writes += 1;
        let written = self.inner.write(buf)?;
        self.stats.bytes += written;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stats.flushes += 1;
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};

    use super::{CountingWriter, WriteStats};

    struct ShortWriter {
        limit: usize,
    }

    impl Write for ShortWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len().min(self.limit))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("write failed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn counts_each_write_and_flush() -> io::Result<()> {
        let mut out = CountingWriter::new(Vec::new());
        out.write_all(b"abc")?;
        out.write_all(b"de")?;
        out.flush()?;

        let expected = WriteStats {
            writes: 2,
            bytes: 5,
            flushes: 1,
        };
        assert_eq!(out.stats(), expected);
        assert_eq!(out.into_inner(), b"abcde");
        Ok(())
    }

    #[test]
    fn counts_every_call_of_a_short_writing_inner() -> io::Result<()> {
        let mut out = CountingWriter::new(ShortWriter { limit: 4 });
        out.write_all(&[0; 10])?;
        assert_eq!(out.stats().writes, 3);
        assert_eq!(out.stats().bytes, 10);
        Ok(())
    }

    #[test]
    fn counts_failed_writes_without_bytes() {
        let mut out = CountingWriter::new(FailingWriter);
        assert!(out.write(b"abc").is_err());
        assert_eq!(out.stats().writes, 1);
        assert_eq!(out.stats().bytes, 0);
    }
}
