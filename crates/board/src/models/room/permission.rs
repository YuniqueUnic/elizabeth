#![allow(unused)]

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use sqlx::{
    Decode, Encode, Type,
    encode::IsNull,
    error::BoxDynError,
    sqlite::{Sqlite, SqliteTypeInfo, SqliteValueRef},
};
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{
        ObjectBuilder,
        schema::{SchemaType, Type as SchemaPrimitive},
    },
};

bitflags! {
    /// 房间权限
    ///
    /// 默认只有 VIEW_ONLY 权限，可以通过 with_* 方法添加权限
    /// - VIEW_ONLY: 只能查看
    /// - EDITABLE: 可以编辑
    /// - SHARE: 可以分享
    /// - DELETE: 可以删除
    /// - VIEW_ONLY | EDITABLE | SHARE | DELETE 可以 everything
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct RoomPermission: u8 {
        const VIEW_ONLY = 1;
        const EDITABLE = 1 << 1;
        const SHARE = 1 << 2;
        const DELETE = 1 << 3;
    }
}

impl Default for RoomPermission {
    fn default() -> Self {
        RoomPermission::VIEW_ONLY
    }
}

impl RoomPermission {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_edit(mut self) -> Self {
        self |= RoomPermission::EDITABLE;
        self
    }
    pub fn with_share(mut self) -> Self {
        self |= RoomPermission::SHARE;
        self
    }
    pub fn with_delete(mut self) -> Self {
        self |= RoomPermission::DELETE;
        self
    }
    pub fn with_all(mut self) -> Self {
        self |= RoomPermission::EDITABLE;
        self |= RoomPermission::SHARE;
        self |= RoomPermission::DELETE;
        self
    }
}

impl RoomPermission {
    pub fn can_view(&self) -> bool {
        self.contains(RoomPermission::VIEW_ONLY)
    }
    pub fn can_edit(&self) -> bool {
        self.contains(RoomPermission::EDITABLE)
    }
    pub fn can_share(&self) -> bool {
        self.contains(RoomPermission::SHARE)
    }
    pub fn can_delete(&self) -> bool {
        self.contains(RoomPermission::DELETE)
    }
    pub fn can_do_all(&self) -> bool {
        self.can_view() && self.can_edit() && self.can_share() && self.can_delete()
    }
}

impl PartialSchema for RoomPermission {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        ObjectBuilder::new()
            .schema_type(SchemaType::from(SchemaPrimitive::Integer))
            .description(Some(
                "房间权限位掩码，使用 bitflags 表示：1=VIEW_ONLY, 2=EDITABLE, 4=SHARE, 8=DELETE。",
            ))
            .into()
    }
}

impl ToSchema for RoomPermission {}

impl Type<Sqlite> for RoomPermission {
    fn type_info() -> SqliteTypeInfo {
        <u8 as Type<Sqlite>>::type_info()
    }
}

impl Encode<'_, Sqlite> for RoomPermission {
    fn encode(
        self,
        buf: &mut <Sqlite as sqlx::Database>::ArgumentBuffer<'_>,
    ) -> Result<IsNull, BoxDynError> {
        <u8 as Encode<Sqlite>>::encode(self.bits(), buf)
    }

    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as sqlx::Database>::ArgumentBuffer<'_>,
    ) -> Result<IsNull, BoxDynError> {
        <u8 as Encode<Sqlite>>::encode(self.bits(), buf)
    }
}

impl Decode<'_, Sqlite> for RoomPermission {
    fn decode(value: SqliteValueRef<'_>) -> Result<Self, BoxDynError> {
        let raw = <u8 as Decode<Sqlite>>::decode(value)?;
        RoomPermission::from_bits(raw)
            .ok_or_else(|| format!("invalid RoomPermission bits: {}", raw).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_permission() {
        let permission = RoomPermission::new().with_all();
        println!("permission: {:?}", permission);
        assert!(permission.can_view());
        assert!(permission.can_edit());
        assert!(permission.can_share());
        assert!(permission.can_delete());
        assert!(permission.can_do_all());
    }

    #[test]
    fn test_room_default() {
        let permission = RoomPermission::default();
        assert!(!permission.is_empty());
        assert!(permission.can_view());
        assert!(!permission.can_edit());
        assert!(!permission.can_share());
        assert!(!permission.can_delete());
        assert!(!permission.can_do_all());
    }
}
