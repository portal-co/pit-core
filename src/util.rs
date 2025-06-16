pub use core;
use core::fmt::Write;

use sha3::digest::Update;
pub struct WriteUpdate<'a, 'b> {
    pub wrapped: &'a mut (dyn Update + 'b),
}
impl<'a,'b> Write for WriteUpdate<'a,'b>{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.wrapped.update(s.as_bytes());
        Ok(())
    }
}
