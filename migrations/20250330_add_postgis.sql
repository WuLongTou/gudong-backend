-- 添加PostGIS扩展并优化地理空间查询
-- 执行日期：2025-03-30

-- 添加PostGIS扩展
CREATE EXTENSION IF NOT EXISTS postgis;

-- 为user_activities表添加地理空间列和索引
ALTER TABLE user_activities ADD COLUMN IF NOT EXISTS geom geography(POINT, 4326);

-- 更新现有数据的geom列
UPDATE user_activities
SET geom = ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)::geography
WHERE geom IS NULL AND longitude IS NOT NULL AND latitude IS NOT NULL;

-- 创建触发器函数，自动更新geom列
CREATE OR REPLACE FUNCTION update_activity_geom()
RETURNS TRIGGER AS $$
BEGIN
    NEW.geom = ST_SetSRID(ST_MakePoint(NEW.longitude, NEW.latitude), 4326)::geography;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 创建触发器
DROP TRIGGER IF EXISTS activity_geom_trigger ON user_activities;
CREATE TRIGGER activity_geom_trigger
BEFORE INSERT OR UPDATE OF longitude, latitude ON user_activities
FOR EACH ROW EXECUTE FUNCTION update_activity_geom();

-- 为user_activities创建地理空间索引
CREATE INDEX IF NOT EXISTS idx_user_activities_geom ON user_activities USING GIST(geom);

-- 为groups表添加地理空间列和索引
ALTER TABLE groups ADD COLUMN IF NOT EXISTS geom geography(POINT, 4326);

-- 更新现有数据的geom列
UPDATE groups
SET geom = ST_SetSRID(ST_MakePoint(longitude, latitude), 4326)::geography
WHERE geom IS NULL AND longitude IS NOT NULL AND latitude IS NOT NULL;

-- 创建触发器函数，自动更新geom列
CREATE OR REPLACE FUNCTION update_group_geom()
RETURNS TRIGGER AS $$
BEGIN
    NEW.geom = ST_SetSRID(ST_MakePoint(NEW.longitude, NEW.latitude), 4326)::geography;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 创建触发器
DROP TRIGGER IF EXISTS group_geom_trigger ON groups;
CREATE TRIGGER group_geom_trigger
BEFORE INSERT OR UPDATE OF longitude, latitude ON groups
FOR EACH ROW EXECUTE FUNCTION update_group_geom();

-- 为groups创建地理空间索引
CREATE INDEX IF NOT EXISTS idx_groups_geom ON groups USING GIST(geom); 