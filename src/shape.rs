use sdl2::rect::Point;
use std::f64::consts::PI;

/// The maximal integer value that can be used for rectangles.
///
/// This value is smaller than strictly needed, but is useful in ensuring that
/// rect sizes will never have to be truncated when clamping.
pub fn max_int_value() -> u32 {
    i32::max_value() as u32 / 2
}

/// The minimal integer value that can be used for rectangle positions
/// and points.
///
/// This value is needed, because otherwise the width of a rectangle created
/// from a point would be able to exceed the maximum width.
pub fn min_int_value() -> i32 {
    i32::min_value() / 2
}

fn clamp_size(val: u32) -> u32 {
    if val == 0 {
        1
    } else if val > max_int_value() {
        max_int_value()
    } else {
        val
    }
}

fn clamp_position(val: i32) -> i32 {
    if val > max_int_value() as i32 {
        max_int_value() as i32
    } else if val < min_int_value() {
        min_int_value()
    } else {
        val
    }
}

// converts angle to an equivalent value between 0 and 2π
fn clamp_angle(val: f64) -> f64 {
    val % (2.0 * PI)
}

pub struct PhysRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    theta: f64,
    coords: [Point; 4],
}

impl PhysRect {
    // rectangle with no rotation
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> PhysRect {
        let cx = clamp_position(x);
        let cy = clamp_position(y);
        let cw = clamp_size(width);
        let ch = clamp_size(height);
        PhysRect {
            x: clamp_position(x),
            y: clamp_position(y),
            w: clamp_size(width) as i32,
            h: clamp_size(height) as i32,
            theta: 0.0,
            coords: [
                Point::new(cx, cy),
                Point::new(cx + cw as i32, cy),
                Point::new(cx + cw as i32, cy + ch as i32),
                Point::new(cx, cy + ch as i32),
            ],
        }
    }

    // rectangle rotated by theta about its upper left corner
    pub fn new_rot(x: i32, y: i32, width: u32, height: u32, theta: f64) -> PhysRect {
        let cx = clamp_position(x);
        let cy = clamp_position(y);
        let cw = clamp_size(width) as i32;
        let ch = clamp_size(height) as i32;
        let ct = clamp_angle(theta);
        let dist = ((cw as f64).powi(2) + (ch as f64).powi(2)).sqrt();
        PhysRect {
            x: cx,
            y: cy,
            w: cw,
            h: ch,
            theta: ct,
            coords: [
                Point::new(cx, cy),
                Point::new(
                    cx + (cw as f64 * ct.cos()) as i32,
                    cy + (cw as f64 * ct.sin()) as i32,
                ),
                Point::new(cx + (dist * ct.cos()) as i32, cy + (dist * ct.sin()) as i32),
                Point::new(
                    cx + (ch as f64 * ct.cos()) as i32,
                    cy + (ch as f64 * ct.sin()) as i32,
                ),
            ],
        }
    }

    // does not work yet; center_on needs to implemented properly
    pub fn from_center<P>(center: P, width: u32, height: u32) -> PhysRect
    where
        P: Into<Point>,
    {
        let cw = clamp_size(width) as i32;
        let ch = clamp_size(height) as i32;
        let mut rect = PhysRect {
            x: 0,
            y: 0,
            w: cw,
            h: ch,
            theta: 0.0,
            coords: [
                Point::new(0, 0),
                Point::new(cw, 0),
                Point::new(cw, ch),
                Point::new(0, ch),
            ],
        };
        rect.center_on(center.into());
        rect
    }

    /// The horizontal position of the original top left corner of the
    /// rectangle.
    pub fn x(&self) -> i32 {
        self.x
    }

    /// The vertical position of the original top left corner of this rectangle.
    pub fn y(&self) -> i32 {
        self.y
    }

    /// The four corners of the rectangle in clockwise order starting with the
    /// original top left
    pub fn coords(&self) -> [Point; 4] {
        self.coords
    }

    /// The width of this rectangle
    pub fn width(&self) -> u32 {
        self.w as u32
    }

    /// The height of this rectangle
    pub fn height(&self) -> u32 {
        self.h as u32
    }

    /// The rotation angle of this rectangle
    pub fn angle(&self) -> f64 {
        self.theta
    }

    /// Sets the horizontal position of this rectangle to the given value,
    /// clamped to be less than or equal to i32::max_value() / 2.
    /// Position is based on the original upper right corner of the rectangle.
    pub fn set_x(&mut self, x: i32) {
        self.x = clamp_position(x);
    }

    /// Sets the vertical position of this rectangle to the given value,
    /// clamped to be less than or equal to i32::max_value() / 2.
    /// Position is based on the original upper right corner of the rectangle.
    pub fn set_y(&mut self, y: i32) {
        self.y = clamp_position(y);
    }

    /// Sets the height of this rectangle to the given value,
    /// clamped to be less than or equal to i32::max_value() / 2.
    pub fn set_height(&mut self, height: u32) {
        self.h = clamp_size(height) as i32;
    }

