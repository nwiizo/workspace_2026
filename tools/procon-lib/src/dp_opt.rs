//! DP Optimization Techniques
//!
//! - Convex Hull Trick (CHT)
//! - Li Chao Tree
//! - Monotone Queue

/// Convex Hull Trick for minimum query
///
/// Maintains a set of lines y = ax + b and answers minimum y queries.
///
/// # Example
/// ```
/// use procon_lib::dp_opt::ConvexHullTrickMin;
///
/// let mut cht = ConvexHullTrickMin::new();
/// cht.add_line(2, 1);   // y = 2x + 1
/// cht.add_line(-1, 5);  // y = -x + 5
/// cht.add_line(1, 0);   // y = x
///
/// assert_eq!(cht.query(0), 0);   // min at x=0: y=x gives 0
/// assert_eq!(cht.query(3), 2);   // min at x=3: y=-x+5 gives 2
/// ```
#[derive(Default)]
pub struct ConvexHullTrickMin {
    lines: Vec<(i64, i64)>, // (a, b) represents y = ax + b
}

impl ConvexHullTrickMin {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if line c is unnecessary given lines a and b
    fn bad(a: (i64, i64), b: (i64, i64), c: (i64, i64)) -> bool {
        // (b.b - a.b) / (a.a - b.a) >= (c.b - b.b) / (b.a - c.a)
        // Cross multiply to avoid floating point
        (b.1 - a.1) as i128 * (b.0 - c.0) as i128 >= (c.1 - b.1) as i128 * (a.0 - b.0) as i128
    }

    /// Add line y = ax + b
    ///
    /// Lines must be added in decreasing order of slope for correctness.
    pub fn add_line(&mut self, a: i64, b: i64) {
        let new_line = (a, b);
        while self.lines.len() >= 2 {
            let n = self.lines.len();
            if Self::bad(self.lines[n - 2], self.lines[n - 1], new_line) {
                self.lines.pop();
            } else {
                break;
            }
        }
        self.lines.push(new_line);
    }

    fn eval(line: (i64, i64), x: i64) -> i64 {
        line.0 * x + line.1
    }

    /// Query minimum y at x
    ///
    /// Queries must be made in increasing order of x for O(1) amortized.
    pub fn query(&self, x: i64) -> i64 {
        if self.lines.is_empty() {
            return i64::MAX;
        }

        // Binary search for optimal line
        let mut lo = 0;
        let mut hi = self.lines.len();

        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if Self::eval(self.lines[mid - 1], x) > Self::eval(self.lines[mid], x) {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        Self::eval(self.lines[lo], x)
    }

    /// Query with pointer (for monotonic queries)
    pub fn query_monotonic(&self, x: i64, ptr: &mut usize) -> i64 {
        if self.lines.is_empty() {
            return i64::MAX;
        }

        while *ptr + 1 < self.lines.len()
            && Self::eval(self.lines[*ptr], x) > Self::eval(self.lines[*ptr + 1], x)
        {
            *ptr += 1;
        }

        Self::eval(self.lines[*ptr], x)
    }
}

/// Li Chao Tree for minimum line query
///
/// Supports arbitrary order of line additions and queries.
///
/// # Example
/// ```
/// use procon_lib::dp_opt::LiChaoTree;
///
/// let mut tree = LiChaoTree::new(-1_000_000_000, 1_000_000_000);
/// tree.add_line(2, 1);   // y = 2x + 1
/// tree.add_line(-1, 5);  // y = -x + 5
/// tree.add_line(1, 0);   // y = x
///
/// assert_eq!(tree.query(0), 0);
/// assert_eq!(tree.query(3), 2);
/// ```
pub struct LiChaoTree {
    nodes: Vec<Option<(i64, i64)>>,
    lo: i64,
    hi: i64,
}

impl LiChaoTree {
    /// Create a new Li Chao Tree for x in [lo, hi]
    pub fn new(lo: i64, hi: i64) -> Self {
        let size = 1 << 20; // Enough for most cases
        Self {
            nodes: vec![None; size],
            lo,
            hi,
        }
    }

    fn eval(line: (i64, i64), x: i64) -> i64 {
        line.0 * x + line.1
    }

    /// Add line y = ax + b
    pub fn add_line(&mut self, a: i64, b: i64) {
        self.add_line_inner(0, self.lo, self.hi, (a, b));
    }

    fn add_line_inner(&mut self, idx: usize, l: i64, r: i64, mut line: (i64, i64)) {
        if idx >= self.nodes.len() {
            return;
        }

        let mid = l + (r - l) / 2;

        let better_at_mid = match self.nodes[idx] {
            None => true,
            Some(cur) => Self::eval(line, mid) < Self::eval(cur, mid),
        };

        if better_at_mid {
            std::mem::swap(&mut self.nodes[idx], &mut Some(line));
            line = self.nodes[idx].unwrap_or(line);
        }

        if l == r {
            return;
        }

        let cur = match self.nodes[idx] {
            None => return,
            Some(cur) => cur,
        };

        if Self::eval(line, l) < Self::eval(cur, l) {
            self.add_line_inner(2 * idx + 1, l, mid, line);
        } else if Self::eval(line, r) < Self::eval(cur, r) {
            self.add_line_inner(2 * idx + 2, mid + 1, r, line);
        }
    }

    /// Query minimum y at x
    pub fn query(&self, x: i64) -> i64 {
        self.query_inner(0, self.lo, self.hi, x)
    }

