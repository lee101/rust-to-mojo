#[derive(Clone, Copy, Default, Debug)]
pub struct Style {
    pub width: f32,
    pub height: f32,
    pub visible: bool,
}

pub fn wide() -> Style {
    Style {
        width: 100.0,
        ..Default::default()
    }
}

pub fn hidden(base: Style) -> Style {
    Style { visible: false, ..base }
}

pub fn from_fn() -> Style {
    Style { height: 2.0, ..make() }
}

fn make() -> Style {
    Style { width: 1.0, height: 1.0, visible: true }
}
