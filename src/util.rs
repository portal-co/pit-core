pub use core;
use core::fmt::Write;

use sha3::digest::Update;
/// Wrapper for types implementing `Update`, allowing use with `core::fmt::Write`.
pub struct WriteUpdate<'a, 'b> {
    pub wrapped: &'a mut (dyn Update + 'b),
}
/// Implements `core::fmt::Write` for `WriteUpdate`, forwarding writes to the underlying `Update`.
impl<'a,'b> Write for WriteUpdate<'a,'b>{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.wrapped.update(s.as_bytes());
        Ok(())
    }
}
