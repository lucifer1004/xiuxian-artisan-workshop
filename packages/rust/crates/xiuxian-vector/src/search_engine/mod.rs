//! Residual DataFusion-backed search-plane and live Arrow compute foundation.

mod context;
#[cfg(feature = "vector-store")]
mod conversion;
#[cfg(feature = "vector-store")]
mod parquet;

pub use context::{SearchEngineContext, SearchEnginePartitionColumn};
#[cfg(feature = "vector-store")]
pub use conversion::{
    engine_batch_to_lance_batch, engine_batches_to_lance_batches, lance_batch_to_engine_batch,
    lance_batches_to_engine_batches,
};
#[cfg(feature = "vector-store")]
pub use parquet::{write_engine_batches_to_parquet_file, write_lance_batches_to_parquet_file};