    fn query_inner(&self, idx: usize, l: i64, r: i64, x: i64) -> i64 {
        if idx >= self.nodes.len() {
            return i64::MAX;
        }

        let result = match self.nodes[idx] {
            None => i64::MAX,
            Some(line) => Self::eval(line, x),
        };

        if l == r {
            return result;
        }

        let mid = l + (r - l) / 2;
        let child_result = if x <= mid {
            self.query_inner(2 * idx + 1, l, mid, x)
        } else {
            self.query_inner(2 * idx + 2, mid + 1, r, x)
        };

        result.min(child_result)
    }

    /// Add segment line (only valid for x in [xl, xr])
    pub fn add_segment(&mut self, a: i64, b: i64, xl: i64, xr: i64) {
        self.add_segment_inner(0, self.lo, self.hi, (a, b), xl, xr);
    }

    fn add_segment_inner(
        &mut self,
        idx: usize,
        l: i64,
        r: i64,
        line: (i64, i64),
        xl: i64,
        xr: i64,
    ) {
        if idx >= self.nodes.len() || r < xl || xr < l {
            return;
        }

        if xl <= l && r <= xr {
            self.add_line_inner(idx, l, r, line);
            return;
        }

        let mid = l + (r - l) / 2;
        self.add_segment_inner(2 * idx + 1, l, mid, line, xl, xr);
        self.add_segment_inner(2 * idx + 2, mid + 1, r, line, xl, xr);
    }
}

/// Monotone Queue for sliding window minimum/maximum
///
/// # Example
/// ```
/// use procon_lib::dp_opt::MonotoneQueue;
///
/// let a = vec![1, 3, -1, -3, 5, 3, 6, 7];
/// let k = 3;
///
/// let mut mq = MonotoneQueue::new();
/// let mut result = Vec::new();
///
/// for i in 0..a.len() {
///     mq.push(a[i], i);
///     if i >= k - 1 {
///         mq.pop_old(i - k + 1);
///         result.push(mq.get_min());
///     }
/// }
///
/// assert_eq!(result, vec![-1, -3, -3, -3, 3, 3]);
/// ```
#[derive(Default)]
pub struct MonotoneQueue {
    deque: std::collections::VecDeque<(i64, usize)>,
}

impl MonotoneQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push value with index
    pub fn push(&mut self, val: i64, idx: usize) {
        while !self.deque.is_empty() && self.deque.back().unwrap().0 >= val {
            self.deque.pop_back();
        }
        self.deque.push_back((val, idx));
    }

    /// Remove elements with index < min_idx
    pub fn pop_old(&mut self, min_idx: usize) {
        while !self.deque.is_empty() && self.deque.front().unwrap().1 < min_idx {
            self.deque.pop_front();
        }
    }

    /// Get minimum value in current window
    pub fn get_min(&self) -> i64 {
        self.deque.front().map_or(i64::MAX, |&(v, _)| v)
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.deque.is_empty()
    }
}

/// Sliding window minimum for array
///
/// # Example
/// ```
/// use procon_lib::dp_opt::sliding_window_min;
///
/// let a = vec![1, 3, -1, -3, 5, 3, 6, 7];
/// let result = sliding_window_min(&a, 3);
/// assert_eq!(result, vec![-1, -3, -3, -3, 3, 3]);
/// ```
pub fn sliding_window_min(a: &[i64], k: usize) -> Vec<i64> {
    let n = a.len();
    if k > n {
        return vec![];
    }

    let mut mq = MonotoneQueue::new();
    let mut result = Vec::with_capacity(n - k + 1);

    for i in 0..n {
        mq.push(a[i], i);
        if i >= k - 1 {
            mq.pop_old(i + 1 - k);
            result.push(mq.get_min());
        }
    }

    result
}

/// Sliding window maximum for array
pub fn sliding_window_max(a: &[i64], k: usize) -> Vec<i64> {
    let negated: Vec<i64> = a.iter().map(|&x| -x).collect();
    sliding_window_min(&negated, k)
        .into_iter()
        .map(|x| -x)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cht_min() {
        let mut cht = ConvexHullTrickMin::new();
        cht.add_line(2, 1);
        cht.add_line(1, 2);
        cht.add_line(0, 5);
        cht.add_line(-1, 10);

        assert_eq!(cht.query(0), 1);
        assert_eq!(cht.query(3), 5);
    }

    #[test]
    fn test_li_chao_tree() {
        let mut tree = LiChaoTree::new(-100, 100);
        tree.add_line(2, 1);
        tree.add_line(-1, 5);
        tree.add_line(1, 0);

        assert_eq!(tree.query(0), 0);
        assert_eq!(tree.query(3), 2);
        assert_eq!(tree.query(-5), -5);
    }

    #[test]
    fn test_monotone_queue() {
        let a = vec![1, 3, -1, -3, 5, 3, 6, 7];
        let k = 3;

        let mut mq = MonotoneQueue::new();
        let mut result = Vec::new();

        for i in 0..a.len() {
            mq.push(a[i], i);
            if i >= k - 1 {
                mq.pop_old(i - k + 1);
                result.push(mq.get_min());
            }
        }

        assert_eq!(result, vec![-1, -3, -3, -3, 3, 3]);
    }

    #[test]
    fn test_sliding_window_min() {
        let a = vec![1, 3, -1, -3, 5, 3, 6, 7];
        assert_eq!(sliding_window_min(&a, 3), vec![-1, -3, -3, -3, 3, 3]);
    }

    #[test]
    fn test_sliding_window_max() {
        let a = vec![1, 3, -1, -3, 5, 3, 6, 7];
        assert_eq!(sliding_window_max(&a, 3), vec![3, 3, 5, 5, 6, 7]);
    }
}
