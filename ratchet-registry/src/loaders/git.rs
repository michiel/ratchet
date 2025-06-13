#[cfg(feature = "git")]
use async_trait::async_trait;
#[cfg(feature = "git")]
use std::collections::HashMap;
#[cfg(feature = "git")]
use std::path::{Path, PathBuf};
#[cfg(feature = "git")]
use std::sync::Arc;
#[cfg(feature = "git")]
use tokio::fs;
#[cfg(feature = "git")]
use tracing::{info, warn};
#[cfg(feature = "git")]
use chrono::Utc;
#[cfg(feature = "git")]
use uuid::Uuid;

#[cfg(feature = "git")]
use crate::config::{TaskSource, GitConfig, GitAuth, GitAuthType};
#[cfg(feature = "git")]
use crate::error::{RegistryError, Result};
#[cfg(feature = "git")]
use crate::loaders::TaskLoader;
#[cfg(feature = "git")]
use crate::types::{DiscoveredTask, TaskDefinition, TaskMetadata, TaskReference};

#[cfg(feature = "git")]
pub struct GitLoader {
    git_client: Arc<GitClient>,
    cache: Arc<GitRepositoryCache>,
    auth_manager: Arc<GitAuthManager>,
}

#[cfg(feature = "git")]
impl Default for GitLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "git")]
impl GitLoader {
    pub fn new() -> Self {
        Self {
            git_client: Arc::new(GitClient::new()),
            cache: Arc::new(GitRepositoryCache::new()),
            auth_manager: Arc::new(GitAuthManager::new()),
        }
    }

    pub fn with_cache_path(cache_path: PathBuf) -> Self {
        Self {
            git_client: Arc::new(GitClient::new()),
            cache: Arc::new(GitRepositoryCache::with_path(cache_path)),
            auth_manager: Arc::new(GitAuthManager::new()),
        }
    }

    async fn get_repository_path(&self, source: &TaskSource) -> Result<PathBuf> {
        if let Some(url) = source.git_url() {
            self.cache.get_repository_path(url).await
        } else {
            Err(RegistryError::Configuration(
                "Source is not a Git repository".to_string(),
            ))
        }
    }

    async fn ensure_repository_synced(&self, source: &TaskSource) -> Result<PathBuf> {
        let url = source.git_url().ok_or_else(|| {
            RegistryError::Configuration("Source is not a Git repository".to_string())
        })?;
        
        let config = source.git_config().unwrap();
        let auth = source.git_auth();
        
        let repo_path = self.cache.get_repository_path(url).await?;
        
        if !repo_path.exists() {
            // Clone repository
            info!("Cloning Git repository: {}", url);
            self.git_client.clone_repository(url, &repo_path, config, auth).await?;
        } else {
            // Check if we need to sync
            if self.cache.should_sync(&repo_path, &config.cache_ttl).await? {
                info!("Syncing Git repository: {}", url);
                self.git_client.sync_repository(&repo_path, config, auth).await?;
            }
        }
        
        Ok(repo_path)
    }

    async fn scan_tasks_directory(&self, repo_path: &Path, subdir: Option<&str>) -> Result<Vec<DiscoveredTask>> {
        let scan_path = if let Some(subdir) = subdir {
            repo_path.join(subdir)
        } else {
            repo_path.to_path_buf()
        };

        let tasks_dir = scan_path.join("tasks");
        if !tasks_dir.exists() {
            warn!("No tasks directory found in repository: {:?}", tasks_dir);
            return Ok(Vec::new());
        }

        self.discover_tasks_in_directory(&tasks_dir).await
    }

