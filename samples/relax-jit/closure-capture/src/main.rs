//! Minimal form of the GaussianProcess::fit difference found with
//! ../probes/gp_probe.rs. rustc prints rhs = [-1.0, 1.0] for every variant.

struct Fitted {
    mean: f64,
    rhs: Vec<f64>,
}

/// As in the original: `mean`'s type is inferred from the struct field.
fn inferred(y: &[f64]) -> Fitted {
    let mean = y.iter().map(|v| v / y.len() as f64).sum();
    let rhs = y.iter().map(|v| v - mean).collect::<Vec<_>>();
    Fitted { mean, rhs }
}

/// The same with `let mean: f64`.
fn annotated(y: &[f64]) -> Fitted {
    let mean: f64 = y.iter().map(|v| v / y.len() as f64).sum();
    let rhs = y.iter().map(|v| v - mean).collect::<Vec<_>>();
    Fitted { mean, rhs }
}

/// The closure replaced by a loop.
fn looped(y: &[f64]) -> Fitted {
    let mean: f64 = y.iter().map(|v| v / y.len() as f64).sum();
    let mut rhs = Vec::with_capacity(y.len());
    for v in y {
        rhs.push(v - mean);
    }
    Fitted { mean, rhs }
}

fn main() {
    let y = [2.0, 4.0];
    for (name, f) in [
        ("inferred", inferred as fn(&[f64]) -> Fitted),
        ("annotated", annotated),
        ("looped", looped),
    ] {
        let fitted = f(&y);
        println!("{name:9} mean = {:?}, rhs = {:?}", fitted.mean, fitted.rhs);
    }
}
