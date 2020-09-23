use kule::*;

// An example app
struct App {
    pos: Vec2,
}

// The `Kule` trait defines app behavior
impl Kule for App {
    type Resources = ();
}

fn main() {
    App::run().unwrap();
}
