use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NodeGetVersion {
    pub binary_type: String,
    pub cargo_version: String,

    pub git_branch: String,
    pub git_commit_sha: String,
    pub git_commit_date: String,
    pub git_commit_message: String,

    pub build_time: String,
    pub cargo_target_triple: String,

    pub rustc_channel: String,
    pub rustc_version: String,
    pub rustc_commit_date: String,
    pub rustc_commit_hash: String,
    pub rustc_llvm_version: String,
}

impl NodeGetVersion {
    #[must_use]
    pub fn get() -> Self {
        Self {
            binary_type: {
                if cfg!(feature = "for-server") {
                    "Server"
                } else if cfg!(feature = "for-agent") {
                    "Agent"
                } else {
                    "Unknown"
                }
            }
            .to_string(),
            cargo_version: env!("CARGO_PKG_VERSION").to_string(),
            git_branch: env!("VERGEN_GIT_BRANCH").to_string(),
            git_commit_sha: env!("VERGEN_GIT_SHA").to_string(),
            git_commit_date: env!("VERGEN_GIT_COMMIT_TIMESTAMP").to_string(),
            git_commit_message: env!("VERGEN_GIT_COMMIT_MESSAGE").to_string(),
            build_time: env!("VERGEN_BUILD_TIMESTAMP").to_string(),
            cargo_target_triple: env!("VERGEN_CARGO_TARGET_TRIPLE").to_string(),
            rustc_channel: env!("VERGEN_RUSTC_CHANNEL").to_string(),
            rustc_version: env!("VERGEN_RUSTC_SEMVER").to_string(),
            rustc_commit_date: env!("VERGEN_RUSTC_COMMIT_DATE").to_string(),
            rustc_commit_hash: env!("VERGEN_RUSTC_COMMIT_HASH").to_string(),
            rustc_llvm_version: env!("VERGEN_RUSTC_LLVM_VERSION").to_string(),
        }
    }
}

impl Display for NodeGetVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NodeGet {} Version: {}\nGit Branch: {}\nCommit SHA: {}\nCommit Date: {}\nCommit Message: {}\nBuild Time: {}\nTarget Triple: {}\nRustc Channel: {}\nRustc Version: {}\nRustc Commit Date: {}\nRustc Commit Hash: {}\nRustc LLVM Version: {}",
            self.binary_type,
            self.cargo_version,
            self.git_branch,
            self.git_commit_sha,
            self.git_commit_date,
            self.git_commit_message,
            self.build_time,
            self.cargo_target_triple,
            self.rustc_channel,
            self.rustc_version,
            self.rustc_commit_date,
            self.rustc_commit_hash,
            self.rustc_llvm_version
        )
    }
}
