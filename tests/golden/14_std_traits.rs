use std::fmt;

#[derive(Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
    }
}

impl fmt::Debug for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.r == self.g && self.g == self.b {
            write!(f, "Gray({})", self.r)
        } else {
            write!(f, "Rgb({}, {}, {})", self.r, self.g, self.b)
        }
    }
}

impl Default for Rgb {
    fn default() -> Self {
        Rgb { r: 0, g: 0, b: 0 }
    }
}

impl From<u32> for Rgb {
    fn from(hex: u32) -> Self {
        Rgb {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }
}

pub fn white() -> Rgb {
    Rgb::from(0xFFFFFF)
}

pub fn black() -> Rgb {
    let c: Rgb = 0x000000.into();
    c
}

pub fn fallback() -> Rgb {
    Default::default()
}
