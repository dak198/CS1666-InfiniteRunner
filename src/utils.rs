// File for simple helper functions/macros that may be used in many places

#[macro_export]
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (Rect::new($x as i32, $y as i32, $w as u32, $h as u32))
);

#[macro_export]
macro_rules! p_rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (PhysRect::new($x as i32, $y as i32, $w as u32, $h as u32))
);

#[allow(dead_code)]
pub fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}
