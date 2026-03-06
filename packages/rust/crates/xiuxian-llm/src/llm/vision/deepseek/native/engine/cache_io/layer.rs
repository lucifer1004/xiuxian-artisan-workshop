#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::llm::vision::deepseek::native::engine) enum CacheLayer {
    Local,
    Valkey,
}

impl CacheLayer {
    pub(in crate::llm::vision::deepseek::native::engine) fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Valkey => "valkey",
        }
    }
}
