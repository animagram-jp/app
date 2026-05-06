// ============================================================
// ログスタック (Log Stack)
// ============================================================

pub enum LogStack {
    Skill {
        // todo: ロール結果など
    },
    Characteristic {
        // todo: ロール結果など
    },
    Message(String),
}

impl std::fmt::Display for LogStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // todo: format!
        write!(f, "")
    }
}
