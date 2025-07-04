use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use clap::{ArgAction, Args};
use color_eyre::eyre::{eyre, OptionExt};
use me3_launcher_attach_protocol::AttachConfig;
use me3_mod_protocol::{
    dependency::sort_dependencies,
    native::Native,
    package::{Package, WithPackageSource},
    ModProfile,
};
use normpath::PathExt;
use steamlocate::{CompatTool, SteamDir};
use tempfile::NamedTempFile;
use tracing::info;

use crate::{AppPaths, Config, Game};

#[derive(Debug, clap::Args)]
#[group(required = true, multiple = false)]
pub struct Selector {
    /// Automatically detect the game to launch from mod profiles.
    #[clap(long, help_heading = "Game selection", action = ArgAction::SetTrue)]
    auto_detect: bool,

    /// A path to the game executable to launch with mod support.
    #[clap(short('e'), long, help_heading = "Game selection", value_hint = clap::ValueHint::FilePath)]
    exe: Option<PathBuf>,

    /// Short name of a game to launch. The launcher will look for the the installation in
    /// available Steam libraries.
    #[clap(
        short('g'),
        long,
        hide_possible_values = false,
        help_heading = "Game selection"
    )]
    #[arg(value_enum)]
    game: Option<Game>,

    /// The Steam APPID of the game to launch. The launcher will attempt to find this app installed
    /// in a Steam library and launch the configured command
    #[clap(short('s'), long, alias("steamid"), help_heading = "Game selection")]
    #[arg(value_parser = clap::value_parser!(u32))]
    steam_id: Option<u32>,
}

#[derive(Args, Debug)]
pub struct LaunchArgs {
    #[clap(flatten)]
    pub target_selector: Selector,

    /// Path to a ModProfile configuration file (TOML, JSON, or YAML) or name of a profile
    /// stored in the me3 profile folder ($XDG_CONFIG_HOME/me3).
    #[arg(
            short('p'),
            long("profile"),
            action = clap::ArgAction::Append,
            help_heading = "Mod configuration",
            value_hint = clap::ValueHint::FilePath,
        )]
    profiles: Vec<String>,

    /// Path to package directores that the mod host will use as VFS mount points.
    #[arg(
            long("package"),
            action = clap::ArgAction::Append,
            help_heading = "Mod configuration",
            value_hint = clap::ValueHint::DirPath,
        )]
    packages: Vec<PathBuf>,

    /// Path to DLLs to be loaded by the mod host.
    #[arg(
            short('n'),
            long("native"),
            action = clap::ArgAction::Append,
            help_heading = "Mod configuration",
            value_hint = clap::ValueHint::FilePath,
        )]
    natives: Vec<PathBuf>,
}

pub fn launcher_for(name: &str) -> Option<PathBuf> {
    let path = match name {
        "ELDEN RING" => "Game/eldenring.exe",
        "ELDEN RING NIGHTREIGN" => "Game/nightreign.exe",
        _ => return None,
    };

    Some(PathBuf::from(path))
}

pub trait Launcher {
    fn into_command(self, launcher: PathBuf) -> color_eyre::Result<Command>;
}

pub struct DirectLauncher;

impl Launcher for DirectLauncher {
    fn into_command(self, launcher: PathBuf) -> color_eyre::Result<Command> {
        Ok(Command::new(launcher))
    }
}

pub struct CompatToolLauncher {
    tool: CompatTool,
    steam: SteamDir,
}

impl Launcher for CompatToolLauncher {
    fn into_command(self, launcher: PathBuf) -> color_eyre::Result<Command> {
        // TODO: parse this from appcache/appinfo.vcf
        let sniper_id = 1628350;
        let proton_id = match self.tool.name.expect("must have a name").as_str() {
            "proton_experimental" => 1493710,
            "proton_hotfix" => 2180100,
            "proton_9" => 2805730,
            _ => return Err(eyre!("unrecognised compat tool")),
        };

        let (sniper_app, sniper_library) = self
            .steam
            .find_app(sniper_id)?
            .ok_or_eyre("unable to find Steam Linux Runtime")?;

        let (proton_app, proton_library) = self
            .steam
            .find_app(proton_id)?
            .ok_or_eyre("configured compat tool isn't installed")?;

        let sniper_path = sniper_library.resolve_app_dir(&sniper_app);
        let proton_path = proton_library.resolve_app_dir(&proton_app);

        let mut command = Command::new(sniper_path.join("run"));

        command.args([
            "--batch",
            "--",
            &*proton_path.join("proton").to_string_lossy(),
            "waitforexitandrun",
            &*launcher.to_string_lossy(),
        ]);

        // <https://gitlab.steamos.cloud/steamrt/steam-runtime-tools/-/blob/main/docs/steam-compat-tool-interface.md>
        command.env("STEAM_COMPAT_CLIENT_INSTALL_PATH", self.steam.path());
        command.env(
            "STEAM_COMPAT_DATA_PATH",
            self.steam.path().join("steamapps/compatdata/1245620"),
        );

        Ok(command)
    }
}

