use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct PublicConfigResponse {
    pub room: PublicRoomConfig,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct PublicRoomConfig {
    pub expiry: PublicRoomExpiryConfig,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[cfg_attr(feature = "typescript-export", derive(ts_rs::TS, schemars::JsonSchema))]
#[cfg_attr(feature = "typescript-export", ts(export))]
pub struct PublicRoomExpiryConfig {
    #[cfg_attr(feature = "typescript-export", ts(type = "number[]"))]
    pub allowed_ages_seconds: Vec<i64>,
    #[cfg_attr(feature = "typescript-export", ts(type = "number"))]
    pub default_age_seconds: i64,
}