    /// The rectangle's current leftmost point
    pub fn left(&self) -> Point {
        let mut left = self.coords[0];
        for p in self.coords {
            if p.x() <= left.x() {
                left = p;
            }
        }
        left
    }

    /// The rectangle's current rightmost point
    pub fn right(&self) -> Point {
        let mut right = self.coords[0];
        for p in self.coords {
            if p.x() >= right.x() {
                right = p;
            }
        }
        right
    }

    /// The rectangle's current topmost point
    pub fn top(&self) -> Point {
        let mut top = self.coords[0];
        for p in self.coords {
            if p.y() <= top.y() {
                top = p;
            }
        }
        top
    }

    /// The rectangle's current bottom-most point
    pub fn bottom(&self) -> Point {
        let mut bottom = self.coords[0];
        for p in self.coords {
            if p.y() <= bottom.y() {
                bottom = p;
            }
        }
        bottom
    }

    /// The rectangle's center point
    pub fn center(&self) -> Point {
        let x = (self.coords[0].x() + self.coords[2].x()) / 2;
        let y = (self.coords[0].y() + self.coords[2].y()) / 2;
        Point::new(x, y)
    }

    // Centers the rectangle on point P
    pub fn center_on<P>(&mut self, point: P)
    where
        P: Into<(i32, i32)>,
    {
        let (x, y) = point.into();
        let d_x = clamp_position(x) - self.center().x();
        let d_y = clamp_position(y) - self.center().y();
        for p in self.coords {
            p.offset(d_x, d_y);
        }
        self.x = self.coords[0].x();
        self.y = self.coords[0].y();
    }

    /// Move this rect and clamp the positions to prevent over/underflow.
    /// This also clamps the size to prevent overflow.
    pub fn offset(&mut self, x: i32, y: i32) {
        let old_x = self.x;
        let old_y = self.y;
        match self.x.checked_add(x) {
            Some(val) => self.x = clamp_position(val),
            None => {
                if x >= 0 {
                    self.x = max_int_value() as i32;
                } else {
                    self.x = i32::min_value();
                }
            }
        }
        match self.y.checked_add(y) {
            Some(val) => self.y = clamp_position(val),
            None => {
                if y >= 0 {
                    self.y = max_int_value() as i32;
                } else {
                    self.y = i32::min_value();
                }
            }
        }
        let d_x = old_x - self.x;
        let d_y = old_x - self.y;
        for i in 0..self.coords.len() {
            self.coords[i] = self.coords[i].offset(d_x, d_y);
        }
    }

    /// Moves this rect to the given position after clamping the values.
    pub fn reposition<P>(&mut self, point: P)
    where
        P: Into<(i32, i32)>,
    {
        let (x, y) = point.into();
        let old_x = self.x();
        let old_y = self.y();
        self.x = clamp_position(x);
        self.y = clamp_position(y);
        let d_x = old_x - self.x();
        let d_y = old_x - self.y();
        for i in 0..self.coords.len() {
            self.coords[i] = self.coords[i].offset(d_x, d_y);
        }
    }

    /// Resizes this rect to the given size after clamping the values
    pub fn resize(&mut self, width: u32, height: u32) {
        let d_w = (width - self.width()) as f64;
        let d_h = (height - self.height()) as f64;
        let dist = (d_w.powi(2) + d_h.powi(2)).sqrt();
        self.coords[1] = self.coords[1].offset(
            (d_w * self.angle().cos()) as i32,
            (d_w * self.angle().sin()) as i32,
        );
        self.coords[2] = self.coords[2].offset(
            (dist * self.angle().cos()) as i32,
            (dist * self.angle().sin()) as i32,
        );
        self.coords[3] = self.coords[3].offset(
            (d_h * self.angle().cos()) as i32,
            (d_w * self.angle().sin()) as i32,
        );
        self.w = clamp_size(width) as i32;
        self.h = clamp_size(height) as i32;
    }

    /// Checks whether this rect contains a given point
    pub fn contains<P>(&self, point: P) -> bool
    where
        P: Into<(i32, i32)>,
    {
        let (x, y) = point.into();
        let mut c = false;
        let mut j = 3;
        for i in 0..self.coords.len() {
            if (((self.coords[i].y() > y) != (self.coords[j].y() > y))
                && (x
                    < (self.coords[j].x() - self.coords[i].x()) * (y - self.coords[i].y())
                        / (self.coords[j].y() - self.coords[i].y())
                        + self.coords[i].x()))
            {
                c = !c;
            }
            j = i;
        }
        c
    }

    /// Checks whether this rect intersects a given rect
    pub fn has_intersection(&self, other: PhysRect) -> bool {
        for i in 0..other.coords.len() {
            if self.contains(other.coords[i]) {
                return true;
            }
        }
        for i in 0..self.coords.len() {
            if other.contains(self.coords[i]) {
                return true;
            }
        }
        false
    }
}