#[tracing::instrument(skip(config, paths, bins_dir))]
pub fn launch(
    config: Config,
    paths: AppPaths,
    bins_dir: PathBuf,
    args: LaunchArgs,
) -> color_eyre::Result<()> {
    let mut all_natives = vec![];
    let mut all_packages = vec![];

    all_packages.extend(
        args.packages
            .into_iter()
            .filter_map(|path| path.normalize().ok())
            .map(|normalized| Package::new(normalized.into_path_buf())),
    );

    all_natives.extend(
        args.natives
            .into_iter()
            .filter_map(|path| path.normalize().ok())
            .map(|normalized| Native::new(normalized.into_path_buf())),
    );

    let mut profile_supported_games = BTreeSet::new();

    for profile_name in args.profiles {
        let profile_path = config.resolve_profile(&profile_name)?;

        let base = profile_path
            .parent()
            .and_then(|parent| parent.normalize().ok())
            .ok_or_eyre("failed to normalize base directory for mod profile")?;

        let profile = ModProfile::from_file(&profile_path)?;

        let mut packages = profile.packages();
        let mut natives = profile.natives();

        packages
            .iter_mut()
            .for_each(|pkg| pkg.source_mut().make_absolute(base.as_path()));
        natives
            .iter_mut()
            .for_each(|pkg| pkg.source_mut().make_absolute(base.as_path()));

        all_packages.extend(packages);
        all_natives.extend(natives);

        for supports in profile.supports() {
            match supports.game {
                me3_mod_protocol::Game::EldenRing => {
                    profile_supported_games.insert(Game::EldenRing);
                }
                me3_mod_protocol::Game::Nightreign => {
                    profile_supported_games.insert(Game::Nightreign);
                }
            }
        }
    }

    let app_id = if args.target_selector.auto_detect {
        if profile_supported_games.len() > 1 {
            Err(eyre!(
                "profile supports more than one game, unable to auto-detect"
            ))
        } else {
            profile_supported_games
                .pop_first()
                .map(Game::app_id)
                .ok_or_eyre("unable to auto-detect appid of game")
        }
    } else {
        args.target_selector
            .steam_id
            .or_else(|| args.target_selector.game.map(Game::app_id))
            .ok_or_eyre("unable to determine app ID for game")
    }?;

    info!(app_id, "resolved app id");

    let steam_dir = config.resolve_steam_dir()?;
    info!(?steam_dir, "found steam dir");

    let (steam_app, steam_library) = steam_dir
        .find_app(app_id)?
        .ok_or_eyre("installation for requested game wasn't found")?;
    info!(name = ?steam_app.name, "found steam app in library");

    let app_install_path = steam_library.resolve_app_dir(&steam_app);
    info!(?app_install_path, "found steam app path");

    let launcher_path = launcher_for(&steam_app.name.expect("app must have a name"))
        .ok_or_eyre("unable to determine path to launcher for game")?;
    info!(?launcher_path, "found steam app launcher");

    let launcher = app_install_path.join(launcher_path);

    let ordered_natives = sort_dependencies(all_natives)?;
    let ordered_packages = sort_dependencies(all_packages)?;

    let attach_config_dir = paths.cache_path.unwrap_or(app_install_path.clone());
    std::fs::create_dir_all(&attach_config_dir)?;

    let attach_config_file = NamedTempFile::new_in(&attach_config_dir)?;
    let attach_config = AttachConfig {
        packages: ordered_packages,
        natives: ordered_natives,
    };

    std::fs::write(&attach_config_file, toml::to_string_pretty(&attach_config)?)?;
    info!(?attach_config_file, ?attach_config, "wrote attach config");

    let injector_path: PathBuf = bins_dir.join("me3-launcher.exe");
    let dll_path: PathBuf = bins_dir.join("me3_mod_host.dll");

    let mut injector_command = if cfg!(target_os = "linux") {
        let compat_tools = steam_dir.compat_tool_mapping()?;
        let app_compat_tool = compat_tools
            .get(&app_id)
            .ok_or_eyre("unable to find compat tool for game")?;

        info!(?app_compat_tool, "found compat tool for appid");

        let launcher = CompatToolLauncher {
            steam: steam_dir,
            tool: app_compat_tool.clone(),
        };

        launcher.into_command(injector_path)
    } else {
        DirectLauncher.into_command(injector_path)
    }?;

    if let Some(dir) = paths.logs_path.as_ref() {
        fs::create_dir_all(dir)?;
    }

    let log_file = tempfile::Builder::new()
        .disable_cleanup(true)
        .suffix(".log")
        .prefix("me3-log-")
        .tempfile_in(paths.logs_path.unwrap_or_default())?;

    injector_command.env("ME3_LOG_FILE", log_file.path().normalize()?);
    injector_command.env("ME3_GAME_EXE", launcher);
    injector_command.env("ME3_HOST_DLL", dll_path);
    injector_command.env("ME3_HOST_CONFIG_PATH", attach_config_file.path());
    injector_command.env("SteamAppId", app_id.to_string());
    injector_command.env("SteamGameId", app_id.to_string());

    if config.crash_reporting {
        injector_command.env("ME3_TELEMETRY", "true");
    }

    info!(?injector_command, "running injector command");

    let running = Arc::new(AtomicBool::new(true));
    let mut launcher_proc = injector_command.spawn()?;

    let monitor_thread_running = running.clone();
    let monitor_thread = std::thread::spawn(move || {
        let mut log_reader = BufReader::new(log_file);

        while monitor_thread_running.load(Ordering::SeqCst) {
            if let Some(_exit_code) = launcher_proc
                .try_wait()
                .expect("error while checking status")
            {
                break;
            }

            let mut line = String::new();
            log_reader
                .read_line(&mut line)
                .expect("failed to read line from logs");

            if !line.is_empty() {
                eprint!("{line}");
            }
        }

        let _ = launcher_proc.kill();
    });

    ctrlc::set_handler(move || {
        running.store(false, Ordering::SeqCst);
    })?;

    let _ = monitor_thread.join();

    Ok(())
}
