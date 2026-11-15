pub struct Handle {
    pub id: u32,
    pub tag: String,
}

impl Clone for Handle {
    fn clone(&self) -> Self {
        Handle { id: self.id + 1000, tag: self.tag.clone() }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        println!("dropping {}", self.id);
    }
}

#[derive(Copy)]
pub struct Tiny(u8);

impl Clone for Tiny {
    fn clone(&self) -> Self {
        *self
    }
}
