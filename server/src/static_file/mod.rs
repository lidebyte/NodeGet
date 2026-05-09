use anyhow::Context;
use base64::Engine as _;
use nodeget_lib::error::NodegetError;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use std::path::{Component, Path, PathBuf};
use tracing::{debug, error, warn};

use crate::DB;
use crate::entity::static_file as static_entity;

pub mod cache;

/// 获取配置文件中的 `static_path，默认` `./static/`
pub fn get_static_path() -> String {
    crate::SERVER_CONFIG
        .get()
        .and_then(|lock| lock.read().ok())
        .map_or_else(
            || "./static/".to_owned(),
            |guard| {
                guard
                    .static_path
                    .clone()
                    .unwrap_or_else(|| "./static/".to_owned())
            },
        )
}

/// 校验 static name 的合法性
///
/// name 只作为 RPC / URL 的标识符，也会顺带落到磁盘提示信息里；
/// 但不会直接拼接磁盘路径（磁盘路径由 `path` 字段决定）。
/// 即便如此，仍严格限制字符集以避免跨层混淆。
pub fn validate_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        return Err(NodegetError::InvalidInput("name cannot be empty".to_owned()).into());
    }
    if name.len() > 128 {
        return Err(NodegetError::InvalidInput("name too long (max 128 chars)".to_owned()).into());
    }
    // 只允许字母、数字、下划线、短横线、点。禁止 `..`、`/`、`\` 等所有路径分隔符及控制字符
    let valid = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.');
    if !valid {
        return Err(NodegetError::InvalidInput(
            "name contains invalid characters (only [A-Za-z0-9_.-] are allowed)".to_owned(),
        )
        .into());
    }
    // 显式拒绝 `.` 与 `..`，以及任意全点组合
    if name.chars().all(|c| c == '.') {
        return Err(NodegetError::InvalidInput("name cannot be '.' or '..'".to_owned()).into());
    }
    Ok(())
}

/// 校验 `path`（即 static 记录里的子目录字段）的合法性
///
/// 语义：实际磁盘根 = `{static_path(config)}/{path}`。
/// 允许使用 `/` 作为子目录分隔符（例如 `"sites/blog-2026"`），
/// 但每一段必须通过 [`validate_name`] 等价的字符集校验，不允许
/// 绝对路径、`.` / `..` 穿透、Windows 盘符前缀等。
pub fn validate_sub_path(path: &str) -> anyhow::Result<()> {
    if path.is_empty() {
        return Err(NodegetError::InvalidInput("path cannot be empty".to_owned()).into());
    }
    if path.len() > 512 {
        return Err(NodegetError::InvalidInput("path too long (max 512 chars)".to_owned()).into());
    }
    // 整体粗筛：禁止反斜杠（Windows 路径分隔符），避免歧义
    if path.contains('\\') {
        return Err(NodegetError::InvalidInput("path cannot contain backslash".to_owned()).into());
    }

    let p = Path::new(path);
    let mut has_component = false;
    for component in p.components() {
        match component {
            Component::Normal(c) => {
                let segment = c.to_str().ok_or_else(|| {
                    NodegetError::InvalidInput("path contains non-UTF8 component".to_owned())
                })?;
                // 每段走 name 同款字符集校验
                validate_name(segment)?;
                has_component = true;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(
                    NodegetError::InvalidInput("path cannot contain '..'".to_owned()).into(),
                );
            }
            Component::RootDir => {
                return Err(NodegetError::InvalidInput(
                    "path cannot be absolute (leading '/')".to_owned(),
                )
                .into());
            }
            Component::Prefix(_) => {
                return Err(NodegetError::InvalidInput(
                    "path cannot contain drive prefix".to_owned(),
                )
                .into());
            }
        }
    }
    if !has_component {
        return Err(NodegetError::InvalidInput("path has no valid component".to_owned()).into());
    }
    Ok(())
}

/// 解析并校验文件路径，防止目录遍历攻击
///
/// 参数语义：
/// - `static_path`: 配置文件中的 `static_path`（总根）
/// - `sub_path`: 某条 static 记录里的 `path` 字段（相对 `static_path` 的子目录）
/// - `file_path`: 相对 `{static_path}/{sub_path}/` 的文件路径
///
/// 返回以 `{static_path}/{sub_path}/` 为基础、拼接 `file_path` 后的安全路径。
///
/// 调用方必须保证 `sub_path` 已通过 [`validate_sub_path`] 校验。
pub fn resolve_safe_file_path(
    static_path: &str,
    sub_path: &str,
    file_path: &str,
) -> anyhow::Result<PathBuf> {
    // 防御性：再次校验 sub_path，避免调用方忘记
    validate_sub_path(sub_path)?;

    let base = Path::new(static_path).join(sub_path);
    let mut resolved = base.clone();

    let path = Path::new(file_path);
    for component in path.components() {
        match component {
            Component::Normal(c) => resolved.push(c),
            Component::RootDir | Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() {
                    return Err(NodegetError::InvalidInput(
                        "Invalid path: path traversal detected".to_owned(),
                    )
                    .into());
                }
            }
            Component::Prefix(_) => {
                return Err(NodegetError::InvalidInput(
                    "Invalid path: absolute path not allowed".to_owned(),
                )
                .into());
            }
        }
    }

    // 双重校验：resolved 必须在 base 目录树内
    if !resolved.starts_with(&base) {
        return Err(
            NodegetError::InvalidInput("Invalid path: path traversal detected".to_owned()).into(),
        );
    }

    Ok(resolved)
}

