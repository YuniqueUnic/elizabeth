//! 协议层常量（仅包含与协议类型构造相关的默认值）

pub mod room {
    /// 房间默认最大内容大小 (50MB)
    pub const DEFAULT_MAX_ROOM_CONTENT_SIZE: i64 = 50 * 1024 * 1024;

    /// 房间默认最大进入次数
    pub const DEFAULT_MAX_TIMES_ENTER_ROOM: i64 = 100;
}
