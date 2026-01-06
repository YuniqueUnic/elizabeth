use crate::models::permission::RoomPermission;

#[test]
fn room_permission_allows_all_when_with_all() {
    let permission = RoomPermission::new().with_all();
    assert!(permission.can_view());
    assert!(permission.can_edit());
    assert!(permission.can_share());
    assert!(permission.can_delete());
    assert!(permission.can_do_all());
}

#[test]
fn room_permission_default_is_view_only() {
    let permission = RoomPermission::default();
    assert!(!permission.is_empty());
    assert!(permission.can_view());
    assert!(!permission.can_edit());
    assert!(!permission.can_share());
    assert!(!permission.can_delete());
    assert!(!permission.can_do_all());
}
