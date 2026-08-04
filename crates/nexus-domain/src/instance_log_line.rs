use serde::Deserialize;
use serde::Serialize;

use crate::InstanceLogStream;

/// 实例输出流中的一行带游标日志。
///
/// 游标由 Core 生成，用于在日志分页和实时订阅之间恢复读取位置；它不是文件
/// 行号，调用方不应自行推导或排序替代值。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceLogLine {
    cursor: String,
    occurred_at: String,
    stream: InstanceLogStream,
    line: String,
}

impl InstanceLogLine {
    /// 创建日志行。
    #[must_use]
    pub const fn new(
        cursor: String,
        occurred_at: String,
        stream: InstanceLogStream,
        line: String,
    ) -> Self {
        Self {
            cursor,
            occurred_at,
            stream,
            line,
        }
    }

    /// 返回日志游标。
    #[must_use]
    pub fn cursor(&self) -> &str {
        &self.cursor
    }

    /// 返回日志发生时间。
    #[must_use]
    pub fn occurred_at(&self) -> &str {
        &self.occurred_at
    }

    /// 返回日志来源流。
    #[must_use]
    pub const fn stream(&self) -> InstanceLogStream {
        self.stream
    }

    /// 返回去除行尾后的日志文本；具体保留策略由 Core 决定。
    #[must_use]
    pub fn line(&self) -> &str {
        &self.line
    }
}
