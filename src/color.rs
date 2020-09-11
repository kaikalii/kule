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
    fn red(r: f32) -> Self {
        Self::black().with_r(r)
    }
    fn green(g: f32) -> Self {
        Self::black().with_g(g)
    }
    fn blue(b: f32) -> Self {
        Self::black().with_b(b)
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
    fn map_rgb<F>(self, f: F) -> Self
    where
        F: Fn(f32) -> f32,
    {
        Self::rgba(f(self.r()), f(self.g()), f(self.b()), self.alpha())
    }
    fn map_all<F>(self, f: F) -> Self
    where
        F: Fn(f32) -> f32,
    {
        Self::rgba(f(self.r()), f(self.g()), f(self.b()), f(self.alpha()))
    }
    fn map_rgb_other<C, F>(self, other: C, f: F) -> Self
    where
        C: Color,
        F: Fn(f32, f32) -> f32,
    {
        Self::rgba(
            f(self.r(), other.r()),
            f(self.g(), other.g()),
            f(self.b(), other.b()),
            self.alpha(),
        )
    }
    fn map_all_other<C, F>(self, other: C, f: F) -> Self
    where
        C: Color,
        F: Fn(f32, f32) -> f32,
    {
        Self::rgba(
            f(self.r(), other.r()),
            f(self.g(), other.g()),
            f(self.b(), other.b()),
            f(self.alpha(), other.alpha()),
        )
    }
    fn lerp<C>(self, other: C, t: f32) -> Self
    where
        C: Color,
    {
        self.map_all_other(other, |a, b| a.lerp(b, t))
    }
    fn min<C>(self, other: C) -> Self
    where
        C: Color,
    {
        self.map_rgb_other(other, f32::min)
    }
    fn max<C>(self, other: C) -> Self
    where
        C: Color,
    {
        self.map_rgb_other(other, f32::max)
    }
    fn mul(self, val: f32) -> Self {
        self.map_rgb(|c| c * val)
    }
    fn mul_alpha(self, val: f32) -> Self {
        self.with_alpha(self.alpha() * val)
    }
    fn mul_color<C>(self, other: C) -> Self
    where
        C: Color,
    {
        self.map_all_other(other, std::ops::Mul::mul)
    }
    fn brighten_normalize(self) -> Self {
        if self.r() > self.g() && self.r() > self.b() {
            self.mul(1.0 / self.r())
        } else if self.g() > self.r() && self.g() > self.b() {
            self.mul(1.0 / self.g())
        } else {
            self.mul(1.0 / self.b())
        }
    }
    fn normalize(self) -> Self {
        if self.r() <= 1.0 && self.g() <= 1.0 && self.b() <= 1.0
            || self.r() == 0.0 && self.g() == 0.0 && self.b() == 0.0
        {
            self
        } else {
            self.brighten_normalize()
        }
    }
    fn dist(self, other: Self) -> f32 {
        ((self.r() - other.r()).powf(2.0)
            + (self.g() - other.g()).powf(2.0)
            + (self.b() - other.b()).powf(2.0))
        .powf(0.5)
    }
    fn as_gray(self) -> f32 {
        (self.r() + self.g() + self.b()) / 3.0
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
