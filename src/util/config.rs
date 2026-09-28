//! Game configuration and retail asset-root discovery.
//!
//! config.toml is machine-specific (contains the local RA2 install path)
//! and is gitignored. A config.toml.example template is provided in the repo.
//! The working-directory override takes precedence over a config beside the
//! executable. The latter lets packaged apps launch without a particular cwd.
//! Without either config, assets are resolved beside the executable.
//!
//! ## Dependency rules
//! - config.rs is part of util/ â€” no dependencies on game modules.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// Optional host override, in the working or executable directory.
const CONFIG_FILE_NAME: &str = "config.toml";

/// Top-level game configuration, deserialized from config.toml.
///
/// Add new sections here as features are implemented (audio, game speed, etc.).
#[derive(Debug, Deserialize)]
pub struct GameConfig {
    /// File system paths (RA2 install directory).
    pub paths: PathsConfig,
    /// Graphics/window settings (all optional — sensible defaults provided).
    #[serde(default)]
    pub graphics: GraphicsConfig,
    /// Deterministic simulation settings.
    #[serde(default)]
    pub gameplay: GameplayConfig,
    /// Local player profile (name pre-filled into skirmish/multiplayer setup).
    #[serde(default)]
    pub profile: ProfileConfig,
    /// Optional OpenAI-compatible controller for computer houses.
    #[serde(default)]
    pub external_ai: ExternalAiConfig,
}

/// External strategy API settings. Credentials are deliberately read by the
/// app from the process environment and are never part of this config value.
#[derive(Debug, Deserialize)]
pub struct ExternalAiConfig {
    /// Whether to send strategic observations to the configured API.
    #[serde(default)]
    pub enabled: bool,
    /// Complete Chat Completions URL, for example `/v1/chat/completions`.
    #[serde(default)]
    pub endpoint: String,
    /// Provider model identifier.
    #[serde(default)]
    pub model: String,
    /// Minimum native binary-frame cadence for requests per computer house.
    #[serde(default = "default_external_ai_request_interval_frames")]
    pub request_interval_frames: u32,
    /// Maximum blocking HTTP request duration, in seconds.
    #[serde(default = "default_external_ai_request_timeout_secs")]
    pub request_timeout_secs: u32,
    /// Maximum number of action entries accepted from one response.
    #[serde(default = "default_external_ai_max_actions_per_response")]
    pub max_actions_per_response: usize,
    /// How long a valid response controls a house, in native binary frames.
    #[serde(default = "default_external_ai_control_lease_frames")]
    pub control_lease_frames: u32,
}

impl Default for ExternalAiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: String::new(),
            model: String::new(),
            request_interval_frames: default_external_ai_request_interval_frames(),
            request_timeout_secs: default_external_ai_request_timeout_secs(),
            max_actions_per_response: default_external_ai_max_actions_per_response(),
            control_lease_frames: default_external_ai_control_lease_frames(),
        }
    }
}

impl ExternalAiConfig {
    /// Validate settings when the app is about to enable the network worker.
    /// Disabled config remains harmless even when no provider is configured.
    pub fn validate(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        anyhow::ensure!(
            has_http_endpoint_authority(&self.endpoint),
            "external AI endpoint must be a complete HTTP or HTTPS URL without embedded credentials"
        );
        anyhow::ensure!(
            !self.model.trim().is_empty(),
            "external AI model must not be blank"
        );
        anyhow::ensure!(
            (1..=3600).contains(&self.request_interval_frames),
            "external AI request_interval_frames must be between 1 and 3600"
        );
        anyhow::ensure!(
            (1..=120).contains(&self.request_timeout_secs),
            "external AI request_timeout_secs must be between 1 and 120"
        );
        anyhow::ensure!(
            (1..=64).contains(&self.max_actions_per_response),
            "external AI max_actions_per_response must be between 1 and 64"
        );
        anyhow::ensure!(
            (1..=3600).contains(&self.control_lease_frames),
            "external AI control_lease_frames must be between 1 and 3600"
        );
        Ok(())
    }
}

