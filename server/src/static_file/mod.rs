use anyhow::Context;
use base64::Engine as _;
use nodeget_lib::error::NodegetError;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use std::collections::VecDeque;
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

/// 描述 static 目录下单个文件的元信息。
///
/// 为 `static_list_file` RPC 的返回元素。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    /// 相对 `{static_path}/{sub_path}/` 的路径，`/` 分隔。
    pub path: String,
    /// 文件大小，字节。
    pub size: u64,
    /// 最后修改时间：Unix 毫秒时间戳；若无法获取则为 `0`。
    pub mtime: i64,
}

/// 递归列出某个 static 记录目录下所有文件的相对路径、体积和修改时间
///
/// 返回的 [`FileInfo::path`] 以 `/` 作为分隔符（跨平台一致），相对于 `{static_path}/{sub_path}/`。
/// 例如 `[{path:"index.html",size:123,mtime:1715000000000}, {path:"docs/1.md",...}]`（`mtime` 为毫秒）。
///
/// 行为：
/// - 如果磁盘目录不存在，视为空目录返回 `vec![]`，而非报错（static 记录刚建但没上传文件是正常态）。
/// - 不跟随符号链接（防止 symlink 逃逸 static 目录）。
/// - 只列出普通文件，跳过目录、符号链接、socket 等。
/// - 结果按 `path` 字典序排序，保证稳定输出。
pub async fn list_file(name: &str) -> anyhow::Result<Vec<FileInfo>> {
    validate_name(name)?;
    let model = cache::StaticCache::global()
        .get_by_name(name)
        .await
        .ok_or_else(|| NodegetError::NotFound(format!("Static '{name}' not found")))?;

    let static_path = get_static_path();
    let base = Path::new(&static_path).join(&model.path);

    let files = tokio::task::spawn_blocking(move || collect_files(&base))
        .await
        .map_err(|e| NodegetError::Other(format!("Failed to join file listing task: {e}")))??;

    debug!(target: "static", name = %name, sub_path = %model.path, count = files.len(), "file list produced");
    Ok(files)
}

/// 将一个文件从 `from` 路径移动/重命名为 `to`，两者均相对当前 static 的磁盘子目录。
///
/// 行为：
/// - 源文件不存在 → 返回 [`NodegetError::NotFound`]。
/// - 目标已存在（包括符号链接、目录）→ 返回 [`NodegetError::InvalidInput`]，不覆盖。
///   在 Linux / macOS 上使用 `renameat2(RENAME_NOREPLACE)` / `renamex_np(RENAME_EXCL)`
///   做原子检查，避免 TOCTOU 竞争。在 Windows 与其他 Unix 上退化为非原子的
///   "stat 然后 rename"，存在极小的竞争窗口。
/// - 自动为目标创建缺失的父目录。
/// - 源与目标指向同一路径 → 视作 no-op，返回 Ok。
/// - 跨 static 移动不支持：`from` 与 `to` 都在同一 static 的磁盘根下。
pub async fn rename_file(name: &str, from: &str, to: &str) -> anyhow::Result<()> {
    validate_name(name)?;
    let model = cache::StaticCache::global()
        .get_by_name(name)
        .await
        .ok_or_else(|| NodegetError::NotFound(format!("Static '{name}' not found")))?;

    let static_path = get_static_path();
    let from_resolved = resolve_safe_file_path(&static_path, &model.path, from)?;
    let to_resolved = resolve_safe_file_path(&static_path, &model.path, to)?;

    // 源与目标相同 → no-op
    if from_resolved == to_resolved {
        debug!(target: "static", name = %name, sub_path = %model.path, from = %from, to = %to, "rename: source == destination, no-op");
        return Ok(());
    }

    // 确保目标父目录存在
    if let Some(parent) = to_resolved.parent()
        && let Err(e) = tokio::fs::create_dir_all(parent).await
    {
        warn!(target: "static", path = %parent.display(), error = %e, "failed to create parent directory for rename");
    }

    // 在 spawn_blocking 中执行原子 rename（或 fallback）
    let from_display = from.to_owned();
    let to_display = to.to_owned();

    tokio::task::spawn_blocking(move || {
        atomic_rename_no_replace(&from_resolved, &to_resolved, &from_display, &to_display)
    })
    .await
    .map_err(|e| NodegetError::Other(format!("Failed to join rename task: {e}")))??;

    debug!(target: "static", name = %name, sub_path = %model.path, from = %from, to = %to, "file renamed");
    Ok(())
}

