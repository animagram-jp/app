use crate::js_client::{
    Operation, DomCmd,
    get_js_str, get_js_f64, 
    EventType, KeyName,
    Dom,
};
use crate::roll;

// ============================================================
// ログスタック (Log Stack)
// ============================================================

enum LogStack {
    Skill{  },
    Characteristic{  },
    Message(String),
}

impl display {
    // format!
}
