/// ソースコード上のバイトオフセット範囲
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// 開始バイトオフセット（含む）
    pub start: usize,
    /// 終了バイトオフセット（含まない）
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// 2つの Span を結合して、両方を含む最小の Span を返す
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// ダミーの Span（テスト用）
    pub fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}
