//! A sample crate to test RustProbe analysis.
//! Contains various patterns that exercise ownership operations.

fn main() {
    let data = generate_data(100);
    let processed = process_data(&data);
    let summary = summarize(processed);
    println!("Summary: {summary}");
}

fn generate_data(count: usize) -> Vec<String> {
    let mut result = Vec::new();
    for i in 0..count {
        result.push(format!("item_{i}"));
    }
    result
}

fn process_data(data: &[String]) -> Vec<String> {
    let mut output = Vec::new();
    for item in data {
        let mut owned = item.clone();
        owned.push_str("_processed");
        output.push(owned);
    }
    output
}

fn summarize(data: Vec<String>) -> String {
    let total = data.len();
    let first = data.into_iter().next().unwrap_or_default();
    format!("{total} items, first: {first}")
}

#[derive(Clone)]
#[allow(dead_code)]
struct Widget {
    name: String,
    value: i64,
}

#[allow(dead_code)]
impl Widget {
    fn new(name: &str, value: i64) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }

    fn transform(&self) -> Widget {
        Widget {
            name: format!("{}_transformed", self.name),
            value: self.value * 2,
        }
    }
}

impl Drop for Widget {
    fn drop(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_data() {
        let data = generate_data(5);
        assert_eq!(data.len(), 5);
        assert_eq!(data[0], "item_0");
    }

    #[test]
    fn test_process_data() {
        let input = vec!["hello".to_string()];
        let output = process_data(&input);
        assert_eq!(output[0], "hello_processed");
    }

    #[test]
    fn test_widget() {
        let w = Widget::new("test", 42);
        let t = w.transform();
        assert_eq!(t.value, 84);
    }
}
