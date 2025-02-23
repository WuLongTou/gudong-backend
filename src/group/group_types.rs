use crate::common::MapLocation;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct NewGroupRequest {
    pub name: String,
    pub location: MapLocation,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct NewGroupResponse {
    pub group_id: String,
    pub name: String,
    pub location: MapLocation,
    pub location_name: String,
    pub member_count: i32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct QueryGroupInfoRequestByName {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct QueryGroupInfoRequestByLocation {
    pub location: MapLocation,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct QueryGroupInfoResponse {
    pub group_id: String,
    pub name: String,
    pub location: MapLocation,
    pub location_name: String,
    pub member_count: i32,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct JoinGroupRequest {
    pub group_id: String,
    pub user_id: String,
    pub room_access_token: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct JoinGroupResponse {
    pub success: bool,
    pub joined_at: Option<String>,
}
