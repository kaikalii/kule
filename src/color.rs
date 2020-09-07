use vector2math::*;

pub type Col = [f32; 4];

pub trait Color: Copy {
    fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self;
    fn r(self) -> f32;
    fn g(self) -> f32;
    fn b(self) -> f32;
    fn alpha(self) -> f32;
    fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }
    fn gray(val: f32) -> Self {
        Self::rgb(val, val, val)
    }
    fn white() -> Self {
        Self::gray(1.0)
    }
    fn black() -> Self {
        Self::gray(0.0)
    }
    fn with_r(self, r: f32) -> Self {
        Self::rgba(r, self.g(), self.b(), self.alpha())
    }
    fn with_g(self, g: f32) -> Self {
        Self::rgba(self.r(), g, self.b(), self.alpha())
    }
    fn with_b(self, b: f32) -> Self {
        Self::rgba(self.r(), self.g(), b, self.alpha())
    }
    fn with_alpha(self, alpha: f32) -> Self {
        Self::rgba(self.r(), self.g(), self.b(), alpha)
    }
    fn map<C>(self) -> C
    where
        C: Color,
    {
        C::rgba(self.r(), self.g(), self.b(), self.alpha())
    }
}

impl Color for Col {
    fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        [r, g, b, a]
    }
    fn r(self) -> f32 {
        self[0]
    }
    fn g(self) -> f32 {
        self[1]
    }
    fn b(self) -> f32 {
        self[2]
    }
    fn alpha(self) -> f32 {
        self[3]
    }
}

impl Color for [f32; 3] {
    fn rgba(r: f32, g: f32, b: f32, _a: f32) -> Self {
        [r, g, b]
    }
    fn r(self) -> f32 {
        self[0]
    }
    fn g(self) -> f32 {
        self[1]
    }
    fn b(self) -> f32 {
        self[2]
    }
    fn alpha(self) -> f32 {
        1.0
    }
}

impl Color for (f32, f32, f32, f32) {
    fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        (r, g, b, a)
    }
    fn r(self) -> f32 {
        self.0
    }
    fn g(self) -> f32 {
        self.1
    }
    fn b(self) -> f32 {
        self.2
    }
    fn alpha(self) -> f32 {
        self.3
    }
}

impl Color for (f32, f32, f32) {
    fn rgba(r: f32, g: f32, b: f32, _a: f32) -> Self {
        (r, g, b)
    }
    fn r(self) -> f32 {
        self.0
    }
    fn g(self) -> f32 {
        self.1
    }
    fn b(self) -> f32 {
        self.2
    }
    fn alpha(self) -> f32 {
        1.0
    }
}
