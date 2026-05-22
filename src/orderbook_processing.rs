pub use crate::tsc::{cycle_end, cycle_start};
use std::sync::atomic::{compiler_fence, Ordering};

#[inline]
pub fn black_box<T>(x: T) -> T {
    std::hint::black_box(x)
}

#[inline]
pub fn clobber() {
    compiler_fence(Ordering::SeqCst);
}

pub type BenchmarkFn = Box<dyn Fn(i32) -> u64 + Send + Sync>;

pub struct BenchmarkDef {
    pub name: &'static str,
    pub ops_per_iteration: u64,
    pub func: BenchmarkFn,
    pub fixed_iterations: i32,
}

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: &'static str,
    pub ops_per_iteration: u64,
    pub iterations: u64,
    pub total_cycles: u64,
}

pub struct Harness {
    pub benchmarks: Vec<BenchmarkDef>,
}

impl Harness {
    pub fn new() -> Self {
        Self {
            benchmarks: Vec::new(),
        }
    }

    pub fn add_benchmark(
        &mut self,
        name: &'static str,
        ops: u64,
        f: impl Fn(i32) -> u64 + 'static + Send + Sync,
        fixed_iters: i32,
    ) {
        self.benchmarks.push(BenchmarkDef {
            name,
            ops_per_iteration: ops,
            func: Box::new(f),
            fixed_iterations: fixed_iters,
        });
    }

    fn calibrate(f: &BenchmarkFn) -> i32 {
        f(1); // warm up
        let single_cycles = f(1);
        const TARGET_CYCLES: u64 = 1_000_000_000;
        let n = TARGET_CYCLES / single_cycles.max(1);
        (n as i32).clamp(3, 1000)
    }

    /// Runs all benchmarks, prints results as JSON (for CLI), returns exit code
    pub fn run(self) -> i32 {
        if self.benchmarks.is_empty() {
            eprintln!("No benchmarks to run.");
            return 1; // error code if nothing to run
        }
        let mut results: Vec<BenchmarkResult> = Vec::with_capacity(self.benchmarks.len());
        for def in &self.benchmarks {
            let iters = if def.fixed_iterations > 0 {
                def.fixed_iterations
            } else {
                Self::calibrate(&def.func)
            };
            let cycles = (def.func)(iters);
            results.push(BenchmarkResult {
                name: def.name,
                ops_per_iteration: def.ops_per_iteration,
                iterations: iters as u64,
                total_cycles: cycles,
            });
        }
        print!("{{\n  \"benchmarks\": [\n");
        for (i, r) in results.iter().enumerate() {
            let cycles_per_op =
                r.total_cycles as f64 / (r.iterations as f64 * r.ops_per_iteration as f64);
            print!("    {{\n");
            print!("      \"name\": \"{}\",\n", r.name);
            print!("      \"iterations\": {},\n", r.iterations);
            print!("      \"ops_per_iteration\": {},\n", r.ops_per_iteration);
            print!("      \"total_cycles\": {},\n", r.total_cycles);
            print!("      \"cycles_per_op\": {:.2}\n", cycles_per_op);
            if i + 1 < results.len() {
                print!("    }},\n");
            } else {
                print!("    }}\n");
            }
        }
        println!("  ]\n}}");
        0
    }

    /// Run a single benchmark (for scripting/FFI/interop usage)
    pub fn run_single(
        name: &'static str,
        ops: u64,
        f: impl Fn(i32) -> u64 + 'static + Send + Sync,
        fixed_iters: i32,
    ) -> BenchmarkResult {
        let benchmark = BenchmarkDef {
            name,
            ops_per_iteration: ops,
            func: Box::new(f),
            fixed_iterations: fixed_iters,
        };
        let iters = if fixed_iters > 0 {
            fixed_iters
        } else {
            Self::calibrate(&benchmark.func)
        };
        let cycles = (benchmark.func)(iters);
        BenchmarkResult {
            name,
            ops_per_iteration: ops,
            iterations: iters as u64,
            total_cycles: cycles,
        }
    }
}

// --- PyO3 Python bindings section ---
#[cfg(feature = "python")]
mod py_harness {
    use super::*;
    use pyo3::prelude::*;

    #[pyclass]
    pub struct PyBenchmarkResult {
        #[pyo3(get)]
        pub name: String,
        #[pyo3(get)]
        pub ops_per_iteration: u64,
        #[pyo3(get)]
        pub iterations: u64,
        #[pyo3(get)]
        pub total_cycles: u64,
    }

    #[pyfunction]
    pub fn bench_example(n: i32) -> PyBenchmarkResult {
        // This is a trivial Rust closure; replace with your real benchmark as needed
        let bench = |iters| {
            let mut acc = 0;
            for i in 0..iters {
                acc += i;
                clobber();
            }
            acc as u64
        };
        let res = Harness::run_single("bench_example", 1, bench, n);
        PyBenchmarkResult {
            name: res.name.to_string(),
            ops_per_iteration: res.ops_per_iteration,
            iterations: res.iterations,
            total_cycles: res.total_cycles,
        }
    }

    #[pymodule]
    pub fn orderbook_processing(_py: Python, m: &PyModule) -> PyResult<()> {
        m.add_class::<PyBenchmarkResult>()?;
        m.add_function(wrap_pyfunction!(bench_example, m)?)?;
        Ok(())
    }
}

// Expose the module at crate root (for maturin/pyo3)
#[cfg(feature = "python")]
pub use py_harness::*;
