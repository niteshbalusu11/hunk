fn load_user_settings(workspace_root: Option<&Path>) -> Result<UserSettings> {
    let mut config = StackedConfig::with_defaults();

    if let Some(home_dir) = dirs::home_dir() {
        load_config_if_exists(
            &mut config,
            ConfigSource::User,
            home_dir.join(".jjconfig.toml"),
        )?;
    }

    if let Some(config_dir) = dirs::config_dir() {
        load_config_if_exists(
            &mut config,
            ConfigSource::User,
            config_dir.join("jj").join("config.toml"),
        )?;
    }

    if let Some(root) = workspace_root {
        load_config_if_exists(
            &mut config,
            ConfigSource::Repo,
            root.join(".jj").join("repo").join("config.toml"),
        )?;
        load_config_if_exists(
            &mut config,
            ConfigSource::Workspace,
            root.join(".jj").join("config.toml"),
        )?;
        add_git_signing_fallback_config(&mut config, root)?;
    }

    UserSettings::from_config(config).context("failed to load jj settings")
}

fn add_git_signing_fallback_config(
    config: &mut StackedConfig,
    workspace_root: &Path,
) -> Result<()> {
    if has_explicit_signing_backend(config) {
        return Ok(());
    }

    let Some(git_signing) = read_git_signing_config(workspace_root) else {
        return Ok(());
    };
    let commit_gpgsign = git_signing.commit_gpgsign.unwrap_or(false);
    let git_signing_key = git_signing.signing_key.clone();

    if !commit_gpgsign && git_signing_key.is_none() {
        return Ok(());
    }

    let signing_backend = match git_signing.gpg_format.as_deref() {
        Some("ssh") => "ssh",
        Some("x509") => "gpgsm",
        _ => "gpg",
    };

    let mut fallback_layer = ConfigLayer::empty(ConfigSource::EnvBase);
    fallback_layer
        .set_value("signing.backend", signing_backend)
        .context("failed to apply Git signing backend fallback")?;
    if commit_gpgsign {
        fallback_layer
            .set_value("signing.behavior", "own")
            .context("failed to apply Git commit signing behavior fallback")?;
    }
    if let Some(signing_key) = git_signing_key {
        fallback_layer
            .set_value("signing.key", signing_key)
            .context("failed to apply Git signing key fallback")?;
    }
    if let Some(program) = git_signing.program_for_backend(signing_backend) {
        let key = match signing_backend {
            "ssh" => "signing.backends.ssh.program",
            "gpgsm" => "signing.backends.gpgsm.program",
            _ => "signing.backends.gpg.program",
        };
        fallback_layer
            .set_value(key, program)
            .context("failed to apply Git signing program fallback")?;
    }

    config.add_layer(fallback_layer);
    Ok(())
}

fn has_explicit_signing_backend(config: &StackedConfig) -> bool {
    config.layers().iter().any(|layer| {
        layer.source != ConfigSource::Default
            && matches!(layer.look_up_item("signing.backend"), Ok(Some(_)))
    })
}

#[derive(Default, Clone)]
struct GitSigningConfig {
    commit_gpgsign: Option<bool>,
    signing_key: Option<String>,
    gpg_format: Option<String>,
    gpg_program: Option<String>,
    gpg_ssh_program: Option<String>,
    gpg_x509_program: Option<String>,
}

impl GitSigningConfig {
    fn program_for_backend(&self, backend: &str) -> Option<String> {
        match backend {
            "ssh" => self.gpg_ssh_program.clone(),
            "gpgsm" => self.gpg_x509_program.clone(),
            _ => self.gpg_program.clone(),
        }
    }
}

fn read_git_signing_config(workspace_root: &Path) -> Option<GitSigningConfig> {
    let mut merged = GitSigningConfig::default();
    let mut saw_any = false;

    for path in git_signing_config_paths(workspace_root) {
        if merge_git_signing_config_file(&mut merged, path.as_path()) {
            saw_any = true;
        }
    }

    if saw_any { Some(merged) } else { None }
}