/// 原子地 rename 文件，目标已存在时返回错误（不覆盖）。
///
/// - Linux / Android：`renameat2(AT_FDCWD, from, AT_FDCWD, to, RENAME_NOREPLACE)`。
///   内核 < 3.15 或文件系统不支持时（ENOSYS / EINVAL），自动回退到非原子 `stat + rename`。
/// - macOS / iOS：`renamex_np(from, to, RENAME_EXCL)`。
/// - 其他 Unix 与 Windows：fallback 到 `symlink_metadata(to)` 检查 + `std::fs::rename`，
///   在理论上存在 TOCTOU 窗口，但无原生原子接口可用。
fn atomic_rename_no_replace(
    from: &Path,
    to: &Path,
    from_display: &str,
    to_display: &str,
) -> anyhow::Result<()> {
    // Case-insensitive 文件系统上（macOS APFS 默认、Windows NTFS）
    // `from` 与 `to` 仅大小写不同时会指向同一 inode，
    // RENAME_NOREPLACE / RENAME_EXCL 会因为"目标存在"失败。
    //
    // 仅在"同目录 + 文件名大小写不敏感相等"时降级为普通 rename，避免
    // 硬链接（不同路径指向同 inode）被误判为同名 rename 而静默删除 `from` 的名字。
    #[cfg(unix)]
    if is_case_only_rename(from, to) {
        return std::fs::rename(from, to)
            .map_err(|e| map_rename_error(&e, from_display, to_display));
    }

    // 构造 NUL 结尾 C 字符串（Unix 系统调用需要）
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    ))]
    let (from_c, to_c) = {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let from_c = CString::new(from.as_os_str().as_bytes()).map_err(|e| {
            NodegetError::InvalidInput(format!("Invalid source path (contains NUL byte): {e}"))
        })?;
        let to_c = CString::new(to.as_os_str().as_bytes()).map_err(|e| {
            NodegetError::InvalidInput(format!("Invalid destination path (contains NUL byte): {e}"))
        })?;
        (from_c, to_c)
    };

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // SAFETY: from_c / to_c 都是有效的 NUL 结尾 C 字符串（CString 保证）；
        // AT_FDCWD 让内核以当前工作目录解析相对路径（此处传入的是绝对路径，不受影响）；
        // RENAME_NOREPLACE 使目标已存在时返回 EEXIST 而非覆盖；
        // 返回值 -1 表示失败，errno 中为具体错误码。
        let ret = unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                from_c.as_ptr(),
                libc::AT_FDCWD,
                to_c.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if ret == 0 {
            return Ok(());
        }
        let err = std::io::Error::last_os_error();
        // 老内核（< 3.15）没有 renameat2 → ENOSYS；
        // 某些文件系统（旧 NFS、某些 FUSE 实现）不支持 RENAME_NOREPLACE → EINVAL。
        // 这两种情况下回退到非原子 stat + rename，虽然失去原子性但保证可用。
        let raw = err.raw_os_error();
        if raw == Some(libc::ENOSYS) || raw == Some(libc::EINVAL) {
            return non_atomic_check_and_rename(from, to, from_display, to_display);
        }
        Err(map_rename_error(&err, from_display, to_display))
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        // SAFETY: from_c / to_c 都是有效的 NUL 结尾 C 字符串；
        // RENAME_EXCL 使目标已存在时返回 EEXIST 而非覆盖。
        let ret = unsafe { libc::renamex_np(from_c.as_ptr(), to_c.as_ptr(), libc::RENAME_EXCL) };
        if ret == 0 {
            return Ok(());
        }
        let err = std::io::Error::last_os_error();
        // 某些文件系统可能不支持 RENAME_EXCL → ENOTSUP / EINVAL，回退到 stat + rename
        let raw = err.raw_os_error();
        if raw == Some(libc::ENOTSUP) || raw == Some(libc::EINVAL) {
            return non_atomic_check_and_rename(from, to, from_display, to_display);
        }
        Err(map_rename_error(&err, from_display, to_display))
    }

    // 其他平台（Windows / *BSD 等）：fallback 到非原子 check + rename
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    )))]
    {
        non_atomic_check_and_rename(from, to, from_display, to_display)
    }
}

