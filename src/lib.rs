mod window;
pub use window::*;
mod error;
pub use error::*;

#[cfg(test)]
#[test]
fn test() {
    Window::builder().run(|window| window)
}
