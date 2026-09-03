//! 授权决策核心的表驱动测试矩阵（PERMISSIONS_REDESIGN.md §10.1）。

use board_protocol::models::room::content::ContentType;
use board_protocol::models::room::role::{
    Capability, Grant, ROLE_ADMIN, ROLE_EDITOR, ROLE_READER, SYSTEM_ROLE_TEMPLATES, grants_to_json,
    parse_grant, parse_grants_json,
};

use crate::authz::{Decision, DenyReason, Principal, Resource, authorize};

fn principal<'a>(jti: &'a str, role: &'a str) -> Principal<'a> {
    Principal {
        jti,
        room_id: 1,
        role,
    }
}

fn room(room_id: i64) -> Resource<'static> {
    Resource::Room { room_id }
}

fn content(room_id: i64, content_type: ContentType, created_by: Option<&str>) -> Resource<'_> {
    Resource::Content {
        room_id,
        content_type,
        created_by_jti: created_by,
    }
}

fn grants(role_key: &str) -> Option<Vec<Grant>> {
    SYSTEM_ROLE_TEMPLATES
        .iter()
        .find(|template| template.key == role_key)
        .map(|template| template.capabilities.to_vec())
}

#[test]
fn manager_allows_room_level_capabilities() {
    let principal = principal("jti-1", ROLE_ADMIN);
    assert_eq!(
        authorize(
            grants(ROLE_ADMIN).as_deref(),
            &principal,
            Capability::RoomDelete,
            &room(1)
        ),
        Decision::Allow
    );
    assert_eq!(
        authorize(
            grants(ROLE_ADMIN).as_deref(),
            &principal,
            Capability::RoomSettingsUpdate,
            &room(1)
        ),
        Decision::Allow
    );
    assert_eq!(
        authorize(
            grants(ROLE_ADMIN).as_deref(),
            &principal,
            Capability::RoomRolesManage,
            &room(1)
        ),
        Decision::Allow
    );
}

#[test]
fn manager_allows_any_content_edit_and_delete() {
    let principal = principal("jti-1", ROLE_ADMIN);
    let someone_elses = content(1, ContentType::Text, Some("other-jti"));
    assert_eq!(
        authorize(
            grants(ROLE_ADMIN).as_deref(),
            &principal,
            Capability::MsgEdit,
            &someone_elses
        ),
        Decision::Allow
    );
    assert_eq!(
        authorize(
            grants(ROLE_ADMIN).as_deref(),
            &principal,
            Capability::MsgDelete,
            &someone_elses
        ),
        Decision::Allow
    );
    assert_eq!(
        authorize(
            grants(ROLE_ADMIN).as_deref(),
            &principal,
            Capability::FileDelete,
            &someone_elses
        ),
        Decision::Allow
    );
}

#[test]
fn reader_is_denied_write_capabilities() {
    let principal = principal("jti-1", ROLE_READER);
    let table = [
        (Capability::MsgSend, room(1)),
        (Capability::FileUpload, room(1)),
        (Capability::RoomDelete, room(1)),
        (Capability::RoomShare, room(1)),
        (Capability::FilePolicyManage, room(1)),
        (
            Capability::MsgEdit,
            content(1, ContentType::Text, Some("jti-1")),
        ),
        (
            Capability::MsgDelete,
            content(1, ContentType::Text, Some("jti-1")),
        ),
        (
            Capability::FileDelete,
            content(1, ContentType::File, Some("jti-1")),
        ),
    ];
    for (capability, resource) in table {
        assert_eq!(
            authorize(
                grants(ROLE_READER).as_deref(),
                &principal,
                capability,
                &resource
            ),
            Decision::Deny(DenyReason::CapabilityMissing),
            "reader must not hold `{capability}`"
        );
    }
}

#[test]
fn reader_read_capabilities_are_allowed() {
    let principal = principal("jti-1", ROLE_READER);
    assert_eq!(
        authorize(
            grants(ROLE_READER).as_deref(),
            &principal,
            Capability::MsgRead,
            &room(1)
        ),
        Decision::Allow
    );
    assert_eq!(
        authorize(
            grants(ROLE_READER).as_deref(),
            &principal,
            Capability::FileDownload,
            &room(1)
        ),
        Decision::Allow
    );
}