/// 仅当 `from` 和 `to` 位于同一父目录、且文件名大小写不敏感相等、且指向同一 inode 时返回 true。
///
/// 用于识别 case-insensitive 文件系统上的"纯大小写重命名"场景。
/// 限制为"同父目录 + 大小写相同"避免硬链接等同 inode 但不同路径的情况被误判。
#[cfg(unix)]
fn is_case_only_rename(from: &Path, to: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;

    // 同父目录
    let (Some(fp), Some(tp)) = (from.parent(), to.parent()) else {
        return false;
    };
    if fp != tp {
        return false;
    }

    // 文件名大小写不敏感相等
    let (Some(fname), Some(tname)) = (from.file_name(), to.file_name()) else {
        return false;
    };
    let (Some(fs), Some(ts)) = (fname.to_str(), tname.to_str()) else {
        return false;
    };
    if !fs.eq_ignore_ascii_case(ts) {
        return false;
    }

    // 同 inode（证明底层是同一个文件，也即 case-insensitive FS 视为同名）
    let (Ok(fm), Ok(tm)) = (
        std::fs::symlink_metadata(from),
        std::fs::symlink_metadata(to),
    ) else {
        return false;
    };
    fm.dev() == tm.dev() && fm.ino() == tm.ino()
}

/// 非原子 fallback：先 stat 目标存在则失败，否则 rename。
fn non_atomic_check_and_rename(
    from: &Path,
    to: &Path,
    from_display: &str,
    to_display: &str,
) -> anyhow::Result<()> {
    if std::fs::symlink_metadata(to).is_ok() {
        return Err(NodegetError::InvalidInput(format!(
            "Destination already exists: {to_display}"
        ))
        .into());
    }
    std::fs::rename(from, to).map_err(|e| map_rename_error(&e, from_display, to_display))
}

/// 将 IO 错误转换为统一的业务错误
fn map_rename_error(err: &std::io::Error, from_display: &str, to_display: &str) -> anyhow::Error {
    match err.kind() {
        std::io::ErrorKind::NotFound => {
            NodegetError::NotFound(format!("Source file not found: {from_display}")).into()
        }
        std::io::ErrorKind::AlreadyExists => {
            NodegetError::InvalidInput(format!("Destination already exists: {to_display}")).into()
        }
        _ => {
            // 某些 libc 实现将 EEXIST 映射为 ErrorKind::Other 而非 AlreadyExists
            #[cfg(unix)]
            if err.raw_os_error() == Some(libc::EEXIST) {
                return NodegetError::InvalidInput(format!(
                    "Destination already exists: {to_display}"
                ))
                .into();
            }
            error!(target: "static", from = %from_display, to = %to_display, error = %err, "failed to rename file");
            NodegetError::IoError(format!("Failed to rename file: {err}")).into()
        }
    }
}

