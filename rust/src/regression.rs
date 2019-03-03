use std::f64;

/// A simple linear regression calculator.  Based on Java commons-math3 3.6.1 SimpleRegression.
pub struct SimpleRegression {
    /// Sum of x values
    sum_x: f64,
    /// Total variation in x (sum of squared deviations from x_bar)
    sum_xx: f64,
    /// Sum of y values
    sum_y: f64,
    /// Total variation in y (sum of squared deviations from y_bar)
    sum_yy: f64,
    /// Sum of products
    sum_xy: f64,
    /// Number of observations
    n: i64,
    /// Mean of accumulated x values, used in updating formulas
    x_bar: f64,
    /// Mean of accumulated y values, used in updating formulas
    y_bar: f64,
    /// Include an intercept or not.  When false, the model is estimated without a constant term and
    /// getIntercept returns 0
    has_intercept: bool,
}

impl SimpleRegression {
    pub fn new() -> SimpleRegression {
        SimpleRegression {
            sum_x: 0f64,
            sum_xx: 0f64,
            sum_y: 0f64,
            sum_yy: 0f64,
            sum_xy: 0f64,
            n: 0,
            x_bar: 0f64,
            y_bar: 0f64,
            has_intercept: true,
        }
    }

    /// Adds the observation (x, y) to the regression data set.
    ///
    /// Uses updating formulas for means and sums of squares defined in "Algorithms for Computing
    /// the Sample Variance: Analysis and Recommendations", Chan, T.F., Golub, G.H., and
    /// LeVeque, R.J. 1983, American Statistician, vol. 37, pp. 242-247, referenced in Weisberg, S.
    /// "Applied Linear Regression". 2nd Ed. 1985.
    pub fn add_data(&mut self, x: f64, y: f64) -> () {
        if self.n == 0 {
            self.x_bar = x;
            self.y_bar = y;
        } else {
            if self.has_intercept {
                let fact1 = 1f64 + self.n as f64;
                let fact2 = self.n as f64 / (1f64 + self.n as f64);
                let dx = x - self.x_bar;
                let dy = y - self.y_bar;
                self.sum_xx += dx * dx * fact2;
                self.sum_yy += dy * dy * fact2;
                self.sum_xy += dx * dy * fact2;
                self.x_bar += dx / fact1;
                self.y_bar += dy / fact1;
            }
        }
        if !self.has_intercept {
            self.sum_xx += x * x;
            self.sum_yy += y * y;
            self.sum_xy += x * y;
        }
        self.sum_x += x;
        self.sum_y += y;
        self.n += 1;
    }

    /// Returns the "predicted" y value associated with the supplied x value, based on the data that
    /// has been added to the model when this method is activated.
    ///
    /// predict(x) = intercept + slope * x
    ///
    /// *Preconditions*: At least two observations (with at least two different x values) must have
    /// been added before invoking this method.  If this method is invoked before a model can be
    /// estimated, NaN is returned.
    pub fn predict(&self, x: f64) -> f64 {
        let b1 = self.get_slope();
        if self.has_intercept {
            self.get_intercept(b1) + b1 * x
        } else {
            b1 * x
        }
    }

    /// Returns the slipe of the estimated regression line.
    ///
    /// The least squared estimate of the slipe is computed using the [normal equations]
    /// (http://www.xycoon.com/estimation4.htm).  The slipe is sometimes denoted b1.
    ///
    /// *Preconditions*: At least two observations (with at least two different x values) must have
    /// been added before invoking this method.  If this method is invoked before a model can be
    /// estimated, NaN is returned.
    fn get_slope(&self) -> f64 {
        if self.n < 2 {
            f64::NAN // not enough data
        } else if self.sum_xx.abs() < 10f64 * f64::MIN {
            f64::NAN // not enough variation in x
        } else {
            self.sum_xy / self.sum_xx
        }
    }

    /// Returns the intercept of the estimated regression line, given the slope.
    ///
    /// Will return NaN if slope is Nan.
    fn get_intercept(&self, slope: f64) -> f64 {
        if self.has_intercept {
            (self.sum_y - slope * self.sum_x) / self.n as f64
        } else {
            0 as f64
        }
    }
}
