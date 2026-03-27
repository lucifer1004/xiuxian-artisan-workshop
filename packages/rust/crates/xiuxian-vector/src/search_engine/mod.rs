//! DataFusion-backed search-plane execution foundation.

mod context;
mod conversion;
mod parquet;

pub use context::{SearchEngineContext, SearchEnginePartitionColumn};
pub use conversion::{
    engine_batch_to_lance_batch, engine_batches_to_lance_batches, lance_batch_to_engine_batch,
    lance_batches_to_engine_batches,
};
pub use parquet::{write_engine_batches_to_parquet_file, write_lance_batches_to_parquet_file};