/// 列出缓存中所有静态服务配置的 `name` 字段，结果按字典序排序。
///
/// 数据源是 [`cache::StaticCache`]，不访问数据库、不涉及磁盘 I/O。
pub async fn list_all_names() -> Vec<String> {
    let mut names: Vec<String> = cache::StaticCache::global()
        .get_all()
        .await
        .iter()
        .map(|m| m.name.clone())
        .collect();
    names.sort();
    debug!(target: "static", count = names.len(), "static name list produced");
    names
}

/// 同步递归收集 `base` 下所有普通文件，返回 [`FileInfo`] 列表。
///
/// 使用显式栈而非递归调用，避免极深目录栈溢出。
fn collect_files(base: &Path) -> anyhow::Result<Vec<FileInfo>> {
    // 目录不存在或不是目录 → 返回空列表（对应 static 记录创建后还没上传文件的情况）
    match std::fs::metadata(base) {
        Ok(m) if m.is_dir() => {}
        Ok(_) => return Ok(Vec::new()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(NodegetError::IoError(format!("Failed to stat static dir: {e}")).into());
        }
    }

    let mut out: Vec<FileInfo> = Vec::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(base.to_path_buf());

    while let Some(dir) = queue.pop_front() {
        let read = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(NodegetError::IoError(format!(
                    "Failed to read dir {}: {e}",
                    dir.display()
                ))
                .into());
            }
        };

        for entry in read {
            let entry = entry.map_err(|e| {
                NodegetError::IoError(format!(
                    "Failed to read dir entry in {}: {e}",
                    dir.display()
                ))
            })?;

            // 使用 symlink_metadata 以识别符号链接本身，不跟随
            let meta = match entry.path().symlink_metadata() {
                Ok(m) => m,
                Err(e) => {
                    warn!(target: "static", path = %entry.path().display(), error = %e, "skip entry: cannot stat");
                    continue;
                }
            };
            let ft = meta.file_type();

            if ft.is_symlink() {
                // 不跟随符号链接，避免逃逸根目录
                continue;
            }

            let path = entry.path();
            if ft.is_dir() {
                queue.push_back(path);
            } else if ft.is_file() {
                // 构造相对路径，使用 '/' 分隔符；遇到非 UTF-8 段则跳过整个文件
                if let Ok(rel) = path.strip_prefix(base) {
                    let mut parts: Vec<&str> = Vec::new();
                    let mut ok = true;
                    for c in rel.components() {
                        if let Component::Normal(s) = c {
                            if let Some(s) = s.to_str() {
                                parts.push(s);
                            } else {
                                ok = false;
                                break;
                            }
                        } else {
                            // 不预期出现非 Normal 组件（来自 walk 结果），保险起见跳过
                            ok = false;
                            break;
                        }
                    }
                    if ok && !parts.is_empty() {
                        // mtime 不可用（某些文件系统不支持）时置 0，不算致命错误
                        let mtime = meta
                            .modified()
                            .ok()
                            .and_then(|t| {
                                t.duration_since(std::time::UNIX_EPOCH)
                                    .ok()
                                    .and_then(|d| i64::try_from(d.as_millis()).ok())
                            })
                            .unwrap_or(0);
                        out.push(FileInfo {
                            path: parts.join("/"),
                            size: meta.len(),
                            mtime,
                        });
                    } else if !ok {
                        warn!(target: "static", path = %path.display(), "skip file: non-UTF-8 path component");
                    }
                }
            }
            // 其他类型（socket、fifo 等）跳过
        }
    }

    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// 生成一个进程内唯一的临时目录路径（不依赖外部 crate）
    fn unique_tempdir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let p = std::env::temp_dir().join(format!(
            "nodeget-static-test-{}-{n}-{ts}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).expect("create tempdir");
        p
    }

    fn write_file(base: &Path, rel: &str, content: &[u8]) {
        let p = base.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn collect_files_missing_dir_returns_empty() {
        let base = std::env::temp_dir().join("nodeget-static-test-does-not-exist-xyz");
        let files = collect_files(&base).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn collect_files_empty_dir_returns_empty() {
        let base = unique_tempdir();
        let files = collect_files(&base).unwrap();
        assert!(files.is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn collect_files_flat_and_nested() {
        let base = unique_tempdir();
        write_file(&base, "index.html", b"<html/>");
        write_file(&base, "docs/1.md", b"# 1");
        write_file(&base, "docs/sub/2.md", b"# 2");
        write_file(&base, "assets/logo.png", b"\x89PNG");

        let files = collect_files(&base).unwrap();
        // 字典序 + 体积
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "assets/logo.png",
                "docs/1.md",
                "docs/sub/2.md",
                "index.html",
            ]
        );
        let sizes: Vec<u64> = files.iter().map(|f| f.size).collect();
        assert_eq!(sizes, vec![4, 3, 3, 7]);
        // mtime：任何合理的文件系统都应返回真实时间戳。
        // 若所有 mtime 都是 0，说明元数据读取或毫秒转换路径全部走了 fallback，
        // 属于实现回归，这里强校验。
        assert!(
            files.iter().any(|f| f.mtime > 0),
            "expected at least one file to have a real mtime, got: {:?}",
            files.iter().map(|f| f.mtime).collect::<Vec<_>>()
        );
        // 非负（i64 永远如此，但作为防御性校验保留）
        assert!(files.iter().all(|f| f.mtime >= 0));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn collect_files_skips_directories_without_files() {
        let base = unique_tempdir();
        std::fs::create_dir_all(base.join("empty_dir/nested")).unwrap();
        write_file(&base, "a.txt", b"a");

        let files = collect_files(&base).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "a.txt");
        assert_eq!(files[0].size, 1);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[cfg(unix)]
    #[test]
    fn collect_files_does_not_follow_symlinks() {
        use std::os::unix::fs::symlink;

        let base = unique_tempdir();
        let outside = unique_tempdir();
        write_file(&outside, "secret.txt", b"secret");
        write_file(&base, "real.txt", b"real");

        // 在 base 下创建指向 outside 的符号链接
        let link = base.join("link-to-outside");
        symlink(&outside, &link).unwrap();

        let files = collect_files(&base).unwrap();
        // 不应跟随 symlink 进入 outside，也不应把 link 本身列为文件
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "real.txt");
        assert_eq!(files[0].size, 4);

        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(&outside);
    }

    /// 从 `anyhow::Error` 中提取 `NodegetError`，便于断言错误种类
    fn extract_nodeget_error(e: &anyhow::Error) -> &NodegetError {
        e.downcast_ref::<NodegetError>()
            .expect("error should be a NodegetError")
    }

    #[test]
    fn atomic_rename_basic_success() {
        let base = unique_tempdir();
        write_file(&base, "a.txt", b"hello");
        let from = base.join("a.txt");
        let to = base.join("b.txt");

        atomic_rename_no_replace(&from, &to, "a.txt", "b.txt").unwrap();
        assert!(!from.exists());
        assert!(to.exists());
        assert_eq!(std::fs::read(&to).unwrap(), b"hello");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn atomic_rename_source_not_found() {
        let base = unique_tempdir();
        let from = base.join("missing.txt");
        let to = base.join("b.txt");

        let err = atomic_rename_no_replace(&from, &to, "missing.txt", "b.txt").unwrap_err();
        match extract_nodeget_error(&err) {
            NodegetError::NotFound(_) => {}
            other => panic!("expected NotFound, got: {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn atomic_rename_destination_exists_fails() {
        let base = unique_tempdir();
        write_file(&base, "a.txt", b"aaa");
        write_file(&base, "b.txt", b"bbb");
        let from = base.join("a.txt");
        let to = base.join("b.txt");

        let err = atomic_rename_no_replace(&from, &to, "a.txt", "b.txt").unwrap_err();
        match extract_nodeget_error(&err) {
            NodegetError::InvalidInput(msg) => {
                assert!(msg.contains("already exists"), "got msg: {msg}");
            }
            other => panic!("expected InvalidInput, got: {other:?}"),
        }
        // 两个文件都应完好无损
        assert_eq!(std::fs::read(&from).unwrap(), b"aaa");
        assert_eq!(std::fs::read(&to).unwrap(), b"bbb");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn atomic_rename_dest_is_directory_fails() {
        let base = unique_tempdir();
        write_file(&base, "a.txt", b"aaa");
        std::fs::create_dir_all(base.join("subdir")).unwrap();
        let from = base.join("a.txt");
        let to = base.join("subdir");

        let err = atomic_rename_no_replace(&from, &to, "a.txt", "subdir").unwrap_err();
        // 目录已存在应被视为"目标已存在"，不能静默替换
        match extract_nodeget_error(&err) {
            NodegetError::InvalidInput(_) | NodegetError::IoError(_) => {}
            other => panic!("expected InvalidInput or IoError, got: {other:?}"),
        }
        // 源文件与目录都应完好无损
        assert!(from.exists());
        assert!(to.is_dir());

        let _ = std::fs::remove_dir_all(&base);
    }

    /// 硬链接：两个不同名字指向同 inode。此场景下绝不能误触发 case-rename fast-path
    /// 而静默删除 `from` 的路径。
    #[cfg(unix)]
    #[test]
    fn atomic_rename_hardlink_not_treated_as_case_rename() {
        let base = unique_tempdir();
        write_file(&base, "original.txt", b"content");
        let from = base.join("original.txt");
        let to = base.join("hardlink.txt");
        std::fs::hard_link(&from, &to).unwrap();
        // 此时 from 与 to 是两个不同的文件名，指向同一 inode

        let err = atomic_rename_no_replace(&from, &to, "original.txt", "hardlink.txt").unwrap_err();
        match extract_nodeget_error(&err) {
            NodegetError::InvalidInput(_) => {}
            other => panic!("expected InvalidInput (dest exists), got: {other:?}"),
        }
        // 两个名字都应继续存在
        assert!(from.exists(), "source hardlink should not be removed");
        assert!(to.exists(), "destination hardlink should not be removed");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// `is_case_only_rename` 只有在"同父目录 + 文件名大小写不敏感相等 + 同 inode"
    /// 三条件全满足时才返回 true。这里测试各种不应返回 true 的情况。
    #[cfg(unix)]
    #[test]
    fn is_case_only_rename_rejects_different_parents() {
        let base = unique_tempdir();
        std::fs::create_dir_all(base.join("dir1")).unwrap();
        std::fs::create_dir_all(base.join("dir2")).unwrap();
        write_file(&base, "dir1/a.txt", b"x");
        write_file(&base, "dir2/A.txt", b"y");

        let from = base.join("dir1/a.txt");
        let to = base.join("dir2/A.txt");
        // 不同父目录 → 即便文件名大小写不敏感相等，也不应触发 fast-path
        assert!(!is_case_only_rename(&from, &to));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[cfg(unix)]
    #[test]
    fn is_case_only_rename_rejects_different_names() {
        let base = unique_tempdir();
        write_file(&base, "a.txt", b"x");
        write_file(&base, "b.txt", b"y");

        let from = base.join("a.txt");
        let to = base.join("b.txt");
        // 文件名完全不同
        assert!(!is_case_only_rename(&from, &to));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[cfg(unix)]
    #[test]
    fn is_case_only_rename_rejects_hardlinks_with_different_names() {
        let base = unique_tempdir();
        write_file(&base, "original.txt", b"x");
        let from = base.join("original.txt");
        let to = base.join("hardlink.txt");
        std::fs::hard_link(&from, &to).unwrap();
        // 同 inode 但文件名完全不同（不是大小写差异）
        assert!(!is_case_only_rename(&from, &to));

        let _ = std::fs::remove_dir_all(&base);
    }
}
