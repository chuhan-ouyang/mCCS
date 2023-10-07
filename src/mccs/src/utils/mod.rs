pub mod pool;
#[macro_export]
macro_rules! cuda_warning {
    ($cuda_op:expr) => {{
        let e = $cuda_op;
        if e != cuda_runtime_sys::cudaError::cudaSuccess {
            eprintln!("CUDA failed at {}:{}", file!(), line!())
        }
    }};
}
