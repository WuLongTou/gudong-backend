use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct MapLocation {
    pub latitude: f64,
    pub longitude: f64,
}

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

#[derive(Serialize, Deserialize)]
pub(crate) struct KeepAliveInGroupRequest {
    pub group_id: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct KeepAliveInGroupResponse {
    pub success: bool,
    pub keep_alive_at: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct LeaveGroupRequest {
    pub group_id: String,
    pub user_id: String,
    pub room_access_token: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct LeaveGroupResponse {
    pub success: bool,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SendMessageToGroupRequest {
    pub group_id: String,
    pub message: String,
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SendMessageToGroupResponse {
    pub success: bool,
    pub sent_at: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct QueryMessageFromGroupRequest {
    pub group_id: String,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct MessageFromGroupResponse {
    pub sender_id: String,
    pub sender_name: String,
    pub message: String,
    pub timestamp: i64,
}
