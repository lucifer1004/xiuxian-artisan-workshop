use tempfile::TempDir;
use xiuxian_wendao::KnowledgeStorage;

pub(super) type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn has_valkey() -> bool {
    if let Ok(url) = std::env::var("VALKEY_URL")
        && !url.trim().is_empty()
    {
        return true;
    }
    false
}

pub(super) fn create_storage(temp_dir: &TempDir) -> KnowledgeStorage {
    KnowledgeStorage::new(temp_dir.path().to_string_lossy().as_ref(), "knowledge")
}

pub(super) fn text_to_vector(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0_f32; 128];
    for (index, byte) in text.as_bytes().iter().enumerate() {
        let bucket = index % 128;
        vector[bucket] += f32::from(*byte) / 255.0;
    }
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}
