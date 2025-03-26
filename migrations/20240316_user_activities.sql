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

-- 创建索引
CREATE INDEX idx_user_locations_user_id ON user_locations(user_id);
CREATE INDEX idx_user_locations_coords ON user_locations(latitude, longitude);
CREATE INDEX idx_user_locations_updated ON user_locations(updated_at DESC);

CREATE INDEX idx_user_activities_user_id ON user_activities(user_id);
CREATE INDEX idx_user_activities_coords ON user_activities(latitude, longitude);
CREATE INDEX idx_user_activities_created ON user_activities(created_at DESC);
CREATE INDEX idx_user_activities_type ON user_activities(activity_type); 