    fn discover_tasks_in_directory<'a>(&'a self, dir: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<DiscoveredTask>>> + Send + 'a>> {
        Box::pin(async move {
        let mut discovered = Vec::new();
        let mut entries = fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            let metadata = entry.metadata().await?;

            if metadata.is_dir() {
                let metadata_file = entry_path.join("metadata.json");
                if metadata_file.exists() {
                    // This is a task directory
                    match self.load_task_metadata(&entry_path).await {
                        Ok(task_metadata) => {
                            let task_ref = TaskReference {
                                name: task_metadata.name.clone(),
                                version: task_metadata.version.clone(),
                                source: format!("git://{}", entry_path.display()),
                            };

                            discovered.push(DiscoveredTask {
                                task_ref,
                                metadata: task_metadata,
                                discovered_at: Utc::now(),
                            });
                        }
                        Err(e) => {
                            warn!("Failed to load task metadata from {:?}: {}", entry_path, e);
                        }
                    }
                } else {
                    // Recursively scan subdirectories
                    match self.discover_tasks_in_directory(&entry_path).await {
                        Ok(mut subdiscovered) => {
                            discovered.append(&mut subdiscovered);
                        }
                        Err(e) => {
                            warn!("Failed to scan subdirectory {:?}: {}", entry_path, e);
                        }
                    }
                }
            }
        }

        Ok(discovered)
        })
    }

    async fn load_task_metadata(&self, task_path: &Path) -> Result<TaskMetadata> {
        let metadata_path = task_path.join("metadata.json");
        let metadata_content = fs::read_to_string(metadata_path).await?;
        let metadata: serde_json::Value = serde_json::from_str(&metadata_content)?;

        // Extract basic metadata fields
        let name = metadata["name"]
            .as_str()
            .ok_or_else(|| RegistryError::ValidationError("Missing 'name' in metadata".to_string()))?
            .to_string();

        let version = metadata["version"]
            .as_str()
            .ok_or_else(|| RegistryError::ValidationError("Missing 'version' in metadata".to_string()))?
            .to_string();

        let uuid = if let Some(uuid_str) = metadata["uuid"].as_str() {
            Uuid::parse_str(uuid_str)
                .map_err(|e| RegistryError::ValidationError(format!("Invalid UUID: {}", e)))?
        } else {
            Uuid::new_v4() // Generate if not present
        };

        let description = metadata["description"].as_str().map(|s| s.to_string());
        let tags = metadata["tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let now = Utc::now();

        Ok(TaskMetadata {
            uuid,
            name,
            version,
            description,
            tags,
            created_at: now,
            updated_at: now,
            checksum: None, // TODO: Calculate checksum from Git commit
        })
    }

    async fn load_task_definition_from_path(&self, task_path: &Path) -> Result<TaskDefinition> {
        let metadata = self.load_task_metadata(task_path).await?;

        // Load main script
        let main_js_path = task_path.join("main.js");
        let script = fs::read_to_string(main_js_path).await?;

        // Load schemas (optional)
        let input_schema = if task_path.join("input.schema.json").exists() {
            let schema_content = fs::read_to_string(task_path.join("input.schema.json")).await?;
            Some(serde_json::from_str(&schema_content)?)
        } else {
            None
        };

        let output_schema = if task_path.join("output.schema.json").exists() {
            let schema_content = fs::read_to_string(task_path.join("output.schema.json")).await?;
            Some(serde_json::from_str(&schema_content)?)
        } else {
            None
        };

        let task_ref = TaskReference {
            name: metadata.name.clone(),
            version: metadata.version.clone(),
            source: format!("git://{}", task_path.display()),
        };

        Ok(TaskDefinition {
            reference: task_ref,
            metadata,
            script,
            input_schema,
            output_schema,
            dependencies: Vec::new(), // TODO: Extract from metadata
            environment: HashMap::new(), // TODO: Extract from metadata
        })
    }

    async fn load_registry_index(&self, repo_path: &Path) -> Result<Option<RegistryIndex>> {
        let index_path = repo_path.join(".ratchet").join("index.json");
        if !index_path.exists() {
            return Ok(None);
        }

        let index_content = fs::read_to_string(index_path).await?;
        let index: RegistryIndex = serde_json::from_str(&index_content)?;
        Ok(Some(index))
    }
}

#[cfg(feature = "git")]
#[async_trait]
impl TaskLoader for GitLoader {
    async fn discover_tasks(&self, source: &TaskSource) -> Result<Vec<DiscoveredTask>> {
        let repo_path = self.ensure_repository_synced(source).await?;
        let config = source.git_config().unwrap();

        // Try to use registry index for fast discovery
        if let Ok(Some(index)) = self.load_registry_index(&repo_path).await {
            info!("Using registry index for fast task discovery");
            let mut discovered = Vec::new();
            
            for task_info in index.tasks {
                let task_path = repo_path.join(&task_info.path);
                if task_path.exists() {
                    let task_ref = TaskReference {
                        name: task_info.name.clone(),
                        version: task_info.version.clone(),
                        source: format!("git://{}", task_path.display()),
                    };

                    // Convert task info to metadata
                    let metadata = TaskMetadata {
                        uuid: task_info.uuid,
                        name: task_info.name,
                        version: task_info.version,
                        description: task_info.description,
                        tags: task_info.tags,
                        created_at: Utc::now(), // TODO: Use actual timestamps
                        updated_at: task_info.last_modified,
                        checksum: task_info.checksum,
                    };

                    discovered.push(DiscoveredTask {
                        task_ref,
                        metadata,
                        discovered_at: Utc::now(),
                    });
                }
            }
            
            return Ok(discovered);
        }

        // Fall back to directory scanning
        info!("Scanning repository directory for tasks");
        self.scan_tasks_directory(&repo_path, config.subdirectory.as_deref()).await
    }

