-- 创建用户表
CREATE TABLE users (
    user_id VARCHAR(255) PRIMARY KEY,
    nickname VARCHAR(255) NOT NULL,
    password_hash TEXT,
    recovery_code TEXT,
    is_temporary BOOLEAN NOT NULL DEFAULT false,
    public_user_id VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 创建群组表
CREATE TABLE groups (
    group_id VARCHAR(255) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    location_name VARCHAR(255) NOT NULL,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    description TEXT NOT NULL,
    password_hash TEXT,
    creator_id VARCHAR(255) NOT NULL REFERENCES users(user_id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    member_count INTEGER NOT NULL DEFAULT 0,
    CONSTRAINT latitude_range CHECK (latitude BETWEEN -90 AND 90),
    CONSTRAINT longitude_range CHECK (longitude BETWEEN -180 AND 180)
);

-- 创建群组成员表
CREATE TABLE group_members (
    group_id VARCHAR(255) REFERENCES groups(group_id),
    user_id VARCHAR(255) REFERENCES users(user_id),
    joined_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_active TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    role VARCHAR(50) DEFAULT 'member',
    PRIMARY KEY (group_id, user_id)
);

-- 创建消息表
CREATE TABLE messages (
    message_id VARCHAR(255) PRIMARY KEY,
    group_id VARCHAR(255) NOT NULL REFERENCES groups(group_id),
    user_id VARCHAR(255) NOT NULL REFERENCES users(user_id),
    content TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 创建用户位置表
CREATE TABLE user_locations (
    location_id SERIAL PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL REFERENCES users(user_id),
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    accuracy DOUBLE PRECISION,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT user_latitude_range CHECK (latitude BETWEEN -90 AND 90),
    CONSTRAINT user_longitude_range CHECK (longitude BETWEEN -180 AND 180)
);

-- 创建用户档案表
CREATE TABLE user_profiles (
    user_id VARCHAR(255) PRIMARY KEY REFERENCES users(user_id),
    status VARCHAR(255) DEFAULT '在线',
    bio TEXT,
    avatar TEXT,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 创建用户活动表
CREATE TABLE user_activities (
    activity_id VARCHAR(255) PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL REFERENCES users(user_id),
    activity_type VARCHAR(50) NOT NULL,
    activity_details TEXT,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    CONSTRAINT activity_latitude_range CHECK (latitude BETWEEN -90 AND 90),
    CONSTRAINT activity_longitude_range CHECK (longitude BETWEEN -180 AND 180)
);

-- 创建活动表
CREATE TABLE activities (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(36) NOT NULL,
    type VARCHAR(50) NOT NULL,
    content TEXT,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

-- 创建索引
CREATE INDEX idx_users_is_temporary ON users(is_temporary);
CREATE INDEX idx_groups_location ON groups(latitude, longitude);
CREATE INDEX idx_groups_name ON groups(name);
CREATE INDEX idx_messages_group_created ON messages(group_id, created_at DESC);
CREATE INDEX idx_group_members_user ON group_members(user_id);
CREATE INDEX idx_user_locations_user_id ON user_locations(user_id);
CREATE INDEX idx_user_locations_coords ON user_locations(latitude, longitude);
CREATE INDEX idx_user_locations_updated ON user_locations(updated_at DESC);
CREATE INDEX idx_user_activities_user_id ON user_activities(user_id);
CREATE INDEX idx_user_activities_coords ON user_activities(latitude, longitude);
CREATE INDEX idx_user_activities_created ON user_activities(created_at DESC);
CREATE INDEX idx_user_activities_type ON user_activities(activity_type);
CREATE INDEX activities_location_idx ON activities USING GIST (
    (ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)::geography)
);
-- CREATE INDEX activities_location_idx ON activities USING GIST (
--     geography(ST_SetSRID(ST_MakePoint(longitude, latitude), 4326))
-- );
CREATE INDEX activities_created_at_idx ON activities(created_at); 