fn get_db() -> anyhow::Result<&'static sea_orm::DatabaseConnection> {
    DB.get().context("DB not initialized")
}

pub async fn create_static(
    name: String,
    path: String,
    is_http_root: bool,
    cors: bool,
) -> anyhow::Result<static_entity::Model> {
    let db = get_db()?;
    let name_trimmed = name.trim().to_owned();
    validate_name(&name_trimmed)?;

    let path_trimmed = path.trim().to_owned();
    validate_sub_path(&path_trimmed)?;

    // 检查是否已存在同名 static
    let existing = static_entity::Entity::find()
        .filter(static_entity::Column::Name.eq(&name_trimmed))
        .one(db)
        .await?;
    if existing.is_some() {
        return Err(
            NodegetError::DatabaseError(format!("Static '{name_trimmed}' already exists")).into(),
        );
    }

    // is_http_root 只能同时存在一个
    if is_http_root {
        let has_root = static_entity::Entity::find()
            .filter(static_entity::Column::IsHttpRoot.eq(true))
            .one(db)
            .await?;
        if has_root.is_some() {
            return Err(NodegetError::InvalidInput(
                "Another static already has is_http_root enabled".to_owned(),
            )
            .into());
        }
    }

    let active_model = static_entity::ActiveModel {
        name: Set(name_trimmed.clone()),
        path: Set(path_trimmed.clone()),
        is_http_root: Set(is_http_root),
        cors: Set(cors),
        ..Default::default()
    };

    let model = active_model.insert(db).await.map_err(|e| {
        error!(target: "static", name = %name_trimmed, error = %e, "failed to insert static");
        NodegetError::DatabaseError(format!("Failed to create static: {e}"))
    })?;

    // 创建实际磁盘目录：{static_path}/{path}
    let static_path = get_static_path();
    let dir = Path::new(&static_path).join(&path_trimmed);
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        warn!(target: "static", dir = %dir.display(), error = %e, "failed to create static directory");
    }

    cache::StaticCache::reload().await?;
    debug!(target: "static", name = %name_trimmed, path = %path_trimmed, "static created");
    Ok(model)
}

pub async fn read_static(name: &str) -> anyhow::Result<Option<static_entity::Model>> {
    let cache = cache::StaticCache::global();
    let model = cache.get_by_name(name).await.map(|arc| (*arc).clone());
    debug!(target: "static", name = %name, found = model.is_some(), "read_static from cache");
    Ok(model)
}

pub async fn update_static(
    name: String,
    new_path: String,
    new_is_http_root: bool,
    new_cors: bool,
) -> anyhow::Result<static_entity::Model> {
    let db = get_db()?;
    let name_trimmed = name.trim().to_owned();
    validate_name(&name_trimmed)?;

    let new_path_trimmed = new_path.trim().to_owned();
    validate_sub_path(&new_path_trimmed)?;

    let model = static_entity::Entity::find()
        .filter(static_entity::Column::Name.eq(&name_trimmed))
        .one(db)
        .await?
        .ok_or_else(|| NodegetError::NotFound(format!("Static '{name_trimmed}' not found")))?;

    // is_http_root 只能同时存在一个
    if new_is_http_root && !model.is_http_root {
        let has_root = static_entity::Entity::find()
            .filter(static_entity::Column::IsHttpRoot.eq(true))
            .filter(static_entity::Column::Id.ne(model.id))
            .one(db)
            .await?;
        if has_root.is_some() {
            return Err(NodegetError::InvalidInput(
                "Another static already has is_http_root enabled".to_owned(),
            )
            .into());
        }
    }

    let mut active_model: static_entity::ActiveModel = model.into();
    active_model.path = Set(new_path_trimmed.clone());
    active_model.is_http_root = Set(new_is_http_root);
    active_model.cors = Set(new_cors);

    let updated = active_model.update(db).await.map_err(|e| {
        error!(target: "static", name = %name_trimmed, error = %e, "failed to update static");
        NodegetError::DatabaseError(format!("Failed to update static: {e}"))
    })?;

    // 如新 path 对应目录尚不存在则创建；不迁移旧目录的内容
    let static_path = get_static_path();
    let dir = Path::new(&static_path).join(&new_path_trimmed);
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        warn!(target: "static", dir = %dir.display(), error = %e, "failed to create static directory");
    }

    cache::StaticCache::reload().await?;
    debug!(target: "static", name = %name_trimmed, path = %new_path_trimmed, "static updated");
    Ok(updated)
}

