/// An RGBA color
#[derive(PartialEq, Clone, Copy, Default)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// An HSLA color
#[derive(Default, Copy, Clone, Debug)]
pub struct Hsla {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

pub fn rgba(hex: u32) -> Rgba {
    let r = ((hex >> 24) & 0xFF) as f32 / 255.0;
    let g = ((hex >> 16) & 0xFF) as f32 / 255.0;
    let b = ((hex >> 8) & 0xFF) as f32 / 255.0;
    let a = (hex & 0xFF) as f32 / 255.0;
    Rgba { r, g, b, a }
}

impl Rgba {
    pub fn blend(&self, other: Rgba) -> Self {
        if other.a >= 1.0 {
            other
        } else if other.a <= 0.0 {
            *self
        } else {
            let new_r = (1.0 - other.a) * self.r + other.a * other.r;
            let new_g = (1.0 - other.a) * self.g + other.a * other.g;
            let new_b = (1.0 - other.a) * self.b + other.a * other.b;
            Rgba { r: new_r, g: new_g, b: new_b, a: self.a }
        }
    }
}

impl From<Hsla> for Rgba {
    fn from(color: Hsla) -> Self {
        let h = color.h;
        let s = color.s;
        let l = color.l;

        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;
        let cm = c + m;
        let xm = x + m;

        let (r, g, b) = match (h * 6.0).floor() as i32 {
            0 | 6 => (cm, xm, m),
            1 => (xm, cm, m),
            2 => (m, cm, xm),
            3 => (m, xm, cm),
            4 => (xm, m, cm),
            _ => (cm, m, xm),
        };

        Rgba {
            r: r.clamp(0., 1.),
            g: g.clamp(0., 1.),
            b: b.clamp(0., 1.),
            a: color.a,
        }
    }
}

impl Hsla {
    pub fn is_transparent(&self) -> bool {
        self.a == 0.0
    }

    pub fn opacity(&self, factor: f32) -> Self {
        Hsla {
            h: self.h,
            s: self.s,
            l: self.l,
            a: self.a * factor.clamp(0., 1.),
        }
    }

    pub fn grayscale(&self) -> Self {
        Hsla { s: 0., ..*self }
    }
}

pub const fn transparent_black() -> Hsla {
    Hsla { h: 0., s: 0., l: 0., a: 0. }
}
