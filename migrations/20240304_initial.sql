-- 创建用户表
CREATE TABLE users (
    user_id VARCHAR(255) PRIMARY KEY,
    nickname VARCHAR(255) NOT NULL,
    password_hash TEXT,
    recovery_code TEXT,
    is_temporary BOOLEAN NOT NULL DEFAULT false,
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

-- 创建索引
CREATE INDEX idx_users_is_temporary ON users(is_temporary);
CREATE INDEX idx_groups_location ON groups(latitude, longitude);
CREATE INDEX idx_groups_name ON groups(name);
CREATE INDEX idx_messages_group_created ON messages(group_id, created_at DESC);
CREATE INDEX idx_group_members_user ON group_members(user_id); 