#[test]
fn editor_own_scope_only_covers_own_content() {
    let principal = principal("jti-1", ROLE_EDITOR);
    let own = content(1, ContentType::Text, Some("jti-1"));
    let others = content(1, ContentType::Text, Some("other-jti"));
    let anonymous = content(1, ContentType::Text, None);

    assert_eq!(
        authorize(
            grants(ROLE_EDITOR).as_deref(),
            &principal,
            Capability::MsgEdit,
            &own
        ),
        Decision::Allow
    );
    assert_eq!(
        authorize(
            grants(ROLE_EDITOR).as_deref(),
            &principal,
            Capability::MsgDelete,
            &own
        ),
        Decision::Allow
    );
    // editor 的 msg.delete 是 Own：删除他人内容被拒
    assert_eq!(
        authorize(
            grants(ROLE_EDITOR).as_deref(),
            &principal,
            Capability::MsgDelete,
            &others
        ),
        Decision::Deny(DenyReason::ScopeOwnViolation)
    );
    // 存量内容 created_by_jti = NULL：Own fail-closed。
    assert_eq!(
        authorize(
            grants(ROLE_EDITOR).as_deref(),
            &principal,
            Capability::MsgDelete,
            &anonymous
        ),
        Decision::Deny(DenyReason::ScopeOwnViolation)
    );
    // editor 的 msg.edit 是 Any：任何人的内容都可编辑
    assert_eq!(
        authorize(
            grants(ROLE_EDITOR).as_deref(),
            &principal,
            Capability::MsgEdit,
            &others
        ),
        Decision::Allow
    );
    assert_eq!(
        authorize(
            grants(ROLE_EDITOR).as_deref(),
            &principal,
            Capability::MsgEdit,
            &anonymous
        ),
        Decision::Allow
    );
}

#[test]
fn editor_any_scope_covers_others_content() {
    let principal = principal("jti-1", ROLE_EDITOR);
    let others = content(1, ContentType::Text, Some("other-jti"));
    assert_eq!(
        authorize(
            grants(ROLE_EDITOR).as_deref(),
            &principal,
            Capability::MsgEdit,
            &others
        ),
        Decision::Allow
    );
}

#[test]
fn every_role_is_denied_across_rooms() {
    for role in [ROLE_ADMIN, ROLE_EDITOR, ROLE_READER] {
        let principal = principal("jti-1", role);
        assert_eq!(
            authorize(
                grants(role).as_deref(),
                &principal,
                Capability::MsgRead,
                &room(2)
            ),
            Decision::Deny(DenyReason::RoomMismatch),
            "room isolation must hold for `{role}`"
        );
    }
}

#[test]
fn unknown_or_deleted_role_is_denied() {
    let principal = principal("jti-1", "removed-role");
    assert_eq!(
        authorize(None, &principal, Capability::MsgRead, &room(1)),
        Decision::Deny(DenyReason::RoleMissing)
    );
    assert_eq!(
        authorize(Some(&[]), &principal, Capability::MsgRead, &room(1)),
        Decision::Deny(DenyReason::CapabilityMissing)
    );
}

#[test]
fn own_grant_never_authorizes_room_resources() {
    // Own 作用域只对 Content 有意义：即便数据被篡改成 Own 的 msg.read，Room 资源也拒绝。
    let principal = principal("jti-1", ROLE_EDITOR);
    let grants = [Grant::own(Capability::MsgRead)];
    assert_eq!(
        authorize(Some(&grants), &principal, Capability::MsgRead, &room(1)),
        Decision::Deny(DenyReason::ScopeOwnViolation)
    );
}

#[test]
fn grant_compact_format_roundtrip() {
    let grants = SYSTEM_ROLE_TEMPLATES[1].capabilities;
    let json = grants_to_json(grants);
    assert_eq!(parse_grants_json(&json).unwrap(), grants.to_vec());
}

#[test]
fn grant_parse_fail_closed_cases() {
    // 未知能力
    assert!(parse_grant("room.explode").is_err());
    // 不可配作用域的能力带后缀
    assert!(parse_grant("room.delete:any").is_err());
    assert!(parse_grant("file.upload:own").is_err());
    // 未知作用域
    assert!(parse_grant("msg.edit:some").is_err());
    // 合法形态
    assert_eq!(
        parse_grant("msg.delete:own").unwrap(),
        Grant::own(Capability::MsgDelete)
    );
    // ownable 缺后缀 → Any
    assert_eq!(
        parse_grant("msg.edit").unwrap(),
        Grant::any(Capability::MsgEdit)
    );
    // 非法 JSON 数组
    assert!(parse_grants_json("not-json").is_err());
    assert!(parse_grants_json(r#"["ok","room.delete:any"]"#).is_err());
}
