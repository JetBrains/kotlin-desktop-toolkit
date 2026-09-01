use desktop_common::logger::PanicDefault;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct PhysicalPixels(i32);

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct LogicalPixels(f64);

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct LogicalPixelsInt(i32);

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy)]
pub struct Scale(f64);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalSize {
    pub width: PhysicalPixels,
    pub height: PhysicalPixels,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct LogicalPoint {
    pub x: LogicalPixels,
    pub y: LogicalPixels,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct LogicalSize {
    pub width: LogicalPixelsInt,
    pub height: LogicalPixelsInt,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct LogicalRect {
    pub x: LogicalPixelsInt,
    pub y: LogicalPixelsInt,
    pub width: LogicalPixelsInt,
    pub height: LogicalPixelsInt,
}

impl PhysicalPixels {
    #[must_use]
    pub const fn raw_physical(&self) -> i32 {
        self.0
    }
}

impl LogicalPixels {
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn raw_logical(&self) -> f64 {
        self.0
    }

    #[must_use]
    pub fn to_raw_physical(self, scale: Scale) -> f64 {
        scale.to_raw_physical(self.0)
    }
}

impl LogicalPixelsInt {
    #[must_use]
    pub const fn new(value: i32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn raw_logical(&self) -> i32 {
        self.0
    }

    #[must_use]
    pub fn to_raw_physical(self, scale: Scale) -> f64 {
        scale.to_raw_physical(f64::from(self.0))
    }

    #[must_use]
    pub fn to_logical(self) -> LogicalPixels {
        LogicalPixels(f64::from(self.0))
    }
}

impl Scale {
    #[must_use]
    pub(crate) fn new(scale: i32) -> Self {
        Self(f64::from(scale))
    }

    #[must_use]
    pub(crate) fn to_raw_physical(self, value: f64) -> f64 {
        value * self.0
    }

    #[must_use]
    pub(crate) fn to_rounded_physical(self, value: LogicalPixelsInt) -> PhysicalPixels {
        let v = self.to_raw_physical(f64::from(value.raw_logical()));
        #[allow(clippy::cast_possible_truncation)]
        let rounded = v.round() as i32;
        PhysicalPixels(rounded)
    }
}

impl PanicDefault for LogicalPoint {
    fn default() -> Self {
        <Self as Default>::default()
    }
}

impl LogicalPoint {
    #[must_use]
    pub(crate) fn new<T: Into<f64>>(x: T, y: T) -> Self {
        Self {
            x: LogicalPixels(x.into()),
            y: LogicalPixels(y.into()),
        }
    }
}

impl LogicalSize {
    #[must_use]
    pub const fn wh(width: i32, height: i32) -> Self {
        Self {
            width: LogicalPixelsInt::new(width),
            height: LogicalPixelsInt::new(height),
        }
    }

    #[must_use]
    pub const fn validate(self) -> Option<Self> {
        if self.width.0 == 0 || self.height.0 == 0 {
            None
        } else {
            Some(self)
        }
    }

    #[must_use]
    pub(crate) fn to_rounded_physical(self, scale: Scale) -> PhysicalSize {
        PhysicalSize {
            width: scale.to_rounded_physical(self.width),
            height: scale.to_rounded_physical(self.height),
        }
    }
}

impl std::ops::Add<Self> for LogicalPixelsInt {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub<Self> for LogicalPixelsInt {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Add<Self> for LogicalPixels {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl From<LogicalPixelsInt> for LogicalPixels {
    fn from(value: LogicalPixelsInt) -> Self {
        Self(f64::from(value.0))
    }
}

impl std::cmp::PartialEq<LogicalPixels> for LogicalPixelsInt {
    fn eq(&self, other: &LogicalPixels) -> bool {
        f64::from(self.0).eq(&other.0)
    }
}

impl std::cmp::PartialOrd<LogicalPixels> for LogicalPixelsInt {
    fn partial_cmp(&self, other: &LogicalPixels) -> Option<std::cmp::Ordering> {
        f64::from(self.0).partial_cmp(&other.0)
    }
}

impl std::cmp::PartialEq<LogicalPixelsInt> for LogicalPixels {
    fn eq(&self, other: &LogicalPixelsInt) -> bool {
        self.0.eq(&f64::from(other.0))
    }
}

impl std::cmp::PartialOrd<LogicalPixelsInt> for LogicalPixels {
    fn partial_cmp(&self, other: &LogicalPixelsInt) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&f64::from(other.0))
    }
}