fn git_signing_config_paths(workspace_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(home_dir) = dirs::home_dir() {
        paths.push(home_dir.join(".gitconfig"));
    }

    let xdg_config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|path| path.join(".config")));
    if let Some(config_home) = xdg_config_home {
        paths.push(config_home.join("git").join("config"));
    }

    if let Some(path) = workspace_git_config_path(workspace_root)
        && !paths.contains(&path)
    {
        paths.push(path);
    }
    if let Some(path) = git_target_config_path(workspace_root)
        && !paths.contains(&path)
    {
        paths.push(path);
    }

    paths
}

fn workspace_git_config_path(workspace_root: &Path) -> Option<PathBuf> {
    let dot_git = workspace_root.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git.join("config"));
    }
    if dot_git.is_file() {
        let git_dir = fs::read_to_string(&dot_git).ok().and_then(|contents| {
            contents
                .lines()
                .find_map(|line| line.trim().strip_prefix("gitdir:"))
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
        })?;
        let git_dir = if git_dir.is_absolute() {
            git_dir
        } else {
            workspace_root.join(git_dir)
        };
        return Some(git_dir.join("config"));
    }

    None
}

fn git_target_config_path(workspace_root: &Path) -> Option<PathBuf> {
    let store_root = workspace_root.join(".jj").join("repo").join("store");
    let git_target_path = store_root.join("git_target");
    let raw_target = fs::read_to_string(&git_target_path).ok()?;
    let target = raw_target.trim();
    if target.is_empty() {
        return None;
    }

    let git_repo_path = {
        let target_path = PathBuf::from(target);
        if target_path.is_absolute() {
            target_path
        } else {
            store_root.join(target_path)
        }
    };
    Some(git_repo_path.join("config"))
}

fn merge_git_signing_config_file(config: &mut GitSigningConfig, path: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };

    let mut saw_any = false;
    let mut section = String::new();
    let mut subsection = None::<String>;

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let header = &line[1..line.len() - 1];
            let (name, sub) = parse_git_config_section_header(header);
            section = name;
            subsection = sub;
            continue;
        }

        let (key, value) = if let Some((key, value)) = line.split_once('=') {
            (
                key.trim().to_ascii_lowercase(),
                normalize_git_config_value(value),
            )
        } else {
            (line.to_ascii_lowercase(), "true".to_string())
        };
        if key.is_empty() {
            continue;
        }

        match (section.as_str(), subsection.as_deref(), key.as_str()) {
            ("commit", None, "gpgsign") => {
                if let Some(value) = parse_git_config_bool(value.as_str()) {
                    config.commit_gpgsign = Some(value);
                    saw_any = true;
                }
            }
            ("user", None, "signingkey") => {
                if !value.is_empty() {
                    config.signing_key = Some(value);
                    saw_any = true;
                }
            }
            ("gpg", None, "format") => {
                if !value.is_empty() {
                    config.gpg_format = Some(value.to_ascii_lowercase());
                    saw_any = true;
                }
            }
            ("gpg", None, "program") => {
                if !value.is_empty() {
                    config.gpg_program = Some(value);
                    saw_any = true;
                }
            }
            ("gpg", Some("ssh"), "program") => {
                if !value.is_empty() {
                    config.gpg_ssh_program = Some(value);
                    saw_any = true;
                }
            }
            ("gpg", Some("x509"), "program") => {
                if !value.is_empty() {
                    config.gpg_x509_program = Some(value);
                    saw_any = true;
                }
            }
            _ => {}
        }
    }

    saw_any
}

fn parse_git_config_section_header(header: &str) -> (String, Option<String>) {
    let mut parts = header.splitn(2, char::is_whitespace);
    let section = parts.next().unwrap_or_default().trim().to_ascii_lowercase();
    let subsection = parts
        .next()
        .map(str::trim)
        .map(normalize_git_config_value)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    (section, subsection)
}

fn normalize_git_config_value(value: &str) -> String {
    let value = value.trim();
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        value[1..value.len() - 1].trim().to_string()
    } else {
        value.to_string()
    }
}

fn parse_git_config_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn load_config_if_exists(
    config: &mut StackedConfig,
    source: ConfigSource,
    path: PathBuf,
) -> Result<()> {
    if path.is_file() {
        config
            .load_file(source, path.clone())
            .with_context(|| format!("failed to load jj config {}", path.display()))?;
    }
    Ok(())
}
