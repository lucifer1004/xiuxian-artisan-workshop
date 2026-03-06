#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::llm::vision::deepseek) enum VisionModelKind {
    Deepseek,
    PaddleOcrVl,
    DotsOcr,
}

impl VisionModelKind {
    pub(in crate::llm::vision::deepseek) const DEFAULT: Self = Self::DotsOcr;

    pub(in crate::llm::vision::deepseek) const fn as_str(self) -> &'static str {
        match self {
            Self::Deepseek => "deepseek",
            Self::PaddleOcrVl => "paddle_ocr_vl",
            Self::DotsOcr => "dots_ocr",
        }
    }

    pub(in crate::llm::vision::deepseek) fn parse(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "deepseek" | "deepseek-ocr" | "deepseek_ocr" => Some(Self::Deepseek),
            "paddle" | "paddleocr" | "paddle-ocr" | "paddle_ocr" | "paddleocr-vl"
            | "paddleocr_vl" | "paddle-ocr-vl" | "paddle_ocr_vl" | "paddle_vl" => {
                Some(Self::PaddleOcrVl)
            }
            "dots" | "dotsocr" | "dots-ocr" | "dots_ocr" | "vl2" | "dots_vl2" | "dots-vl2" => {
                Some(Self::DotsOcr)
            }
            _ => None,
        }
    }

    #[cfg(feature = "vision-dots")]
    pub(in crate::llm::vision::deepseek) const fn as_core_kind(
        self,
    ) -> deepseek_ocr_core::ModelKind {
        match self {
            Self::Deepseek => deepseek_ocr_core::ModelKind::Deepseek,
            Self::PaddleOcrVl => deepseek_ocr_core::ModelKind::PaddleOcrVl,
            Self::DotsOcr => deepseek_ocr_core::ModelKind::DotsOcr,
        }
    }
}