    async fn load_task(&self, task_ref: &TaskReference) -> Result<TaskDefinition> {
        if !task_ref.source.starts_with("git://") {
            return Err(RegistryError::Configuration(
                "GitLoader can only load git:// sources".to_string(),
            ));
        }

        let path_str = task_ref.source.strip_prefix("git://").unwrap();
        let task_path = PathBuf::from(path_str);

        self.load_task_definition_from_path(&task_path).await
    }

    async fn supports_source(&self, source: &TaskSource) -> bool {
        matches!(source, TaskSource::Git { .. })
    }
}

// Supporting structures

#[cfg(feature = "git")]
#[derive(Debug, serde::Deserialize)]
struct RegistryIndex {
    generated_at: chrono::DateTime<Utc>,
    repository: RepositoryInfo,
    tasks: Vec<TaskInfo>,
    #[allow(dead_code)]
    collections: Option<Vec<CollectionInfo>>,
}

#[cfg(feature = "git")]
#[derive(Debug, serde::Deserialize)]
struct RepositoryInfo {
    name: String,
    version: String,
    commit: String,
}

#[cfg(feature = "git")]
#[derive(Debug, serde::Deserialize)]
struct TaskInfo {
    name: String,
    version: String,
    path: String,
    uuid: Uuid,
    description: Option<String>,
    tags: Vec<String>,
    last_modified: chrono::DateTime<Utc>,
    checksum: Option<String>,
}

#[cfg(feature = "git")]
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct CollectionInfo {
    name: String,
    path: String,
    description: Option<String>,
    tasks: Vec<String>,
}

// Git client implementation
#[cfg(feature = "git")]
pub struct GitClient {
    #[allow(dead_code)]
    config: GitClientConfig,
}

#[cfg(feature = "git")]
#[derive(Debug)]
pub struct GitClientConfig {
    pub default_timeout: std::time::Duration,
}

#[cfg(feature = "git")]
impl Default for GitClientConfig {
    fn default() -> Self {
        Self {
            default_timeout: std::time::Duration::from_secs(300),
        }
    }
}

#[cfg(feature = "git")]
impl GitClient {
    pub fn new() -> Self {
        Self {
            config: GitClientConfig::default(),
        }
    }

    pub async fn clone_repository(
        &self,
        url: &str,
        local_path: &Path,
        config: &GitConfig,
        auth: Option<&GitAuth>,
    ) -> Result<()> {
        use git2::{FetchOptions, RemoteCallbacks};

        // Create parent directory if it doesn't exist
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let local_path_str = local_path.to_string_lossy().to_string();
        let url = url.to_string();
        let git_ref = config.branch.clone();
        let shallow = config.shallow;
        let depth = config.depth;
        let auth_info = auth.map(|a| a.auth_type.clone());

        // Run Git operations in blocking task since git2 is synchronous
        let result = tokio::task::spawn_blocking(move || {
            let mut builder = git2::build::RepoBuilder::new();
            
            // Set up authentication if provided
            let mut callbacks = RemoteCallbacks::new();
            if let Some(auth_type) = auth_info {
                match Self::setup_auth_callbacks(&mut callbacks, &auth_type) {
                    Ok(_) => {},
                    Err(e) => return Err(git2::Error::from_str(&format!("Auth setup failed: {}", e))),
                }
            }
            
            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);
            builder.fetch_options(fetch_options);

            // Configure for shallow clone if requested
            if shallow {
                if let Some(_depth) = depth {
                    // Note: git2 doesn't directly support shallow clones
                    // We'd need to use git command line or implement manually
                    warn!("Shallow clones not fully supported with git2, performing full clone");
                }
            }

            // Set branch if not default
            if git_ref != "main" && git_ref != "master" {
                builder.branch(&git_ref);
            }

            // Perform the clone
            let _repo = builder.clone(&url, Path::new(&local_path_str))?;
            
            info!("Successfully cloned repository {} to {}", url, local_path_str);
            Ok::<(), git2::Error>(())
        })
        .await?;

