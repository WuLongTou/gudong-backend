-- 启用PostGIS扩展（如果尚未启用）
CREATE EXTENSION IF NOT EXISTS postgis;

-- 创建活动表
CREATE TABLE IF NOT EXISTS activities (
    id VARCHAR(36) PRIMARY KEY,
    user_id VARCHAR(36) NOT NULL,
    type VARCHAR(50) NOT NULL,
    content TEXT,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- 创建位置索引
CREATE INDEX activities_location_idx ON activities USING GIST (
    ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)::geography
);

-- 创建时间索引
CREATE INDEX activities_created_at_idx ON activities(created_at);

-- 创建用户位置表
CREATE TABLE IF NOT EXISTS user_locations (
    user_id VARCHAR(36) PRIMARY KEY,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    last_active_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- 创建用户位置索引
CREATE INDEX user_locations_idx ON user_locations USING GIST (
    ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)::geography
); 