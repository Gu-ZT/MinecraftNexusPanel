/// 解码后的单个长度前缀帧。
///
/// `payload` 借用输入缓冲区，`consumed` 包含四字节长度头和负载，
/// 因而调用方可以在同一缓冲区中继续解析后续帧。
#[derive(Debug, Eq, PartialEq)]
pub struct Frame<'a> {
    payload: &'a [u8],
    consumed: usize,
}

impl<'a> Frame<'a> {
    pub(crate) const fn new(payload: &'a [u8], consumed: usize) -> Self {
        Self { payload, consumed }
    }

    /// 返回本帧在输入缓冲区中消耗的字节数。
    #[must_use]
    pub const fn consumed(&self) -> usize {
        self.consumed
    }

    /// 返回不包含长度头的帧负载。
    #[must_use]
    pub const fn payload(&self) -> &'a [u8] {
        self.payload
    }
}
