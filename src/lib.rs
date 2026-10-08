use serde::Deserialize;
use std::time::{Duration, Instant};

pub const GRID: usize = 16;
pub const STEP: Duration = Duration::from_millis(20);
pub const HOLD: Duration = Duration::from_millis(300);
pub type Frame = [bool; GRID * GRID];

#[derive(Deserialize, Debug)]
pub struct Reply {
    pub id: String,
    pub size: usize,
    pub text: String,
    pub sequence: String,
}

pub fn parse_sequence(source: &str) -> Result<Vec<Frame>, String> {
    let words: Vec<_> = source.split(|c: char| c.is_whitespace() || ",，;；|".contains(c))
        .filter(|w| !w.is_empty()).collect();
    if words.is_empty() || words.len() % GRID != 0 {
        return Err("Each frame requires 16 four-digit hexadecimal words".into());
    }
    let rows: Result<Vec<u16>, _> = words.iter().map(|word| {
        let word = word.strip_prefix("0x").or_else(|| word.strip_prefix("0X")).unwrap_or(word);
        if word.len() != 4 || !word.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Invalid four-digit hexadecimal word".to_string());
        }
        u16::from_str_radix(word, 16).map_err(|e| e.to_string())
    }).collect();
    Ok(rows?.chunks(GRID).map(|rows| {
        let mut frame = [false; GRID * GRID];
        for (row, word) in rows.iter().enumerate() {
            for column in 0..GRID { frame[row * GRID + column] = word & (1 << (15 - column)) != 0; }
        }
        frame
    }).collect())
}

#[derive(Clone, Debug)]
struct Pixel { current: usize, target: usize }

#[derive(Default)]
pub struct Player {
    pub text: String,
    pub id: Option<String>,
    frames: Vec<Frame>,
    index: usize,
    pixels: Vec<Pixel>,
    moving: bool,
    deadline: Option<Instant>,
}

impl Player {
    pub fn positions(&self) -> Vec<usize> { self.pixels.iter().map(|p| p.current).collect() }

    pub fn set_reply(&mut self, reply: Reply, now: Instant) -> Result<bool, String> {
        if reply.size != GRID { return Err("Only 16×16 replies are supported".into()); }
        if self.id.as_ref() == Some(&reply.id) { return Ok(false); }
        let frames = parse_sequence(&reply.sequence)?;
        let first = self.id.is_none();
        self.frames = frames;
        self.index = 0;
        self.text = reply.text;
        self.id = Some(reply.id);
        if first {
            self.pixels = self.frames[0].iter().enumerate().filter(|(_, on)| **on)
                .map(|(i, _)| Pixel { current: i, target: i }).collect();
            self.moving = false;
            self.deadline = (self.frames.len() > 1).then_some(now + HOLD);
        } else { self.transition(now); }
        Ok(true)
    }

    fn transition(&mut self, now: Instant) {
        let frame = self.frames[self.index];
        let mut unused: Vec<usize> = frame.iter().enumerate().filter(|(_, on)| **on).map(|(i, _)| i).collect();
        let mut sources = self.positions();
        sources.sort_unstable(); sources.dedup();
        let mut pixels = Vec::new();
        // Keep pixels already at a destination in place before matching moving ones.
        sources.retain(|source| {
            if let Some(i) = unused.iter().position(|t| t == source) {
                unused.remove(i); pixels.push(Pixel { current: *source, target: *source }); false
            } else { true }
        });
        for source in sources {
            let closest = unused.iter().enumerate().min_by_key(|(_, target)| {
                let dx = (source % GRID) as isize - (**target % GRID) as isize;
                let dy = (source / GRID) as isize - (**target / GRID) as isize;
                dx * dx + dy * dy
            }).map(|(i, _)| i);
            if let Some(i) = closest { pixels.push(Pixel { current: source, target: unused.remove(i) }); }
        }
        pixels.extend(unused.into_iter().map(|target| Pixel { current: target, target }));
        self.moving = pixels.iter().any(|p| p.current != p.target);
        self.pixels = pixels;
        self.deadline = if self.moving { Some(now + STEP) }
            else { (self.frames.len() > 1).then_some(now + HOLD) };
    }

    pub fn tick(&mut self, now: Instant) -> bool {
        if self.deadline.is_none_or(|deadline| now < deadline) { return false; }
        if !self.moving {
            self.index = (self.index + 1) % self.frames.len();
            self.transition(now);
            return true;
        }
        for pixel in &mut self.pixels {
            let column = pixel.current % GRID;
            let target_column = pixel.target % GRID;
            if column != target_column {
                pixel.current = (pixel.current as isize + if column < target_column { 1 } else { -1 }) as usize;
            } else if pixel.current / GRID != pixel.target / GRID {
                pixel.current = (pixel.current as isize + if pixel.current < pixel.target { GRID as isize } else { -(GRID as isize) }) as usize;
            }
        }
        self.moving = self.pixels.iter().any(|p| p.current != p.target);
        self.deadline = if self.moving { Some(now + STEP) }
            else { (self.frames.len() > 1).then_some(now + HOLD) };
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reply(id: &str, frames: &[Frame]) -> Reply {
        let sequence = frames.iter().flat_map(|frame| frame.chunks(GRID).map(|row| {
            let word = row.iter().fold(0u16, |word, on| (word << 1) | u16::from(*on));
            format!("{word:04X}")
        })).collect::<Vec<_>>().join(" ");
        Reply { id: id.into(), size: GRID, text: "test".into(), sequence }
    }
    #[test]
    fn wire_format_and_invalid_input() {
        let frame = parse_sequence("8001 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0000 0001").unwrap()[0];
        assert!(frame[0] && frame[15] && frame[255]);
        assert_eq!(frame.iter().filter(|p| **p).count(), 3);
        assert!(parse_sequence("FFFF").is_err());
        assert!(parse_sequence(&"ZZZZ ".repeat(16)).is_err());
    }
    #[test]
    fn one_cell_per_step_then_hold_and_interrupt() {
        let now = Instant::now();
        let mut first = [false; 256]; first[0] = true;
        let mut second = [false; 256]; second[18] = true;
        let mut player = Player::default();
        player.set_reply(reply("1", &[first, second]), now).unwrap();
        assert!(!player.tick(now + HOLD - Duration::from_millis(1)));
        player.tick(now + HOLD);
        for (step, expected) in [1, 2, 18].iter().enumerate() {
            player.tick(now + HOLD + STEP * (step as u32 + 1));
            assert_eq!(player.positions(), vec![*expected]);
        }
        let settled = now + HOLD + STEP * 3;
        assert!(!player.tick(settled + HOLD - Duration::from_millis(1)));
        player.tick(settled + HOLD);
        player.tick(settled + HOLD + STEP);
        assert_eq!(player.positions(), vec![17]);
        player.set_reply(reply("2", &[second]), settled + HOLD + STEP).unwrap();
        assert_eq!(player.positions(), vec![17]);
        player.tick(settled + HOLD + STEP * 2);
        assert_eq!(player.positions(), vec![18]);
        assert!(player.set_reply(Reply { id: "bad".into(), size: 16, text: "".into(), sequence: "bad".into() }, now).is_err());
        assert_eq!(player.positions(), vec![18]);
    }
}
