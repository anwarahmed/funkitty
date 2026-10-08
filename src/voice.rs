//! Pink Kitty's voice: the clips in `voice/`, built into the program. They are WAV
//! files in IMA ADPCM, which is a quarter the size of plain samples and which this
//! file turns back into plain samples itself, so no audio library is linked.
//!
//! The clips are made by `tools/voice.py` from what `words::spoken` lists, and
//! `voice/lines.tsv` records the text each was made from.

include!(concat!(env!("OUT_DIR"), "/clips.rs"));

/// Samples a second in every clip, of one channel.
pub const RATE: u32 = 22_050;

fn find(name: &str) -> Option<&'static [u8]> {
    CLIPS.binary_search_by(|(clip, _)| (*clip).cmp(name)).ok().map(|i| CLIPS[i].1)
}

#[cfg(test)]
pub fn has(name: &str) -> bool {
    find(name).is_some()
}

/// The samples of a clip. Nothing, for a clip there is not.
pub fn clip(name: &str) -> Option<Vec<i16>> {
    decode(find(name)?)
}

const INDEX: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];
const STEP: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230,
    253, 279, 307, 337, 371, 408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272, 2499, 2749, 3024, 3327,
    3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794,
    32767,
];

fn u16_at(bytes: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?) as usize)
}

fn u32_at(bytes: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?) as usize)
}

/// A WAV file of one channel of IMA ADPCM, as samples. Nothing for any other file.
fn decode(wav: &[u8]) -> Option<Vec<i16>> {
    if wav.get(..4)? != b"RIFF" || wav.get(8..12)? != b"WAVE" {
        return None;
    }
    let (mut block, mut total, mut data) = (0, None, None);
    let mut at = 12;
    while let (Some(id), Some(size)) = (wav.get(at..at + 4), u32_at(wav, at + 4)) {
        let body = at + 8;
        match id {
            // Format 0x11 is IMA ADPCM. One channel at `RATE`, or it is not ours.
            b"fmt " => {
                if u16_at(wav, body)? != 0x11 || u16_at(wav, body + 2)? != 1 || u32_at(wav, body + 4)? != RATE as usize {
                    return None;
                }
                block = u16_at(wav, body + 12)?;
            }
            // How many samples there really are: the last block is padded.
            b"fact" => total = u32_at(wav, body),
            b"data" => data = wav.get(body..(body + size).min(wav.len())),
            _ => {}
        }
        at = body + size + size % 2;
    }
    let data = data?;
    if block <= 4 {
        return None;
    }
    let mut out = Vec::with_capacity(data.len() * 2);
    for chunk in data.chunks(block).filter(|chunk| chunk.len() >= 4) {
        // A block starts with a whole sample and where in the step table it stands.
        let mut value = i16::from_le_bytes([chunk[0], chunk[1]]) as i32;
        let mut index = (chunk[2] as i32).clamp(0, 88);
        out.push(value as i16);
        for &byte in &chunk[4..] {
            // Then four bits a sample, the low four first.
            for nibble in [byte & 15, byte >> 4] {
                let step = STEP[index as usize];
                let mut change = step >> 3;
                if nibble & 1 != 0 {
                    change += step >> 2;
                }
                if nibble & 2 != 0 {
                    change += step >> 1;
                }
                if nibble & 4 != 0 {
                    change += step;
                }
                value = (if nibble & 8 != 0 { value - change } else { value + change }).clamp(i16::MIN as i32, i16::MAX as i32);
                index = (index + INDEX[nibble as usize]).clamp(0, 88);
                out.push(value as i16);
            }
        }
    }
    if let Some(total) = total {
        out.truncate(total);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::words;

    /// One block: a first sample of 100 at step 7, then the four-bit codes 4, 4, 12, 0.
    #[test]
    fn a_block_is_decoded_by_the_book() {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF\0\0\0\0WAVEfmt ");
        wav.extend_from_slice(&20u32.to_le_bytes());
        wav.extend_from_slice(&0x11u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&RATE.to_le_bytes());
        wav.extend_from_slice(&(RATE / 2).to_le_bytes());
        wav.extend_from_slice(&6u16.to_le_bytes());
        wav.extend_from_slice(&4u16.to_le_bytes());
        wav.extend_from_slice(&[2, 0, 5, 0]);
        wav.extend_from_slice(b"fact");
        wav.extend_from_slice(&4u32.to_le_bytes());
        wav.extend_from_slice(&4u32.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&6u32.to_le_bytes());
        wav.extend_from_slice(&[100, 0, 0, 0, 0x44, 0x0C]);
        // Step 7: code 4 adds 0 + 7 = 7, and moves two steps on, to 9. Code 4 again
        // adds 1 + 9 = 10, on to step 11. Code 12 takes away 1 + 11 = 12. The fifth
        // sample is padding, which the count of four leaves out.
        assert_eq!(decode(&wav).unwrap(), [100, 107, 117, 105]);
        assert_eq!(decode(b"RIFF"), None);
        assert_eq!(decode(&wav[..40]), None);
        wav[20] = 1;
        assert_eq!(decode(&wav), None);
    }

    /// The game must never want a clip that is not there, nor say an old recording of
    /// a line that has since been changed.
    #[test]
    fn every_line_has_its_clip_and_the_clip_was_made_from_that_line() {
        let made: Vec<(&str, &str)> = include_str!("../voice/lines.tsv").lines().filter_map(|line| line.split_once('\t')).collect();
        let wanted = words::spoken();
        for (name, text) in &wanted {
            assert!(has(name), "no clip {name}: run tools/voice.py");
            assert!(made.contains(&(name.as_str(), text.as_str())), "the clip {name} was not made from {text:?}: run tools/voice.py");
        }
        assert_eq!(CLIPS.len(), wanted.len(), "there are clips nothing asks for: run tools/voice.py");
        assert!(CLIPS.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }

    #[test]
    fn every_clip_is_speech_of_a_sensible_length() {
        for (name, _) in CLIPS {
            let samples = clip(name).unwrap_or_else(|| panic!("{name} is not a clip"));
            let seconds = samples.len() as f32 / RATE as f32;
            assert!((0.15..9.0).contains(&seconds), "{name} lasts {seconds}");
            let loudest = samples.iter().map(|s| s.unsigned_abs()).max().unwrap();
            assert!(loudest > 4000, "{name} peaks at {loudest}");
        }
    }
}
