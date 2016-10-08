use std::cmp::{max, Ordering};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Point{x: x, y: y}
    }

    pub fn distance(&self, other: Point) -> f32 {
        let a = (self.x - other.x).pow(2);
        let b = (self.y - other.y).pow(2);
        ((a + b) as f32).sqrt()
    }

    pub fn tile_distance(self, other: Point) -> i32 {
        max((self.x - other.x).abs(), (self.y - other.y).abs())
    }
}

impl std::fmt::Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl std::ops::Add for Point {
    type Output = V2;

    fn add(self, rhs: V2) -> Point {
        Point{ x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl std::cmp::PartialOrd for Point {
    fn partial_cmp(&self, other: &Point) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if (self.x < other.x) && (self.y < other.y) {
            Some(Ordering::Less)
        } else if (self.x > other.x) && (self.y > other.y) {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
}

impl std::ops::Add<(i32, i32)> for Point {
    type Output = V2;

    fn add(self, rhs: (i32, i32)) -> Point {
        Point{ x: self.x + rhs.0, y: self.y + rhs.1 }
    }
}

impl PartialEq<(i32, i32)> for Point {
    fn eq(&self, other: &(i32, i32)) -> bool {
        self == Point::new(other.0, other.1)
    }
}

impl PartialOrd<(i32, i32)> for Point {
    fn partial_cmp(&self, other: &(i32, i32)) -> Option<Ordering> {
        self.partial_cmp(&Point::new(other.0, other.1))
    }
}

pub fn point(x: i32, y: i32) -> Point {
    Point::new(x, y)
}


pub struct CircularArea {
    x: i32,
    y: i32,
    center: Point,
    radius: i32,
    initial_x: i32,
    max_x: i32,
    max_y: i32,
}

impl CircularArea {
    pub fn new(center: Point, radius: i32) -> Self {
        let (center_x, center_y) = center;
        CircularArea {
            x: center_x - radius,
            y: center_y - radius,
            center: center,
            radius: radius,
            initial_x: center_x - radius,
            max_x: center_x + radius,
            max_y: center_y + radius,
        }
    }
}

impl Iterator for CircularArea {
    type Item = Point;

    fn next(&mut self) -> Option<Point> {
        loop {
            if (self.y > self.max_y) || (self.x > self.max_x) {
                return None;
            }
            let current_point = (self.x, self.y);
            self.x += 1;
            if self.x > self.max_x {
                self.x = self.initial_x;
                self.y += 1;
            }
            if distance(current_point, self.center) < self.radius as f32 {
                return Some(current_point)
            } else {
                // Keep looping for another point
            }
        }
    }
}

pub struct SquareArea {
    x: i32,
    y: i32,
    min_x: i32,
    max_x: i32,
    max_y: i32,
}

impl SquareArea {
    pub fn new(center: Point, half_side: i32) -> Self {
        let (center_x, center_y) = center;
        SquareArea {
            x: center_x - half_side,
            y: center_y - half_side,
            min_x: center_x - half_side,
            max_x: center_x + half_side,
            max_y: center_y + half_side,
        }
    }
}

impl Iterator for SquareArea {
    type Item = Point;

    fn next(&mut self) -> Option<Point> {
        if self.y > self.max_y {
            return None
        }
        let current_point = (self.x, self.y);
        self.x += 1;
        if self.x > self.max_x {
            self.y += 1;
            self.x = self.min_x;
        }
        return Some(current_point)
    }
}

#[cfg(test)]
mod test {
    use std::iter::FromIterator;
    use std::f32::EPSILON;
    use super::{point, Point, SquareArea};

    #[test]
    fn test_tile_distance() {
        assert_eq!(tile_distance((0, 0), (0, 0)), 0);

        assert_eq!(tile_distance((0, 0), ( 1, 0)), 1);
        assert_eq!(tile_distance((0, 0), (-1, 0)), 1);
        assert_eq!(tile_distance((0, 0), ( 1, 1)), 1);
        assert_eq!(tile_distance((0, 0), (-1, 1)), 1);
        assert_eq!(tile_distance((0, 0), (0,  1)), 1);
        assert_eq!(tile_distance((0, 0), (0, -1)), 1);
        assert_eq!(tile_distance((0, 0), (1,  1)), 1);
        assert_eq!(tile_distance((0, 0), (1, -1)), 1);

        assert_eq!(tile_distance((0, 0), (2, 2)), 2);
        assert_eq!(tile_distance((0, 0), (-2, -2)), 2);
        assert_eq!(tile_distance((0, 0), (0, 2)), 2);
        assert_eq!(tile_distance((0, 0), (2, 0)), 2);

        assert_eq!(tile_distance((-3, -3), (10, 10)), 13);
        assert_eq!(tile_distance((-3, -3), (5, -2)), 8);
    }

    #[test]
    fn test_euclidean_distance() {
        let actual = distance((0, 0), (0, 0));
        let expected = 0.0;
        assert!((actual - expected).abs() <= EPSILON);

        let actual = distance((0, 0), (10, 10));
        let expected = 14.142136;
        assert!((actual - expected).abs() <= EPSILON);

        let actual = distance((0, 0), (10, -10));
        let expected = 14.142136;
        assert!((actual - expected).abs() <= EPSILON);

        let actual = distance((0, 0), (-10, 10));
        let expected = 14.142136;
        assert!((actual - expected).abs() <= EPSILON);

        let actual = distance((0, 0), (10, -10));
        let expected = 14.142136;
        assert!((actual - expected).abs() <= EPSILON);

        let actual = distance((0, 0), (3, 4));
        let expected = 5.0;
        assert!((actual - expected).abs() <= EPSILON);

        let actual = distance((0, 0), (-3, 4));
        let expected = 5.0;
        assert!((actual - expected).abs() <= EPSILON);

        let actual = distance((0, 0), (3, -4));
        let expected = 5.0;
        assert!((actual - expected).abs() <= EPSILON);

        let actual = distance((0, 0), (-3, -4));
        let expected = 5.0;
        assert!((actual - expected).abs() <= EPSILON);
}

    #[test]
    fn test_points_within_radius_of_zero() {
        let actual: Vec<Point> = FromIterator::from_iter(SquareArea::new((3, 3), 0));
        assert_eq!(actual, [(3, 3)]);
    }

    #[test]
    fn test_points_within_radius_of_one() {
        let actual: Vec<Point> = FromIterator::from_iter(SquareArea::new((0, 0), 1));
        let expected = [(-1, -1), (0, -1), (1, -1),
                        (-1,  0), (0,  0), (1,  0),
                        (-1,  1), (0,  1), (1,  1)];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_points_within_radius_of_five() {
        let mut actual: Vec<Point> = FromIterator::from_iter(SquareArea::new((0, 0), 5));

        let mut expected = Vec::new();
        for x in -5..6 {
            for y in -5..6 {
                expected.push((x, y));
            }
        }
        // the order is undefined so make sure we don't fail just because of ordering
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected);
    }
}
