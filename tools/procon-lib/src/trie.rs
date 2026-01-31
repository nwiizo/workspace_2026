//! Trie Data Structure
//!
//! - Standard Trie
//! - XOR Trie for maximum XOR queries

/// Trie for strings
///
/// # Example
/// ```
/// use procon_lib::trie::Trie;
///
/// let mut trie = Trie::new();
/// trie.insert(b"apple");
/// trie.insert(b"app");
/// trie.insert(b"application");
///
/// assert!(trie.contains(b"app"));
/// assert!(trie.contains(b"apple"));
/// assert!(!trie.contains(b"ap"));
/// assert!(trie.starts_with(b"ap"));
/// assert_eq!(trie.count_prefix(b"app"), 3);
/// ```
#[derive(Default)]
pub struct Trie {
    children: [Option<Box<Trie>>; 26],
    is_end: bool,
    count: usize,
}

impl Trie {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a word
    pub fn insert(&mut self, word: &[u8]) {
        let mut node = self;
        for &c in word {
            let idx = (c - b'a') as usize;
            node.count += 1;
            node = node.children[idx].get_or_insert_with(|| Box::new(Trie::default()));
        }
        node.count += 1;
        node.is_end = true;
    }

    /// Check if word exists
    pub fn contains(&self, word: &[u8]) -> bool {
        self.find(word).map_or(false, |node| node.is_end)
    }

    /// Check if any word starts with prefix
    pub fn starts_with(&self, prefix: &[u8]) -> bool {
        self.find(prefix).is_some()
    }

    /// Count words with given prefix
    pub fn count_prefix(&self, prefix: &[u8]) -> usize {
        self.find(prefix).map_or(0, |node| node.count)
    }

    fn find(&self, word: &[u8]) -> Option<&Trie> {
        let mut node = self;
        for &c in word {
            let idx = (c - b'a') as usize;
            node = node.children[idx].as_ref()?;
        }
        Some(node)
    }

    /// Delete a word (returns true if word existed)
    pub fn delete(&mut self, word: &[u8]) -> bool {
        fn delete_recursive(node: &mut Trie, word: &[u8], depth: usize) -> bool {
            if depth == word.len() {
                if !node.is_end {
                    return false;
                }
                node.is_end = false;
                node.count -= 1;
                return true;
            }

            let idx = (word[depth] - b'a') as usize;
            if let Some(child) = &mut node.children[idx] {
                if delete_recursive(child, word, depth + 1) {
                    node.count -= 1;
                    if child.count == 0 {
                        node.children[idx] = None;
                    }
                    return true;
                }
            }
            false
        }

        delete_recursive(self, word, 0)
    }
}

/// XOR Trie for finding maximum XOR
///
/// # Example
/// ```
/// use procon_lib::trie::XorTrie;
///
/// let mut trie = XorTrie::new(30);  // up to 30 bits
/// trie.insert(5);   // 101
/// trie.insert(2);   // 010
/// trie.insert(3);   // 011
///
/// // Maximum XOR with 6 (110) is 6 ^ 5 = 3... wait
/// // 6 = 110, 5 = 101, XOR = 011 = 3
/// // 6 = 110, 2 = 010, XOR = 100 = 4
/// // 6 = 110, 3 = 011, XOR = 101 = 5
/// assert_eq!(trie.max_xor(6), 5);  // 6 ^ 3 = 5
/// ```
pub struct XorTrie {
    children: [Option<Box<XorTrie>>; 2],
    count: usize,
    max_bits: usize,
}

impl XorTrie {
    /// Create a new XOR trie with specified maximum bits
    pub fn new(max_bits: usize) -> Self {
        Self {
            children: [None, None],
            count: 0,
            max_bits,
        }
    }

    /// Insert a number
    pub fn insert(&mut self, mut num: u64) {
        let mut node = self;
        for i in (0..self.max_bits).rev() {
            let bit = ((num >> i) & 1) as usize;
            node.count += 1;
            node = node.children[bit].get_or_insert_with(|| {
                Box::new(XorTrie {
                    children: [None, None],
                    count: 0,
                    max_bits: 0,
                })
            });
        }
        node.count += 1;
    }

