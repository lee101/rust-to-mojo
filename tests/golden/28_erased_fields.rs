//! One untranslatable field no longer stubs the whole type.
use parking_lot::Mutex;
use std::rc::Rc;

pub trait Platform {
    fn name(&self) -> String;
}

pub struct App {
    pub platform: Rc<dyn Platform>,
    pub on_quit: Option<Box<dyn FnOnce()>>,
    pub frame_count: usize,
    pub title: String,
}

impl App {
    /// Does not touch the erased fields: translates.
    pub fn next_frame(&mut self) -> usize {
        self.frame_count += 1;
        self.frame_count
    }

    pub fn title_len(&self) -> usize {
        self.title.len()
    }

    /// Reads an erased field: stubbed.
    pub fn platform_name(&self) -> String {
        self.platform.name()
    }

    /// Constructs the erased field: stubbed.
    pub fn new(platform: Rc<dyn Platform>) -> App {
        App { platform, on_quit: None, frame_count: 0, title: String::new() }
    }
}

pub struct Counter {
    pub hits: Mutex<u32>,
}

impl Counter {
    pub fn bump(&self) -> u32 {
        let mut hits = self.hits.lock();
        *hits += 1;
        *hits
    }

    pub fn read(&self) -> u32 {
        *self.hits.lock()
    }
}

pub struct Span<'a> {
    pub text: &'a String,
    pub start: usize,
}

impl<'a> Span<'a> {
    pub fn new(text: &'a String, start: usize) -> Self {
        Span { text, start }
    }

    pub fn len(&self) -> usize {
        self.text.len() - self.start
    }
}

pub struct Meta {
    pub names: &'static [&'static str],
    pub id: u32,
}

impl Meta {
    pub fn get_id(&self) -> u32 {
        self.id
    }
}
