#![no_std]

#[cfg(test)]
#[macro_use]
extern crate std;

#[cfg(test)]
use std::println;

#[derive(Debug)]
pub struct Event {
    pub duration: u32,      // milliseconds
    pub freq: Option<u32>,  // frequency in Hz
}

#[derive(Debug)]
pub struct AbcIter {
    content: &'static [u8],
    cursor: usize,
    _bpm: u32,       // beats per minute
    _l_num: u32,     // numerator of L: (default note length)
    _l_denom: u32,   // denominator of L:
    _m_num: u32,     // numerator of M: (time signature)
    _m_denom: u32,   // denominator of M:
}

#[derive(Debug)]
pub enum AbcError {
    MissingL,
    MissingM,
    InvalidFraction,
}

impl Iterator for AbcIter {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        // skip until note
        while !self.is_finished() && !self.get_byte().is_ascii_alphabetic() {
            self.cursor += 1
        }

        if self.is_finished() {
            return None;
        }

        let start = self.cursor;
        let mut len = 1;

        self.cursor += 1;

        if self.get_byte() == b'\'' || self.get_byte() == b',' {
            len += 1;
        }

        let duration = if !self.is_finished() && self.get_byte().is_ascii_digit() {
            (self.get_byte() as char).to_digit(10).unwrap()
        } else {
            1
        };

        if !self.is_finished() && self.get_byte() == b'-' {

        }

        let slice = &self.content[start..start+len];
        // #[cfg(test)]
        // println!("{:?}", String::from_utf8_lossy(slice));
        let freq = note_to_freq(slice);

        Some(Event {
            duration,
            freq,
        })
    }
}


fn note_to_freq(note: &[u8]) -> Option<u32> {
    match note {
        // Only C-major / A-minor for now
        b"C,"  => Some(130),
        b"D,"  => Some(146),
        b"E,"  => Some(164),
        b"F,"  => Some(174),
        b"G,"  => Some(196),
        b"A,"  => Some(220),
        b"B,"  => Some(246),
        b"C"   => Some(261),
        b"D"   => Some(293),
        b"E"   => Some(329),
        b"F"   => Some(349),
        b"G"   => Some(392),
        b"A"   => Some(440),
        b"B"   => Some(493),
        b"c"   => Some(523),
        b"d"   => Some(587),
        b"e"   => Some(659),
        b"f"   => Some(698),
        b"g"   => Some(783),
        b"a"   => Some(880),
        b"b"   => Some(987),
        b"c'"  => Some(1046),
        b"d'"  => Some(1174),
        b"e'"  => Some(1318),
        b"f'"  => Some(1396),
        b"g'"  => Some(1568),
        b"a'"  => Some(1760),
        b"b'"  => Some(1975),
        _ => None,
    }
}


impl AbcIter {
    pub fn get_byte(&self) -> u8 {
        self.content[self.cursor]
    }
    pub fn is_finished(&self) -> bool {
        self.cursor >= self.content.len()
    }
    pub fn new(content: &'static [u8], bpm: u32) -> Result<Self, AbcError> {
        let mut cursor = 0;

        let mut l_num: Option<u32> = None;
        let mut l_denom: Option<u32> = None;
        let mut m_num: Option<u32> = None;
        let mut m_denom: Option<u32> = None;

        loop {
            // skip leading whitespace/newlines
            while cursor < content.len() && content[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if content[cursor].is_ascii_uppercase() && content[cursor + 1] == b':' {
                let key = content[cursor];
                cursor += 2; // skip key and ':'
                let start = cursor;
                while content[cursor] != b'\n' {
                    cursor += 1;
                }
                let line = &content[start..cursor];
                match key {
                    b'M' => {
                        if let Some((n, d)) = parse_fraction(line) {
                            m_num = Some(n);
                            m_denom = Some(d);
                        } else {
                            return Err(AbcError::InvalidFraction);
                        }
                    }
                    b'L' => {
                        if let Some((n, d)) = parse_fraction(line) {
                            l_num = Some(n);
                            l_denom = Some(d);
                        } else {
                            return Err(AbcError::InvalidFraction);
                        }
                    }
                    _ => {}
                }
                cursor += 1; // move past newline
            } else {
                break;
            }
        }

        Ok(Self {
            content,
            cursor,
            _bpm: bpm,
            _l_num: l_num.ok_or(AbcError::MissingL)?,
            _l_denom: l_denom.ok_or(AbcError::MissingL)?,
            _m_num: m_num.ok_or(AbcError::MissingM)?,
            _m_denom: m_denom.ok_or(AbcError::MissingM)?,
        })
    }
}

fn parse_fraction(bytes: &[u8]) -> Option<(u32, u32)> {
    let s = core::str::from_utf8(bytes).ok()?;
    let mut parts = s.trim().split('/');
    let n = parts.next()?.parse().ok()?;
    let d = parts.next()?.parse().ok()?;
    Some((n, d))
}
