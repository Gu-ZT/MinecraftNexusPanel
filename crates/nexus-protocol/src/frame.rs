#[derive(Debug, Eq, PartialEq)]
pub struct Frame<'a> {
    payload: &'a [u8],
    consumed: usize,
}

impl<'a> Frame<'a> {
    pub(crate) const fn new(payload: &'a [u8], consumed: usize) -> Self {
        Self { payload, consumed }
    }

    #[must_use]
    pub const fn consumed(&self) -> usize {
        self.consumed
    }

    #[must_use]
    pub const fn payload(&self) -> &'a [u8] {
        self.payload
    }
}
