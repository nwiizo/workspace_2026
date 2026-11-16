//! Copied into the work copy of ../rust-grokking-ai-algorithms as an example by
//! scripts/grokking.sh. Prints the intermediate values of the Gaussian process
//! test that fails under Relax, so rustc and Relax can be compared line by line.

use grokking_ai_algorithms::numeric::solve_linear;
use grokking_ai_algorithms::tuning::GaussianProcess;

struct Fitted {
    mean: f64,
    rhs: Vec<f64>,
}

/// The first lines of `GaussianProcess::fit`, with `mean`'s type inferred from
/// the struct field as in the original.
fn fit_prefix(y: &[f64]) -> Fitted {
    let mean = y.iter().map(|v| v / y.len() as f64).sum();
    let rhs = y.iter().map(|v| v - mean).collect::<Vec<_>>();
    Fitted { mean, rhs }
}

fn main() {
    let annotated: f64 = [2.0f64, 4.0].iter().map(|v| v / 2.0).sum();
    println!("mean (annotated) = {annotated:?}");
    let fitted = fit_prefix(&[2.0, 4.0]);
    println!("mean (inferred) = {:?}, rhs = {:?}", fitted.mean, fitted.rhs);
    let k = (-0.5f64 * 4.0 / 1.0f64.powi(2)).exp();
    println!("kernel(-1, 1) = {k:?}");
    let covariance = vec![vec![1.0 + 1e-8, k], vec![k, 1.0 + 1e-8]];
    println!("solve_linear = {:?}", solve_linear(&covariance, &[-1.0, 1.0]));
    match GaussianProcess::fit(&[vec![-1.0], vec![1.0]], &[2.0, 4.0], 1.0, 1e-8) {
        Ok(gp) => {
            for q in [-1.0, 1.0, 5.0] {
                println!("predict({q}) = {:?}", gp.predict(&[q]));
            }
        }
        Err(e) => println!("fit failed: {e}"),
    }
}