fn has_http_endpoint_authority(endpoint: &str) -> bool {
    if endpoint.is_empty() || endpoint.chars().any(char::is_whitespace) {
        return false;
    }
    let Some(authority_and_path) = endpoint
        .strip_prefix("https://")
        .or_else(|| endpoint.strip_prefix("http://"))
    else {
        return false;
    };
    let authority = authority_and_path
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    !authority.is_empty() && !authority.contains('@')
}

/// Local player profile settings.
///
/// `[profile]` may be omitted entirely. `name` is the persistent player handle
/// the setup screen pre-fills into the name field; when unset the setup UI
/// falls back to its own default. This mirrors the original reading the player
/// name from a persistent profile source rather than a baked-in string.
#[derive(Debug, Deserialize, Default)]
pub struct ProfileConfig {
    /// Player name shown/edited in skirmish setup. `None` (or empty) means use
    /// the setup screen's built-in default.
    #[serde(default)]
    pub name: Option<String>,
}

fn default_external_ai_request_interval_frames() -> u32 {
    225
}

fn default_external_ai_request_timeout_secs() -> u32 {
    15
}

fn default_external_ai_max_actions_per_response() -> usize {
    16
}

fn default_external_ai_control_lease_frames() -> u32 {
    450
}

impl ProfileConfig {
    /// The configured player name, trimmed; `None` when unset or blank so the
    /// caller can apply its own default.
    pub fn player_name(&self) -> Option<&str> {
        self.name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
    }
}

/// Paths to external resources (the player's RA2 installation).
#[derive(Debug, Deserialize)]
pub struct PathsConfig {
    /// Path to the user's RA2 installation directory.
    /// MIX files (ra2.mix, language.mix, theme.mix) are loaded from here.
    /// Example: "C:/Program Files/EA Games/Command and Conquer Red Alert II"
    pub ra2_dir: PathBuf,
}

/// Graphics and window settings.
///
/// Every field has a sensible default so `[graphics]` can be omitted entirely.
#[derive(Debug, Deserialize)]
pub struct GraphicsConfig {
    /// Window width in pixels.
    #[serde(default = "default_width")]
    pub width: u32,
    /// Window height in pixels.
    #[serde(default = "default_height")]
    pub height: u32,
    /// Whether to enable vertical sync (reduces tearing, caps framerate).
    #[serde(default = "default_true")]
    pub vsync: bool,
    /// Enable Catmull-Rom bicubic upscaling (renders at half resolution, upscales to window).
    #[serde(default)]
    pub upscale: bool,
    /// Enable cosmetic per-frame effects: water/ore sparkles. Also intended to
    /// gate future cosmetic effects (laser beam pulses, particle systems, line
    /// trails) per gamemd's "Extra Animations" option. Default ON to match
    /// gamemd's default.
    #[serde(default = "default_true")]
    pub extra_animations: bool,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
            vsync: true,
            upscale: false,
            extra_animations: true,
        }
    }
}

impl GraphicsConfig {
    /// Render width: half of window width when upscaling, otherwise full window width.
    pub fn render_width(&self) -> u32 {
        if self.upscale {
            self.width / 2
        } else {
            self.width
        }
    }

    /// Render height: half of window height when upscaling, otherwise full window height.
    pub fn render_height(&self) -> u32 {
        if self.upscale {
            self.height / 2
        } else {
            self.height
        }
    }
}

/// Deterministic simulation and command scheduling settings.
#[derive(Debug, Deserialize)]
pub struct GameplayConfig {
    /// Fixed simulation tick rate (Hz).
    #[serde(default = "default_sim_tick_hz")]
    pub sim_tick_hz: u32,
    /// Input delay in ticks for lockstep-style command execution.
    #[serde(default = "default_input_delay_ticks")]
    pub input_delay_ticks: u32,
}