        result.map_err(|e| RegistryError::GitError(format!("Clone failed: {}", e)))
    }

    pub async fn sync_repository(
        &self,
        repo_path: &Path,
        config: &GitConfig,
        auth: Option<&GitAuth>,
    ) -> Result<()> {
        use git2::{Repository, FetchOptions, RemoteCallbacks, ErrorCode};

        let repo_path_str = repo_path.to_string_lossy().to_string();
        let git_ref = config.branch.clone();
        let auth_info = auth.map(|a| a.auth_type.clone());

        let result = tokio::task::spawn_blocking(move || {
            let repo = Repository::open(&repo_path_str)?;
            
            // Set up authentication
            let mut callbacks = RemoteCallbacks::new();
            if let Some(auth_type) = auth_info {
                match Self::setup_auth_callbacks(&mut callbacks, &auth_type) {
                    Ok(_) => {},
                    Err(e) => return Err(git2::Error::from_str(&format!("Auth setup failed: {}", e))),
                }
            }

            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);

            // Fetch from origin
            let mut remote = repo.find_remote("origin")?;
            remote.fetch(&[&git_ref], Some(&mut fetch_options), None)?;

            // Check out the requested ref - try multiple strategies
            let checkout_result = Self::checkout_ref(&repo, &git_ref);
            match checkout_result {
                Ok(_) => {
                    info!("Successfully synced repository at {} (branch: {})", repo_path_str, git_ref);
                    Ok::<(), git2::Error>(())
                }
                Err(e) => {
                    // Provide detailed error message for missing branches
                    if e.code() == ErrorCode::NotFound {
                        let available_branches = Self::list_available_branches(&repo);
                        let branch_list = available_branches.join(", ");
                        let error_msg = if available_branches.is_empty() {
                            format!("Branch '{}' not found in repository. No remote branches are available.", git_ref)
                        } else {
                            format!("Branch '{}' not found in repository. Available branches: {}", git_ref, branch_list)
                        };
                        return Err(git2::Error::from_str(&error_msg));
                    }
                    Err(e)
                }
            }
        })
        .await?;

        result.map_err(|e| RegistryError::GitError(format!("Sync failed: {}", e)))
    }

    /// Try to check out the specified ref, with fallback strategies
    fn checkout_ref(repo: &git2::Repository, git_ref: &str) -> std::result::Result<(), git2::Error> {
        use git2::ErrorCode;

        // Strategy 1: Try refs/remotes/origin/{git_ref}
        let remote_refname = format!("refs/remotes/origin/{}", git_ref);
        match repo.revparse_single(&remote_refname) {
            Ok(obj) => {
                let oid = obj.id();
                repo.set_head_detached(oid)?;
                repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
                return Ok(());
            }
            Err(e) if e.code() == ErrorCode::NotFound => {
                // Continue to next strategy
            }
            Err(e) => return Err(e),
        }

        // Strategy 2: Try refs/heads/{git_ref} (local branch)
        let local_refname = format!("refs/heads/{}", git_ref);
        match repo.revparse_single(&local_refname) {
            Ok(obj) => {
                let oid = obj.id();
                repo.set_head_detached(oid)?;
                repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
                return Ok(());
            }
            Err(e) if e.code() == ErrorCode::NotFound => {
                // Continue to next strategy
            }
            Err(e) => return Err(e),
        }

        // Strategy 3: Try as tag refs/tags/{git_ref}
        let tag_refname = format!("refs/tags/{}", git_ref);
        match repo.revparse_single(&tag_refname) {
            Ok(obj) => {
                let oid = obj.id();
                repo.set_head_detached(oid)?;
                repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
                return Ok(());
            }
            Err(e) if e.code() == ErrorCode::NotFound => {
                // Continue to failure
            }
            Err(e) => return Err(e),
        }

        // All strategies failed - return NotFound error
        Err(git2::Error::from_str(&format!("Reference '{}' not found", git_ref)))
    }

    /// List available remote branches for error reporting
    fn list_available_branches(repo: &git2::Repository) -> Vec<String> {
        let mut branches = Vec::new();
        
        // Get remote branches
        if let Ok(branch_iter) = repo.branches(Some(git2::BranchType::Remote)) {
            for branch_result in branch_iter {
                if let Ok((branch, _branch_type)) = branch_result {
                    if let Some(name) = branch.name().unwrap_or(None) {
                        // Remove the "origin/" prefix for cleaner display
                        if let Some(clean_name) = name.strip_prefix("origin/") {
                            branches.push(clean_name.to_string());
                        }
                    }
                }
            }
        }
        
        // Get local branches too
        if let Ok(branch_iter) = repo.branches(Some(git2::BranchType::Local)) {
            for branch_result in branch_iter {
                if let Ok((branch, _branch_type)) = branch_result {
                    if let Some(name) = branch.name().unwrap_or(None) {
                        branches.push(name.to_string());
                    }
                }
            }
        }
        
        // Remove duplicates and sort
        branches.sort();
        branches.dedup();
        branches
    }

    fn setup_auth_callbacks(
        callbacks: &mut git2::RemoteCallbacks,
        auth_type: &GitAuthType,
    ) -> std::result::Result<(), String> {
        match auth_type {
            GitAuthType::Token { token } => {
                let token = token.clone();
                callbacks.credentials(move |_url, username_from_url, _allowed_types| {
                    git2::Cred::userpass_plaintext(
                        username_from_url.unwrap_or("git"),
                        &token,
                    )
                });
            }
            GitAuthType::Basic { username, password } => {
                let username = username.clone();
                let password = password.clone();
                callbacks.credentials(move |_url, _username_from_url, _allowed_types| {
                    git2::Cred::userpass_plaintext(&username, &password)
                });
            }
            GitAuthType::SshKey { private_key_path, passphrase } => {
                let private_key_path = private_key_path.clone();
                let passphrase = passphrase.clone();
                callbacks.credentials(move |_url, username_from_url, _allowed_types| {
                    git2::Cred::ssh_key(
                        username_from_url.unwrap_or("git"),
                        None,
                        Path::new(&private_key_path),
                        passphrase.as_deref(),
                    )
                });
            }
            GitAuthType::GitHubApp { .. } => {
                // GitHub App authentication would require additional JWT generation
                // For now, return an error
                return Err("GitHub App authentication not yet implemented".to_string());
            }
        }

        Ok(())
    }
}

