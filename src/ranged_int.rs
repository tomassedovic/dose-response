use std::ops::{Add, AddAssign, Deref, Sub, SubAssign};


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RangedInt {
    val: i32,
    _min: i32,
    _max: i32,
}

impl RangedInt {
    pub fn new(value: i32, min: i32, max: i32) -> RangedInt {
        assert!(min <= max);
        assert!(value >= min);
        assert!(value <= max);
        RangedInt {
            val: value,
            _min: min,
            _max: max,
        }
    }

    pub fn set(&mut self, n: i32) {
        assert!(n >= self._min);
        assert!(n <= self._max);
        self.val = n;
    }
}

impl Add<i32> for RangedInt {
    type Output = RangedInt;

    fn add(self, other: i32) -> Self::Output {
        match self.val.checked_add(other) {
            Some(v) => {
                let new_val = if v > self._max {
                    self._max
                } else if v < self._min {
                    self._min
                } else {
                    v
                };
                RangedInt::new(new_val, self._min, self._max)
            }
            None => {
                if other > 0 {
                    RangedInt::new(self._max, self._min, self._max)
                } else {
                    RangedInt::new(self._min, self._min, self._max)
                }
            }
        }
    }
}

impl AddAssign<i32> for RangedInt {
    fn add_assign(&mut self, other: i32) {
        *self = self.clone() + other
    }
}

impl Sub<i32> for RangedInt {
    type Output = RangedInt;

    fn sub(self, other: i32) -> Self::Output {
        let (negated_val, overflowed) = other.overflowing_neg();
        if overflowed {
            RangedInt::new(self._max, self._min, self._max)
        } else {
            self + negated_val
        }
    }
}

impl SubAssign<i32> for RangedInt {
    fn sub_assign(&mut self, other: i32) {
        *self = self.clone() - other
    }
}

impl Deref for RangedInt {
    type Target = i32;

    fn deref(&self) -> &i32 {
        &self.val
    }
}

#[cfg(test)]
mod test {
    use super::RangedInt;
    use std::i32::{MAX, MIN};

    #[test]
    fn new() {
        assert_eq!(*RangedInt::new(1, 1, 1), 1);
        assert_eq!(*RangedInt::new(3, -3, 3), 3);
        assert_eq!(*RangedInt::new(-3, -3, 3), -3);
    }

    #[test]
    #[should_panic]
    fn below_minimum_negative() {
        RangedInt::new(-1, 0, 1);
    }

    #[test]
    #[should_panic]
    fn below_minimum_positive() {
        RangedInt::new(5, 10, 20);
    }

    #[test]
    #[should_panic]
    fn above_maximum_positive() {
        RangedInt::new(10, 1, 2);
    }

    #[test]
    fn adding_positive() {
        assert_eq!(RangedInt::new(1, -5, 5) + 3, RangedInt::new(4, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) + 4, RangedInt::new(5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) + 5, RangedInt::new(5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) + 6, RangedInt::new(5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) + 2938, RangedInt::new(5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) + MAX, RangedInt::new(5, -5, 5));
    }

    #[test]
    fn adding_negative() {
        assert_eq!(RangedInt::new(1, -5, 5) + (-1), RangedInt::new(0, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) + (-5), RangedInt::new(-4, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) + (-6), RangedInt::new(-5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) + (-7), RangedInt::new(-5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) + (-9328), RangedInt::new(-5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) + MIN, RangedInt::new(-5, -5, 5));
    }

    #[test]
    fn subtracting_positive() {
        assert_eq!(RangedInt::new(1, -5, 5) - 1, RangedInt::new(0, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) - 5, RangedInt::new(-4, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) - 6, RangedInt::new(-5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) - 7, RangedInt::new(-5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) - 9328, RangedInt::new(-5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) - MAX, RangedInt::new(-5, -5, 5));
    }

    #[test]
    fn subtracting_negative() {
        assert_eq!(RangedInt::new(1, -5, 5) - (-3), RangedInt::new(4, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) - (-4), RangedInt::new(5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) - (-5), RangedInt::new(5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) - (-6), RangedInt::new(5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) - (-2938), RangedInt::new(5, -5, 5));
        assert_eq!(RangedInt::new(1, -5, 5) - MIN, RangedInt::new(5, -5, 5));
    }

    #[test]
    fn add_assign_positive() {
        let mut a = RangedInt::new(1, -5, 5);
        a += 3;
        assert_eq!(a, RangedInt::new(4, -5, 5));
        a += 1;
        assert_eq!(a, RangedInt::new(5, -5, 5));
        a += 1;
        assert_eq!(a, RangedInt::new(5, -5, 5));
        a += 23898923;
        assert_eq!(a, RangedInt::new(5, -5, 5));
        a += MAX;
        assert_eq!(a, RangedInt::new(5, -5, 5));
    }

    #[test]
    fn add_assign_negative() {
        let mut b = RangedInt::new(1, -5, 5);
        b += -5;
        assert_eq!(b, RangedInt::new(-4, -5, 5));
        b += -1;
        assert_eq!(b, RangedInt::new(-5, -5, 5));
        b += -1;
        assert_eq!(b, RangedInt::new(-5, -5, 5));
        b += -23898923;
        assert_eq!(b, RangedInt::new(-5, -5, 5));
        b += MIN;
        assert_eq!(b, RangedInt::new(-5, -5, 5));
    }

    #[test]
    fn sub_assign_positive() {
        let mut a = RangedInt::new(1, -5, 5);
        a -= 5;
        assert_eq!(a, RangedInt::new(-4, -5, 5));
        a -= 1;
        assert_eq!(a, RangedInt::new(-5, -5, 5));
        a -= 1;
        assert_eq!(a, RangedInt::new(-5, -5, 5));
        a -= 389832;
        assert_eq!(a, RangedInt::new(-5, -5, 5));
        a -= MAX;
        assert_eq!(a, RangedInt::new(-5, -5, 5));
    }

    #[test]
    fn sub_assign_negative() {
        let mut b = RangedInt::new(1, -5, 5);
        b -= -3;
        assert_eq!(b, RangedInt::new(4, -5, 5));
        b -= -1;
        assert_eq!(b, RangedInt::new(5, -5, 5));
        b -= -1;
        assert_eq!(b, RangedInt::new(5, -5, 5));
        b -= -389832;
        assert_eq!(b, RangedInt::new(5, -5, 5));
        b -= MIN;
        assert_eq!(b, RangedInt::new(5, -5, 5));
    }
}
