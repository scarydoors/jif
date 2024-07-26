pub struct BitReader<'a> {
    buf: &'a [u8],
    // index by bit instead of by byte
    position: usize,
    length: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            position: 0,
            length: buf.len() * 8,
        }
    }

    pub fn next(&mut self, count: u32) -> Option<u64> {
        let start_position = self.position;
        let end_position = self.position + count as usize;

        if end_position > self.length {
            return None;
        }
        // end_position not inclusive, i always forget..
        let mut value: u64 = 0;
        let mut out_shift = 0;
        for i in start_position..end_position {
            let byte_idx = (i / 8) as usize;
            let byte = self.buf[byte_idx];
            let shift = i % 8;
            let bit = (byte >> shift) as u64 & 1;
            value = value | (bit << out_shift);
            out_shift += 1;
        }
        self.position = end_position;
        Some(value)
    }

    pub fn len(&self) -> usize {
        self.length
    }
}

#[cfg(test)]
mod tests {
    use super::BitReader;

    #[test]
    fn it_works() {
        let buffer = &[
            0b10000100,
            0b10001111,
            0b10101001,
            0b11001011,
            0b11101101,
            0b00001111,
            0b10100011
        ];
        let mut reader = BitReader::new(buffer);
        assert_eq!(reader.next(3), 0b00000100);
        assert_eq!(reader.next(3), 0b00000000);
        assert_eq!(reader.next(3), 0b00000110);
        assert_eq!(reader.next(3), 0b00000111);
        assert_eq!(reader.next(3), 0b00000000);
        assert_eq!(reader.next(3), 0b00000011);
        assert_eq!(reader.next(3), 0b00000010);
        assert_eq!(reader.next(3), 0b00000101);
    }
}