pub async fn delete_static(name: &str) -> anyhow::Result<()> {
    let db = get_db()?;
    let name_trimmed = name.trim();
    validate_name(name_trimmed)?;

    let model = static_entity::Entity::find()
        .filter(static_entity::Column::Name.eq(name_trimmed))
        .one(db)
        .await?
        .ok_or_else(|| NodegetError::NotFound(format!("Static '{name_trimmed}' not found")))?;

    static_entity::Entity::delete_by_id(model.id)
        .exec(db)
        .await?;

    cache::StaticCache::reload().await?;
    debug!(target: "static", name = %name_trimmed, "static deleted");
    Ok(())
}

pub async fn upload_file(
    name: &str,
    file_path: &str,
    body: Option<Vec<u8>>,
    base64_str: Option<String>,
) -> anyhow::Result<()> {
    if body.is_some() && base64_str.is_some() {
        return Err(
            NodegetError::InvalidInput("Cannot provide both body and base64".to_owned()).into(),
        );
    }
    if body.is_none() && base64_str.is_none() {
        return Err(
            NodegetError::InvalidInput("Must provide either body or base64".to_owned()).into(),
        );
    }

    validate_name(name)?;
    // 必须先存在对应的 static 配置，并拿到它的 path 字段
    let model = cache::StaticCache::global()
        .get_by_name(name)
        .await
        .ok_or_else(|| NodegetError::NotFound(format!("Static '{name}' not found")))?;

    let data = if let Some(b) = body {
        b
    } else {
        let b64 = base64_str.unwrap();
        base64::engine::general_purpose::STANDARD
            .decode(&b64)
            .map_err(|e| NodegetError::InvalidInput(format!("Invalid base64: {e}")))?
    };

    let static_path = get_static_path();
    let resolved = resolve_safe_file_path(&static_path, &model.path, file_path)?;

    if let Some(parent) = resolved.parent()
        && let Err(e) = tokio::fs::create_dir_all(parent).await
    {
        warn!(target: "static", path = %parent.display(), error = %e, "failed to create parent directory");
    }

    tokio::fs::write(&resolved, data).await.map_err(|e| {
        error!(target: "static", path = %resolved.display(), error = %e, "failed to write file");
        NodegetError::IoError(format!("Failed to write file: {e}"))
    })?;

    debug!(target: "static", name = %name, sub_path = %model.path, file = %file_path, "file uploaded");
    Ok(())
}

pub async fn read_file(name: &str, file_path: &str) -> anyhow::Result<String> {
    validate_name(name)?;
    let model = cache::StaticCache::global()
        .get_by_name(name)
        .await
        .ok_or_else(|| NodegetError::NotFound(format!("Static '{name}' not found")))?;

    let static_path = get_static_path();
    let resolved = resolve_safe_file_path(&static_path, &model.path, file_path)?;

    let data = tokio::fs::read(&resolved).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            NodegetError::NotFound(format!("File not found: {file_path}"))
        } else {
            NodegetError::IoError(format!("Failed to read file: {e}"))
        }
    })?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(&data);
    debug!(target: "static", name = %name, sub_path = %model.path, file = %file_path, size = data.len(), "file read");
    Ok(encoded)
}

pub async fn delete_file(name: &str, file_path: &str) -> anyhow::Result<()> {
    validate_name(name)?;
    let model = cache::StaticCache::global()
        .get_by_name(name)
        .await
        .ok_or_else(|| NodegetError::NotFound(format!("Static '{name}' not found")))?;

    let static_path = get_static_path();
    let resolved = resolve_safe_file_path(&static_path, &model.path, file_path)?;

    match tokio::fs::remove_file(&resolved).await {
        Ok(()) => {
            debug!(target: "static", name = %name, sub_path = %model.path, file = %file_path, "file deleted");
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            debug!(target: "static", name = %name, sub_path = %model.path, file = %file_path, "file not found, ignoring");
            Ok(())
        }
        Err(e) => {
            error!(target: "static", name = %name, sub_path = %model.path, file = %file_path, error = %e, "failed to delete file");
            Err(NodegetError::IoError(format!("Failed to delete file: {e}")).into())
        }
    }
}
