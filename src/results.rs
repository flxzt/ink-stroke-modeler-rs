/// result struct
/// contains the position, time, presusre as well as the velocity and acceleration data
#[derive(Debug, PartialEq)]
pub struct ModelerResult {
    pub pos: (f32, f32),
    pub velocity: (f32, f32),
    pub acceleration: (f32, f32),
    pub time: f64,
    pub pressure: f32,
}

/// A [ModelerResult] that does not have yet a pressure information
#[derive(Clone, Debug)]
pub(crate) struct ModelerPartial {
    pub pos: (f32, f32),
    pub velocity: (f32, f32),
    pub acceleration: (f32, f32),
    pub time: f64,
}

impl ModelerResult {
    #[cfg(test)]
    pub fn near(self, other: ModelerResult) -> bool {
        let tol = 1e-4;
        approx::abs_diff_eq!(self.pos.0, other.pos.0, epsilon = tol)
            && approx::abs_diff_eq!(self.pos.1, other.pos.1, epsilon = tol)
            && approx::abs_diff_eq!(self.time, other.time, epsilon = tol as f64)
            && approx::abs_diff_eq!(self.acceleration.0, other.acceleration.0, epsilon = tol)
            && approx::abs_diff_eq!(self.acceleration.1, other.acceleration.1, epsilon = tol)
            && approx::abs_diff_eq!(self.velocity.0, other.velocity.0, epsilon = tol)
            && approx::abs_diff_eq!(self.velocity.1, other.velocity.1, epsilon = tol)
            && approx::abs_diff_eq!(self.pressure, other.pressure, epsilon = tol)
    }
}

impl Default for ModelerResult {
    fn default() -> Self {
        Self {
            pos: (0.0, 0.0),
            velocity: (0.0, 0.0),
            acceleration: (0.0, 0.0),
            pressure: 1.0,
            time: 0.0,
        }
    }
}

/// Utility to compare [ModelerResult] up to `tol =1e-4` (not settable for now)
/// with printed output for debug purposes
///
/// Only used for testing purposes
#[allow(unused)]
#[cfg(test)]
pub(crate) fn compare_results(left: Vec<ModelerResult>, right: Vec<ModelerResult>) -> bool {
    if left.len() != right.len() {
        println!("\n\nleft : {:?} right {:?}", left.len(), right.len());
        //iterate
        println!("left");
        for el in left {
            println!("{:?}", el);
        }
        println!("right");
        for el in right {
            println!("{:?}", el);
        }
        false
    } else {
        println!("\n\n\nleft : {:?} right {:?}", &left.len(), &right.len());
        //iterate
        println!("left");
        for el in &left {
            println!("{:?}", el);
        }
        println!("right");
        for el in &right {
            println!("{:?}", el);
        }

        left.into_iter().zip(right).all(|x| x.0.near(x.1))
    }
}
