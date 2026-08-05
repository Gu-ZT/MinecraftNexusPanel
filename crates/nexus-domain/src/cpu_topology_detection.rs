use serde::Deserialize;
use serde::Serialize;

/// 记录 CPU 拓扑信息的探测来源和可信度。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuTopologyDetection {
    source: String,
    confidence: String,
}

impl CpuTopologyDetection {
    /// 创建拓扑探测结果。
    ///
    /// 来源和置信度使用稳定文本，便于后续接入 Linux sysfs、Windows
    /// Processor Relationship 和 macOS/ARM 平台能力而不破坏现有响应结构。
    #[must_use]
    pub fn new(source: String, confidence: String) -> Self {
        Self { source, confidence }
    }

    /// 返回探测来源标识。
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// 返回探测置信度标识。
    #[must_use]
    pub fn confidence(&self) -> &str {
        &self.confidence
    }
}