    /// Delete a number (returns true if it existed)
    pub fn delete(&mut self, num: u64) -> bool {
        fn delete_recursive(node: &mut XorTrie, num: u64, bit: usize) -> bool {
            if bit == 0 {
                if node.count == 0 {
                    return false;
                }
                node.count -= 1;
                return true;
            }

            let b = ((num >> (bit - 1)) & 1) as usize;
            if let Some(child) = &mut node.children[b] {
                if delete_recursive(child, num, bit - 1) {
                    node.count -= 1;
                    if child.count == 0 {
                        node.children[b] = None;
                    }
                    return true;
                }
            }
            false
        }

        delete_recursive(self, num, self.max_bits)
    }

    /// Find maximum XOR with given number
    ///
    /// Returns 0 if trie is empty
    pub fn max_xor(&self, num: u64) -> u64 {
        if self.count == 0 {
            return 0;
        }

        let mut node = self;
        let mut result = 0u64;

        for i in (0..self.max_bits).rev() {
            let bit = ((num >> i) & 1) as usize;
            let opposite = 1 - bit;

            // Try to go opposite direction for maximum XOR
            if node.children[opposite].as_ref().map_or(false, |c| c.count > 0) {
                result |= 1 << i;
                node = node.children[opposite].as_ref().unwrap();
            } else if node.children[bit].as_ref().map_or(false, |c| c.count > 0) {
                node = node.children[bit].as_ref().unwrap();
            } else {
                break;
            }
        }

        result
    }

    /// Find minimum XOR with given number
    pub fn min_xor(&self, num: u64) -> u64 {
        if self.count == 0 {
            return u64::MAX;
        }

        let mut node = self;
        let mut result = 0u64;

        for i in (0..self.max_bits).rev() {
            let bit = ((num >> i) & 1) as usize;

            // Try to go same direction for minimum XOR
            if node.children[bit].as_ref().map_or(false, |c| c.count > 0) {
                node = node.children[bit].as_ref().unwrap();
            } else if node.children[1 - bit].as_ref().map_or(false, |c| c.count > 0) {
                result |= 1 << i;
                node = node.children[1 - bit].as_ref().unwrap();
            } else {
                break;
            }
        }

        result
    }

    /// Check if trie is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get number of elements
    pub fn len(&self) -> usize {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trie_basic() {
        let mut trie = Trie::new();
        trie.insert(b"apple");
        trie.insert(b"app");
        trie.insert(b"application");

        assert!(trie.contains(b"app"));
        assert!(trie.contains(b"apple"));
        assert!(!trie.contains(b"ap"));
        assert!(!trie.contains(b"apples"));
    }

    #[test]
    fn test_trie_prefix() {
        let mut trie = Trie::new();
        trie.insert(b"apple");
        trie.insert(b"app");
        trie.insert(b"application");

        assert!(trie.starts_with(b"ap"));
        assert!(trie.starts_with(b"app"));
        assert!(!trie.starts_with(b"b"));

        assert_eq!(trie.count_prefix(b"app"), 3);
        assert_eq!(trie.count_prefix(b"apple"), 1);
    }

    #[test]
    fn test_trie_delete() {
        let mut trie = Trie::new();
        trie.insert(b"app");
        trie.insert(b"apple");

        assert!(trie.contains(b"app"));
        assert!(trie.delete(b"app"));
        assert!(!trie.contains(b"app"));
        assert!(trie.contains(b"apple"));
    }

    #[test]
    fn test_xor_trie() {
        let mut trie = XorTrie::new(30);
        trie.insert(5);
        trie.insert(2);
        trie.insert(3);

        // 6 ^ 3 = 5 (maximum)
        assert_eq!(trie.max_xor(6), 5);
        // 6 ^ 5 = 3
        // 6 ^ 2 = 4
        // 6 ^ 3 = 5 (max)
    }

    #[test]
    fn test_xor_trie_min() {
        let mut trie = XorTrie::new(30);
        trie.insert(5);
        trie.insert(2);
        trie.insert(3);

        // 6 ^ 2 = 4 (minimum)
        assert_eq!(trie.min_xor(6), 4);
    }

    #[test]
    fn test_xor_trie_delete() {
        let mut trie = XorTrie::new(30);
        trie.insert(5);
        trie.insert(5);

        assert_eq!(trie.len(), 2);
        assert!(trie.delete(5));
        assert_eq!(trie.len(), 1);
        assert!(trie.delete(5));
        assert!(trie.is_empty());
    }
}
