#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use lunatui_term::{Event, KeyCode, KeyEvent};

    use crate::Input;

    struct FakeSource {
        bytes: Vec<u8>,
        delivered: bool,
    }

    impl FakeSource {
        fn new(bytes: Vec<u8>) -> Self {
            Self {
                bytes,
                delivered: false,
            }
        }
    }

    impl Read for FakeSource {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.delivered {
                return Ok(0);
            }
            self.delivered = true;
            let length = self.bytes.len().min(buffer.len());
            buffer[..length].copy_from_slice(&self.bytes[..length]);
            Ok(length)
        }
    }

    #[test]
    fn bursts_coalesce_to_one_frame() -> Result<(), Box<dyn std::error::Error>> {
        let source = FakeSource::new(b"abc".to_vec());
        let mut input = Input::from_reader(source)?;

        let frame = input.next_frame()?.ok_or("source ended before its burst")?;

        assert_eq!(
            frame,
            vec![
                Event::Key(KeyEvent::press(KeyCode::Char('a'))),
                Event::Key(KeyEvent::press(KeyCode::Char('b'))),
                Event::Key(KeyEvent::press(KeyCode::Char('c'))),
            ]
        );
        assert_eq!(input.next_frame()?, None);
        Ok(())
    }
}
