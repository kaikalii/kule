use vector2math::*;

/// The standard color type
pub type Col = [f32; 4];

/// Trait for manipulating colors
pub trait Color: Copy {
    /// Create a new color from rgba components
    fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self;
    /// Get the red component
    fn r(self) -> f32;
    /// Get the green component
    fn g(self) -> f32;
    /// Get the blue component
    fn b(self) -> f32;
    /// Get the alpha component
    fn alpha(self) -> f32;
    /// Create an opaque color from rgb components
    fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }
    /// Create an opaque gray color
    fn gray(val: f32) -> Self {
        Self::rgb(val, val, val)
    }
    /// Create an opaque white color
    fn white() -> Self {
        Self::gray(1.0)
    }
    /// Create an opaque black color
    fn black() -> Self {
        Self::gray(0.0)
    }
    /// Create an opaque red color
    fn red(r: f32) -> Self {
        Self::black().with_r(r)
    }
    /// Create an opaque green color
    fn green(g: f32) -> Self {
        Self::black().with_g(g)
    }
    /// Create an opaque blue color
    fn blue(b: f32) -> Self {
        Self::black().with_b(b)
    }
    /// Create an opaque yellow color
    fn yellow(y: f32) -> Self {
        Self::gray(y).with_b(0.0)
    }
    /// Create an opaque magenta color
    fn magenta(m: f32) -> Self {
        Self::gray(m).with_g(0.0)
    }
    /// Create an opaque cyan color
    fn cyan(c: f32) -> Self {
        Self::gray(c).with_r(0.0)
    }
    /// Get the color with a different red component
    fn with_r(self, r: f32) -> Self {
        Self::rgba(r, self.g(), self.b(), self.alpha())
    }
    /// Get the color with a different green component
    fn with_g(self, g: f32) -> Self {
        Self::rgba(self.r(), g, self.b(), self.alpha())
    }
    /// Get the color with a different blue component
    fn with_b(self, b: f32) -> Self {
        Self::rgba(self.r(), self.g(), b, self.alpha())
    }
    /// Get the color with a different alpha component
    fn with_alpha(self, alpha: f32) -> Self {
        Self::rgba(self.r(), self.g(), self.b(), alpha)
    }
    /// Map this color to another color type
    fn map<C>(self) -> C
    where
        C: Color,
    {
        C::rgba(self.r(), self.g(), self.b(), self.alpha())
    }
    /// Transform this color by applying a function to only the rgb components
    fn map_rgb<F>(self, f: F) -> Self
    where
        F: Fn(f32) -> f32,
    {
        Self::rgba(f(self.r()), f(self.g()), f(self.b()), self.alpha())
    }
    /// Transform this color by applying a function to all components
    fn map_all<F>(self, f: F) -> Self
    where
        F: Fn(f32) -> f32,
    {
        Self::rgba(f(self.r()), f(self.g()), f(self.b()), f(self.alpha()))
    }
    /// Create a color by applying a function to the pairs of r, g, and b components
    /// of this color and another.
    ///
    /// The new color has this color's alpha component
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
    /// Create a color by applying a function to the pairs of components
    /// of this color and another.
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
    /// Lineraly interpolate the components of this color and another
    fn lerp<C>(self, other: C, t: f32) -> Self
    where
        C: Color,
    {
        self.map_all_other(other, |a, b| a.lerp(b, t))
    }
    /// Create a color from the minima of the rgb components of this color and another
    ///
    /// The new color has this color's alpha component
    fn min<C>(self, other: C) -> Self
    where
        C: Color,
    {
        self.map_rgb_other(other, f32::min)
    }
    /// Create a color from the maxima of the rgb components of this color and another
    ///
    /// The new color has this color's alpha component
    fn max<C>(self, other: C) -> Self
    where
        C: Color,
    {
        self.map_rgb_other(other, f32::max)
    }
    /// Create a color by multiplying the color's rgb components by some value
    ///
    /// The new color has this color's alpha component
    fn mul(self, val: f32) -> Self {
        self.map_rgb(|c| c * val)
    }
    /// Multiply the color's alpha component by some value
    fn mul_alpha(self, val: f32) -> Self {
        self.with_alpha(self.alpha() * val)
    }
    /// Create a new color by multiplying this color's components by those
    /// of another
    fn mul_color<C>(self, other: C) -> Self
    where
        C: Color,
    {
        self.map_all_other(other, std::ops::Mul::mul)
    }
    /// Adjust the rgb components of the color such that the
    /// maximum component has a value of `1.0` while keeping
    /// the overall hue the same
    fn normalize(self) -> Self {
        if self.r() > self.g() && self.r() > self.b() {
            self.mul(1.0 / self.r())
        } else if self.g() > self.r() && self.g() > self.b() {
            self.mul(1.0 / self.g())
        } else {
            self.mul(1.0 / self.b())
        }
    }
    /// Adjust the rgb components of the color such that the
    /// maximum component has a value of `1.0` while keeping
    /// the overall hue the same
    ///
    /// This brighten the color, never darken it
    fn brighten_normalize(self) -> Self {
        if self.r() <= 1.0 && self.g() <= 1.0 && self.b() <= 1.0
            || self.r() == 0.0 && self.g() == 0.0 && self.b() == 0.0
        {
            self
        } else {
            self.brighten_normalize()
        }
    }
    /// Get the euclidean distance in rgb space between this color and another
    fn dist(self, other: Self) -> f32 {
        ((self.r() - other.r()).powf(2.0)
            + (self.g() - other.g()).powf(2.0)
            + (self.b() - other.b()).powf(2.0))
        .powf(0.5)
    }
    /// Get the value that represents this color in grayscale
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
