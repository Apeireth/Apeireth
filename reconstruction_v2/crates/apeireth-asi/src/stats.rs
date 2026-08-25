//! R207 ASI 高级统计 utilities (v1 等价, 0 新依赖)
//! mean / variance / stddev / median / percentile / z_score / min_max_scale / Welford

pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    values.iter().sum::<f64>() / values.len() as f64
}
pub fn variance_pop(values: &[f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    let m = mean(values);
    values.iter().map(|x| (x - m).powi(2)).sum::<f64>() / values.len() as f64
}
pub fn variance_sample(values: &[f64]) -> f64 {
    if values.len() < 2 { return 0.0; }
    let m = mean(values);
    values.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (values.len() as f64 - 1.0)
}
pub fn stddev_pop(values: &[f64]) -> f64 { variance_pop(values).sqrt() }
pub fn stddev_sample(values: &[f64]) -> f64 { variance_sample(values).sqrt() }

pub fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = values.len();
    if n % 2 == 1 { values[n / 2] } else { (values[n / 2 - 1] + values[n / 2]) / 2.0 }
}
pub fn percentile(values: &mut [f64], p: f64) -> f64 {
    if values.is_empty() { return 0.0; }
    let p = p.clamp(0.0, 1.0);
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if values.len() == 1 { return values[0]; }
    let n = values.len();
    let rank = p * (n - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let frac = rank - lower as f64;
    if lower == upper { values[lower] } else { values[lower] * (1.0 - frac) + values[upper] * frac }
}

pub fn z_score(values: &[f64]) -> Vec<f64> {
    let m = mean(values); let s = stddev_pop(values);
    if s == 0.0 { return vec![0.0; values.len()]; }
    values.iter().map(|x| (x - m) / s).collect()
}
pub fn min_max_scale(values: &[f64]) -> Vec<f64> {
    if values.is_empty() { return Vec::new(); }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range == 0.0 { return vec![0.5; values.len()]; }
    values.iter().map(|x| (x - min) / range).collect()
}

pub struct Welford { count: u64, mean: f64, m2: f64 }
impl Welford {
    pub fn new() -> Self { Self { count: 0, mean: 0.0, m2: 0.0 } }
    pub fn update(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }
    pub fn count(&self) -> u64 { self.count }
    pub fn mean(&self) -> f64 { self.mean }
    pub fn variance(&self) -> f64 { if self.count < 2 { 0.0 } else { self.m2 / (self.count - 1) as f64 } }
    pub fn stddev(&self) -> f64 { self.variance().sqrt() }
}
impl Default for Welford { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    fn approx(a: f64, b: f64, e: f64) -> bool { (a - b).abs() < e }
    #[test] fn t01_mean() { assert_eq!(mean(&[1.0,2.0,3.0,4.0,5.0]), 3.0); }
    #[test] fn t02_mean_empty() { assert_eq!(mean(&[]), 0.0); }
    #[test] fn t03_var_pop() { assert!(approx(variance_pop(&[1.0,2.0,3.0,4.0,5.0]), 2.0, 0.001)); }
    #[test] fn t04_var_sample() { assert!(approx(variance_sample(&[1.0,2.0,3.0,4.0,5.0]), 2.5, 0.001)); }
    #[test] fn t05_stddev_pop() { assert!(approx(stddev_pop(&[1.0,2.0,3.0,4.0,5.0]), 1.414, 0.01)); }
    #[test] fn t06_median_odd() { let mut v = vec![3.0,1.0,2.0]; assert_eq!(median(&mut v), 2.0); }
    #[test] fn t07_median_even() { let mut v = vec![4.0,1.0,3.0,2.0]; assert_eq!(median(&mut v), 2.5); }
    #[test] fn t08_p50() { let mut v = vec![1.0,2.0,3.0,4.0,5.0]; assert_eq!(percentile(&mut v, 0.5), 3.0); }
    #[test] fn t09_p90() { let mut v: Vec<f64> = (1..=100).map(|i| f64::from(i)).collect(); assert!(approx(percentile(&mut v, 0.9), 90.1, 0.001)); }
    #[test] fn t10_z() { let z = z_score(&[1.0,2.0,3.0,4.0,5.0]); assert!(approx(z[2], 0.0, 0.001)); }
    #[test] fn t11_minmax() { let s = min_max_scale(&[1.0,2.0,3.0,4.0,5.0]); assert!(approx(s[0], 0.0, 0.001)); assert!(approx(s[4], 1.0, 0.001)); }
    #[test] fn t12_welford() { let mut w = Welford::new(); for i in 1..=5 { w.update(f64::from(i)); } assert_eq!(w.count(), 5); assert_eq!(w.mean(), 3.0); assert!(approx(w.variance(), 2.5, 0.001)); }
    #[test] fn t13_welford_default() { let w = Welford::default(); assert_eq!(w.count(), 0); }
    #[test] fn t14_welford_matches() { let v = vec![2.0,4.0,4.0,4.0,5.0,5.0,7.0,9.0]; let mut w = Welford::new(); for &x in &v { w.update(x); } assert!(approx(w.mean(), mean(&v), 0.001)); assert!(approx(w.variance(), variance_sample(&v), 0.001)); }
}