impl Default for GameplayConfig {
    fn default() -> Self {
        Self {
            sim_tick_hz: default_sim_tick_hz(),
            input_delay_ticks: default_input_delay_ticks(),
        }
    }
}

fn default_width() -> u32 {
    1024
}

fn default_height() -> u32 {
    768
}

fn default_true() -> bool {
    true
}

fn default_sim_tick_hz() -> u32 {
    15
}

fn default_input_delay_ticks() -> u32 {
    2
}

impl GameConfig {
    /// Load the optional host override, otherwise use the retail module directory.
    ///
    /// Retail provenance: Executable-root path discovery — `WinMain` @ `0x006BB9A0`.
    /// Active `gamemd.exe` `WinMain @ 0x006BB9A0` calls
    /// `GetModuleFileNameA`, splits/rebuilds its drive and directory, and calls
    /// `SetCurrentDirectoryA` before opening `RA2MD.INI` or any MIX archive.
    /// Rust keeps the resolved directory explicit instead of mutating the
    /// process-wide current directory.
    pub fn load() -> Result<Self> {
        let working_dir =
            std::env::current_dir().context("Failed to locate the working directory")?;
        let executable = std::env::current_exe()
            .context("Failed to locate the running executable for retail asset discovery")?;
        Self::load_from(&working_dir, &executable)
    }

    fn load_from(working_dir: &Path, executable: &Path) -> Result<Self> {
        let root = retail_asset_root_from_executable(executable)?;
        for directory in [working_dir, root.as_path()] {
            let path = directory.join(CONFIG_FILE_NAME);
            match std::fs::read_to_string(&path) {
                Ok(contents) => return Self::parse_from(&contents, &path),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(err).with_context(|| {
                        format!("Failed to read config file: {}", path.display())
                    });
                }
            }
        }
        log::info!(
            "No {}; using retail executable directory: {}",
            CONFIG_FILE_NAME,
            root.display()
        );
        Ok(Self::from_retail_asset_root(root))
    }

    fn parse_from(contents: &str, path: &Path) -> Result<Self> {
        let mut config: GameConfig = toml::from_str(contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        // A packaged config must mean the same thing after Finder changes cwd.
        if config.paths.ra2_dir.is_relative() {
            config.paths.ra2_dir = path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(&config.paths.ra2_dir);
        }

        log::info!("Loaded config from {}", path.display());
        log::info!("RA2 directory: {}", config.paths.ra2_dir.display());

        Ok(config)
    }

    fn from_retail_asset_root(ra2_dir: PathBuf) -> Self {
        Self {
            paths: PathsConfig { ra2_dir },
            graphics: GraphicsConfig::default(),
            gameplay: GameplayConfig::default(),
            profile: ProfileConfig::default(),
            external_ai: ExternalAiConfig::default(),
        }
    }
}

