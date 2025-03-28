# GeoTrack Backend

这是 GeoTrack 项目的后端服务器，使用 Rust + Axum + Redis + PostgreSQL 构建。

## 功能特性

- 用户管理
  - 用户注册（支持用户名、密码和昵称）
  - 临时用户创建
  - 用户登录
  - 用户信息更新
  - 密码重置（通过恢复码）

- 群组管理
  - 创建群组（支持位置信息和可选密码）
  - 按 ID、名称或位置搜索群组
  - 加入/退出群组
  - 群组成员管理

- 消息系统
  - 发送群组消息
  - 获取群组消息历史

- 安全特性
  - JWT 认证
  - 请求限流
  - 密码加密存储
  - CORS 支持

## 环境要求

- Rust 1.75 或更高版本
- PostgreSQL 12 或更高版本
- Redis 6 或更高版本

## 安装和设置

1. 克隆仓库：
   ```bash
   git clone <repository-url>
   cd geotrack/backend
   ```

2. 创建并配置 `.env` 文件：
   ```bash
   cp .env.example .env
   # 编辑 .env 文件，设置数据库连接等配置
   ```

3. 设置数据库：
   ```bash
   # 创建数据库
   createdb geotrack

   # 运行数据库迁移
   sqlx database create
   sqlx migrate run
   ```

4. 安装依赖并运行：
   ```bash
   cargo build
   cargo run
   ```

## API 文档

### 用户相关

#### POST /api/users/register
注册新用户
```json
{
    "user_id": "string",
    "password": "string",
    "nickname": "string"
}
```

#### POST /api/users/temporary
创建临时用户（无需请求体）

#### POST /api/users/login
用户登录
```json
{
    "user_id": "string",
    "password": "string"
}
```

#### POST /api/users/update
更新用户信息（需要认证）
```json
{
    "nickname": "string",
    "password": "string"
}
```

#### POST /api/users/reset-password
重置密码
```json
{
    "user_id": "string",
    "recovery_code": "string",
    "new_password": "string"
}
```

### 群组相关

#### POST /api/groups/create
创建群组（需要认证）
```json
{
    "name": "string",
    "location_name": "string",
    "latitude": 0.0,
    "longitude": 0.0,
    "description": "string",
    "password": "string"
}
```

#### GET /api/groups/by-id?id=xxx
按 ID 查询群组

#### GET /api/groups/by-name?name=xxx
按名称查询群组

#### GET /api/groups/by-location?latitude=0.0&longitude=0.0&radius=1000
按位置查询群组

#### POST /api/groups/join
加入群组（需要认证）
```json
{
    "group_id": "string",
    "password": "string"
}
```

#### POST /api/groups/leave
退出群组（需要认证）
```json
{
    "id": "string"
}
```

### 消息相关

#### POST /api/messages/create
发送消息（需要认证）
```json
{
    "group_id": "string",
    "content": "string"
}
```

#### POST /api/messages/get
获取消息（需要认证）
```json
{
    "group_id": "string",
    "message_id": "string",
    "limit": 50
}
```

## 错误处理

所有 API 响应都遵循以下格式：

成功响应：
```json
{
    "data": {
        // 响应数据
    }
}
```

错误响应：
```json
{
    "error": "错误信息"
}
```

常见 HTTP 状态码：
- 200: 成功
- 201: 创建成功
- 400: 请求参数错误
- 401: 未认证或认证失败
- 403: 权限不足
- 404: 资源不存在
- 429: 请求过于频繁
- 500: 服务器内部错误

## 开发

### 添加新的数据库迁移

```bash
sqlx migrate add <migration-name>
sqlx migrate run
```

### 运行测试

```bash
cargo test
```

### 构建发布版本

```bash
cargo build --release
```

## API类型规范

为确保前后端接口类型定义一致且明确，我们采用以下规范：

### 类型命名规范

- API返回类型：`ApiResponse<T>`，包含统一的结构：
  ```rust
  pub struct ApiResponse<T> {
      pub code: i32,         // 错误码，0表示成功
      pub msg: String,       // 错误消息
      pub resp_data: Option<T>, // 响应数据
  }
  ```

- 请求类型: `XxxRequest`，例如 `CreateUserRequest`
- 响应类型: `XxxResponse`，例如 `CreateUserResponse`
- 对于没有请求体的请求，使用 `EmptyRequest` 类型
- 对于没有响应体的响应，使用 `EmptyResponse` 类型

### API函数规范

所有API处理函数应明确定义请求和响应类型，并使用`ApiResponse`包装响应数据：

```rust
// 标准格式
pub async fn handler_name(
    Json(req): Json<SomeRequest>,
) -> impl IntoResponse {
    // 处理逻辑
    success_to_api_response(SomeResponse { ... })
    
    // 错误情况
    error_to_api_response(ERROR_CODE, "错误信息".to_string())
}

// 无请求体示例
pub async fn handler_without_req() -> impl IntoResponse {
    success_to_api_response(SomeResponse { ... })
}

// 无响应体示例
pub async fn handler_without_resp(
    Json(req): Json<SomeRequest>,
) -> impl IntoResponse {
    // 处理逻辑
    success_to_api_response(EmptyResponse {})
}
```

这样做可以确保前后端类型定义的一致性，方便接口对接和代码维护。 