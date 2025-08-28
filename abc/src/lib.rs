#![no_std]

#[cfg(test)]
#[macro_use]
extern crate std;

#[derive(Debug)]
pub struct Event {
    pub duration: u32,
    pub freq: Option<u32>,
}

#[derive(Debug)]
pub struct AbcIter{
    content: &'static [u8],
    cursor: usize,
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
        println!("{:?}", String::from_utf8_lossy(slice));
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
    pub fn new(content: &'static [u8]) -> Self {
        let mut cursor = 0;

        // skip header
        loop {
            if content[cursor].is_ascii_uppercase() && content[cursor + 1] == b':' {
                while content[cursor] != b'\n' {
                    cursor += 1;
                }

                cursor += 1;
            } else {
                break;
            }
        }

        Self { content, cursor }
    }

    pub fn get_byte(&self) -> u8 {
        self.content[self.cursor]
    }

    pub fn is_finished(&self) -> bool {
        self.cursor >= self.content.len()
    }
}


// #[cfg(test)]
// mod tests {
//     use super::*;

//     const EXAMPLE: str = 
// "X:1
// T:Speed the Plough
// M:4/4
// C:Trad.
// K:G
// |:GABc dedB|dedB dedB|c2ec B2dB|c2A2 A2BA|
//   GABc dedB|dedB dedB|c2ec B2dB|A2F2 G4:|
// |:g2gf gdBd|g2f2 e2d2|c2ec B2dB|c2A2 A2df|
//   g2gf g2Bd|g2f2 e2d2|c2ec B2dB|A2F2 G4:|";


// }