/// Return the directory that retail `WinMain` makes its file-search root.
fn retail_asset_root_from_executable(executable: &Path) -> Result<PathBuf> {
    executable
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .with_context(|| {
            format!(
                "Running executable has no containing directory: {}",
                executable.display()
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ConfigFixture(PathBuf);

    impl ConfigFixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let root =
                std::env::temp_dir().join(format!("vera20k-config-{}-{id}", std::process::id()));
            std::fs::create_dir(&root).expect("unique config fixture");
            std::fs::create_dir(root.join("launch")).unwrap();
            std::fs::create_dir(root.join("app")).unwrap();
            Self(root)
        }

        fn load(&self) -> Result<GameConfig> {
            GameConfig::load_from(&self.0.join("launch"), &self.0.join("app/vera20k"))
        }
    }

    impl Drop for ConfigFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn packaged_config_loads_from_an_unrelated_working_directory() {
        let fixture = ConfigFixture::new();
        std::fs::write(
            fixture.0.join("app/config.toml"),
            "[paths]\nra2_dir = 'retail files'\n[profile]\nname = 'Packaged player'\n",
        )
        .unwrap();
        let config = fixture.load().unwrap();
        assert_eq!(config.paths.ra2_dir, fixture.0.join("app/retail files"));
        assert_eq!(config.profile.player_name(), Some("Packaged player"));
    }

    #[test]
    fn working_directory_config_overrides_packaged_config() {
        let fixture = ConfigFixture::new();
        std::fs::write(fixture.0.join("app/config.toml"), "invalid package config").unwrap();
        std::fs::write(
            fixture.0.join("launch/config.toml"),
            "[paths]\nra2_dir = 'local retail'\n",
        )
        .unwrap();
        assert_eq!(
            fixture.load().unwrap().paths.ra2_dir,
            fixture.0.join("launch/local retail")
        );
    }

    #[test]
    fn invalid_override_is_reported_instead_of_silently_using_packaged_assets() {
        let fixture = ConfigFixture::new();
        std::fs::write(fixture.0.join("launch/config.toml"), "[broken").unwrap();
        std::fs::write(
            fixture.0.join("app/config.toml"),
            "[paths]\nra2_dir = '.'\n",
        )
        .unwrap();
        let message = format!("{:#}", fixture.load().unwrap_err());
        let expected_path = fixture.0.join("launch").join("config.toml");
        assert!(
            message.contains(expected_path.to_str().unwrap()),
            "error context did not contain expected path {expected_path:?}: {message}"
        );
        assert!(message.contains("Failed to parse config"));
    }

    #[test]
    fn missing_configs_keep_retail_executable_directory_discovery() {
        let fixture = ConfigFixture::new();
        assert_eq!(fixture.load().unwrap().paths.ra2_dir, fixture.0.join("app"));
    }

    #[test]
    fn test_minimal_config() {
        let toml_str = r#"
[paths]
ra2_dir = "C:/Westwood/RA2"
"#;
        let config: GameConfig = toml::from_str(toml_str).expect("Failed to parse test config");
        assert_eq!(config.graphics.width, 1024);
        assert_eq!(config.graphics.height, 768);
        assert!(config.graphics.vsync);
        assert!(!config.graphics.upscale);
        assert_eq!(config.gameplay.sim_tick_hz, 15);
        assert_eq!(config.gameplay.input_delay_ticks, 2);
        // No [profile] section -> no pre-filled player name.
        assert_eq!(config.profile.player_name(), None);
        assert!(!config.external_ai.enabled);
        assert_eq!(config.external_ai.request_interval_frames, 225);
        assert_eq!(config.external_ai.request_timeout_secs, 15);
        assert_eq!(config.external_ai.max_actions_per_response, 16);
        assert_eq!(config.external_ai.control_lease_frames, 450);
        assert!(config.external_ai.validate().is_ok());
    }

    #[test]
    fn enabled_external_ai_accepts_a_complete_compatible_endpoint() {
        let config: GameConfig = toml::from_str(
            r#"
[paths]
ra2_dir = "C:/Westwood/RA2"

[external_ai]
enabled = true
endpoint = "http://127.0.0.1:8000/v1/chat/completions"
model = "test-model"
request_interval_frames = 90
request_timeout_secs = 7
max_actions_per_response = 12
control_lease_frames = 180
"#,
        )
        .expect("valid external AI config");

        config.external_ai.validate().expect("valid enabled config");
        assert!(config.external_ai.enabled);
        assert_eq!(
            config.external_ai.endpoint,
            "http://127.0.0.1:8000/v1/chat/completions"
        );
        assert_eq!(config.external_ai.model, "test-model");
        assert_eq!(config.external_ai.request_interval_frames, 90);
        assert_eq!(config.external_ai.request_timeout_secs, 7);
        assert_eq!(config.external_ai.max_actions_per_response, 12);
        assert_eq!(config.external_ai.control_lease_frames, 180);
    }

    #[test]
    fn enabled_external_ai_requires_a_complete_http_endpoint_and_model() {
        for endpoint in [
            "",
            "localhost:8000/chat/completions",
            "ftp://example.test/chat",
            "https://user:secret@example.test/v1/chat/completions",
        ] {
            let config: GameConfig = toml::from_str(&format!(
                "[paths]\nra2_dir = '.'\n[external_ai]\nenabled = true\nendpoint = {endpoint:?}\nmodel = 'test-model'\n"
            ))
            .expect("parse endpoint fixture");
            assert!(
                config.external_ai.validate().is_err(),
                "accepted {endpoint:?}"
            );
        }

        let config: GameConfig = toml::from_str(
            "[paths]\nra2_dir = '.'\n[external_ai]\nenabled = true\nendpoint = 'https://example.test/v1/chat/completions'\nmodel = '  '\n",
        )
        .expect("parse blank model fixture");
        assert!(config.external_ai.validate().is_err());
    }

    #[test]
    fn enabled_external_ai_rejects_out_of_range_scheduling_limits() {
        for (field, value) in [
            ("request_interval_frames", 0),
            ("request_interval_frames", 3601),
            ("request_timeout_secs", 0),
            ("request_timeout_secs", 121),
            ("max_actions_per_response", 0),
            ("max_actions_per_response", 65),
            ("control_lease_frames", 0),
            ("control_lease_frames", 3601),
        ] {
            let config: GameConfig = toml::from_str(&format!(
                "[paths]\nra2_dir = '.'\n[external_ai]\nenabled = true\nendpoint = 'https://example.test/v1/chat/completions'\nmodel = 'test-model'\n{field} = {value}\n"
            ))
            .expect("parse range fixture");
            assert!(
                config.external_ai.validate().is_err(),
                "accepted {field}={value}"
            );
        }
    }

    #[test]
    fn test_profile_player_name_trims_and_blank_is_none() {
        let toml_str = r#"
[paths]
ra2_dir = "C:/Westwood/RA2"

[profile]
name = "  Commander  "
"#;
        let config: GameConfig = toml::from_str(toml_str).expect("Failed to parse test config");
        assert_eq!(config.profile.player_name(), Some("Commander"));

        let blank = r#"
[paths]
ra2_dir = "C:/Westwood/RA2"

[profile]
name = "   "
"#;
        let config: GameConfig = toml::from_str(blank).expect("Failed to parse test config");
        assert_eq!(config.profile.player_name(), None);
    }

    /// Host-native paths: `Path` only splits on the host's separators, so a
    /// Windows literal has no parent on Linux or macOS.
    fn host_path(windows: &str, unix: &str) -> PathBuf {
        PathBuf::from(if cfg!(windows) { windows } else { unix })
    }

    #[test]
    fn retail_asset_root_is_the_executable_directory() {
        let directory = host_path(r"C:\Westwood\RA2", "/opt/westwood/ra2");
        let executable = directory.join("gamemd.exe");
        assert_eq!(
            retail_asset_root_from_executable(&executable).expect("module directory"),
            directory
        );
    }

    #[test]
    fn retail_asset_root_preserves_a_volume_root_boundary() {
        let root = host_path(r"C:\", "/");
        let executable = root.join("gamemd.exe");
        assert_eq!(
            retail_asset_root_from_executable(&executable).expect("volume root"),
            root
        );
    }

    #[test]
    fn discovered_config_keeps_host_defaults() {
        let config = GameConfig::from_retail_asset_root(PathBuf::from(r"D:\Games\RA2"));
        assert_eq!(config.paths.ra2_dir, PathBuf::from(r"D:\Games\RA2"));
        assert_eq!(config.graphics.width, 1024);
        assert_eq!(config.graphics.height, 768);
        assert_eq!(config.gameplay.sim_tick_hz, 15);
        assert_eq!(config.profile.player_name(), None);
    }
}