// Git repository cache
#[cfg(feature = "git")]
pub struct GitRepositoryCache {
    cache_root: PathBuf,
}

#[cfg(feature = "git")]
impl GitRepositoryCache {
    pub fn new() -> Self {
        let cache_root = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("ratchet")
            .join("git-repos");
            
        Self { cache_root }
    }

    pub fn with_path(cache_root: PathBuf) -> Self {
        Self { cache_root }
    }

    pub async fn get_repository_path(&self, url: &str) -> Result<PathBuf> {
        // Create a safe directory name from the URL
        let url_hash = format!("{:x}", md5::compute(url.as_bytes()));
        let repo_name = url
            .split('/')
            .last()
            .unwrap_or("unknown")
            .replace(".git", "");
        
        let repo_dir = format!("{}_{}", repo_name, url_hash);
        let repo_path = self.cache_root.join(repo_dir);
        
        Ok(repo_path)
    }

    pub async fn should_sync(&self, repo_path: &Path, cache_ttl: &std::time::Duration) -> Result<bool> {
        if !repo_path.exists() {
            return Ok(true);
        }

        // Check if cache has expired based on last modified time
        let metadata = fs::metadata(repo_path).await?;
        if let Ok(modified) = metadata.modified() {
            let elapsed = modified.elapsed().unwrap_or(std::time::Duration::MAX);
            return Ok(elapsed > *cache_ttl);
        }

        // If we can't determine the age, assume we should sync
        Ok(true)
    }
}

// Git authentication manager
#[cfg(feature = "git")]
pub struct GitAuthManager {
    #[allow(dead_code)]
    auth_configs: HashMap<String, GitAuth>,
}

#[cfg(feature = "git")]
impl GitAuthManager {
    pub fn new() -> Self {
        Self {
            auth_configs: HashMap::new(),
        }
    }
}

// Stub implementation for when git feature is disabled
#[cfg(not(feature = "git"))]
pub struct GitLoader;

#[cfg(not(feature = "git"))]
impl GitLoader {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(feature = "git"))]
impl Default for GitLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "git"))]
#[async_trait]
impl TaskLoader for GitLoader {
    async fn discover_tasks(&self, _source: &TaskSource) -> Result<Vec<DiscoveredTask>> {
        Err(RegistryError::NotImplemented(
            "Git support is not compiled in. Enable the 'git' feature.".to_string(),
        ))
    }

    async fn load_task(&self, _task_ref: &TaskReference) -> Result<TaskDefinition> {
        Err(RegistryError::NotImplemented(
            "Git support is not compiled in. Enable the 'git' feature.".to_string(),
        ))
    }

    async fn supports_source(&self, source: &TaskSource) -> bool {
        matches!(source, TaskSource::Git { .. })
    }
}