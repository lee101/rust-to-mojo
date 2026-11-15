# TODO(rust-to-mojo): fn `raw_read` not translated: unsafe fn
# --- original Rust ---
# pub unsafe fn raw_read(p: *const u8) -> u8 {
#     *p
# }
# --- end original Rust ---

# TODO(rust-to-mojo): fn `fetch` not translated: async fn
# --- original Rust ---
# pub async fn fetch() -> u32 {
#     42
# }
# --- end original Rust ---

# TODO(rust-to-mojo): static `COUNTER` not translated: `static mut` global
# --- original Rust ---
# static mut COUNTER: u32 = 0;
# --- end original Rust ---

# TODO(rust-to-mojo): extern `extern block` not translated: `extern` block (FFI)
# --- original Rust ---
# extern "C" {
#     fn abs(x: i32) -> i32;
# }
# --- end original Rust ---

# TODO(rust-to-mojo): fn `dynamic` not translated: `dyn Trait` (no existential types in Mojo) (dyn Fn(i32) -> i32)
# --- original Rust ---
# pub fn dynamic(items: Vec<Box<dyn Fn(i32) -> i32>>) -> i32 {
#     items.len() as i32
# }
# --- end original Rust ---

trait Describe:
    def describe(self) -> String:
        ...

# TODO(rust-to-mojo): impl `Describe for i32` not translated: impl on a std/primitive type (Mojo has no extensions)
# --- original Rust ---
# impl Describe for i32 {
#     fn describe(&self) -> String {
#         format!("int {}", self)
#     }
# }
# --- end original Rust ---

# TODO(rust-to-mojo): impl `Describe for Vec` not translated: impl on a std/primitive type (Mojo has no extensions)
# --- original Rust ---
# impl<T: Clone> Describe for Vec<T> {
#     fn describe(&self) -> String {
#         "vec".to_string()
#     }
# }
# --- end original Rust ---

# TODO(rust-to-mojo): struct `Holder` not translated: reference type in field/value position (needs Pointer + origin) (type `&'a[u8]`)
# --- original Rust ---
# pub struct Holder<'a> {
#     pub data: &'a [u8],
# }
# --- end original Rust ---

# TODO(rust-to-mojo): union `IntOrFloat` not translated: union
# --- original Rust ---
# pub union IntOrFloat {
#     i: u32,
#     f: f32,
# }
# --- end original Rust ---

# TODO(rust-to-mojo): fn `label_loop` not translated: labeled for loop ('outer : for i in 0..n { for j in 0..n { if i * j> 10 { break 'outer; } c += 1; } })
# signature hint: def label_loop(n: Int32) -> Int32
# --- original Rust ---
# pub fn label_loop(n: i32) -> i32 {
#     let mut c = 0;
#     'outer: for i in 0..n {
#         for j in 0..n {
#             if i * j > 10 {
#                 break 'outer;
#             }
#             c += 1;
#         }
#     }
#     c
# }
# --- end original Rust ---
