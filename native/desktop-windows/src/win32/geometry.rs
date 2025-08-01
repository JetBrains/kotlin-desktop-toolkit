#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalPixels(pub i32);

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy)]
pub struct LogicalPixels(pub f32);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalPoint {
    pub x: PhysicalPixels,
    pub y: PhysicalPixels,
}

impl PhysicalPoint {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x: PhysicalPixels(x),
            y: PhysicalPixels(y),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalSize {
    pub width: PhysicalPixels,
    pub height: PhysicalPixels,
}

impl PhysicalSize {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width: PhysicalPixels(width),
            height: PhysicalPixels(height),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LogicalPoint {
    pub x: LogicalPixels,
    pub y: LogicalPixels,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct LogicalSize {
    pub width: LogicalPixels,
    pub height: LogicalPixels,
}
