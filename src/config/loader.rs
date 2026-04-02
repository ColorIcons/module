use super::model::Config;
use anyhow::{Context, Result};
use std::{env, fs, path::PathBuf};

pub fn load(path: PathBuf) -> Result<Config> {
    let path = resolve(path)?;

    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file {:?}", path))?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("failed to parse TOML from {:?}", path))?;

    Ok(config)
}

fn resolve(path: PathBuf) -> Result<PathBuf> {
    if path.exists() {
        return Ok(path);
    }

    if let Ok(env_path) = env::var("CIP_CONFIG") {
        let p: PathBuf = env_path.into();
        if p.exists() {
            return Ok(p);
        }
    }

    let local = PathBuf::from(".cip.toml");
    if local.exists() {
        return Ok(local);
    }

    if let Some(mut global) = dirs::config_dir() {
        global.push("cip/config.toml");
        if global.exists() {
            return Ok(global);
        }
    }

    anyhow::bail!("no config found, run `cip init`");
}
