pub trait NumHelper {
    fn limit_max(self, max: Self) -> Self;
    fn limit_min(self, min: Self) -> Self;
}

impl NumHelper for u32 {
    fn limit_max(self, max: Self) -> Self {
        if self > max {
            max
        } else {
            self
        }
    }

    fn limit_min(self, min: Self) -> Self {
        if self < min {
            min
        } else {
            self
        }
    }
}
