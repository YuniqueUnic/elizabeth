use crate::models::content::{ContentType, RoomContent};

#[test]
fn set_text_records_utf8_byte_size() {
    let now = chrono::Utc::now().naive_utc();
    let mut content = RoomContent::builder()
        .id(1)
        .room_id(1)
        .content_type(ContentType::Text)
        .now(now)
        .build();

    content.set_text("你好世界".to_string());

    // "你好世界" 占用 12 个字节（每个汉字 3 个字节）
    assert_eq!(content.size, Some(12));
}

#[test]
fn set_url_records_utf8_byte_size() {
    let now = chrono::Utc::now().naive_utc();
    let mut content = RoomContent::builder()
        .id(1)
        .room_id(1)
        .content_type(ContentType::Url)
        .now(now)
        .build();

    content.set_url(
        "https://例子。测试/路径".to_string(),
        Some("text/html".to_string()),
    );

    // Calculate UTF-8 byte length
    let expected = "https://例子。测试/路径".len() as i64;
    assert_eq!(content.size, Some(expected));
}
