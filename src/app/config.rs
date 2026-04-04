use crate::{
    cli::config::{GetCmd, SetCmd},
    config::{loader, model},
    core::types::Index,
};
use anyhow::Result;
use std::fs;
use toml_edit::{value, DocumentMut};

const DEFAULT_CONFIG: &str = r#"[icons]
light = true
dark = false
mat = false
monochrome = true

[network]
concurrency = 4

[repo]
base_url = "https://coloricons.github.io/icons/"
"#;

pub fn init() -> anyhow::Result<()> {
    let path = model::CONFIG_PATH.clone();
    if path.exists() {
        anyhow::bail!("config exists");
    }

    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }

    fs::write(&path, DEFAULT_CONFIG)?;

    Ok(())
}

pub fn get(cmd: GetCmd) -> Result<()> {
    let config = loader::load(model::CONFIG_PATH.clone())?;

    if cmd.json {
        println!("{}", serde_json::to_string_pretty(&config)?);
    } else {
        println!("{:#?}", config);
    }

    Ok(())
}

fn set_value(doc: &mut DocumentMut, path: &[&str], val: impl Into<toml_edit::Value>) {
    use toml_edit::{Item, Table};

    let mut current: &mut Item = doc.as_item_mut();

    for key in &path[..path.len() - 1] {
        current = current
            .as_table_mut()
            .expect("should be table")
            .entry(key)
            .or_insert(Item::Table(Table::new()));
    }

    current
        .as_table_mut()
        .expect("should be table")
        .insert(path[path.len() - 1], value(val));
}

/// 修改配置
pub fn set(cmd: SetCmd) -> Result<()> {
    let path = model::CONFIG_PATH.clone();
    let index_path = model::INDEX_FILE_PATH.clone();

    let content = fs::read_to_string(&path)?;
    let mut doc = content.parse::<DocumentMut>()?;

    let mut icons_changed = false;

    if let Some(base_url) = cmd.base_url {
        set_value(&mut doc, &["repo", "base_url"], base_url);
    }

    if let Some(concurrency) = cmd.concurrency {
        set_value(&mut doc, &["network", "concurrency"], concurrency as i64);
    }

    if let Some(light) = cmd.light {
        set_value(&mut doc, &["icons", "light"], light);
        icons_changed = true;
    }

    if let Some(dark) = cmd.dark {
        set_value(&mut doc, &["icons", "dark"], dark);
        icons_changed = true;
    }

    if let Some(mat) = cmd.mat {
        set_value(&mut doc, &["icons", "mat"], mat);
        icons_changed = true;
    }

    if let Some(monochrome) = cmd.monochrome {
        set_value(&mut doc, &["icons", "monochrome"], monochrome);
        icons_changed = true;
    }

    if icons_changed {
        let content = fs::read_to_string(&index_path)?;
        let mut index: Index = serde_json::from_str(&content)?;

        index.generated_at = index.generated_at.saturating_sub(100);

        fs::write(&index_path, serde_json::to_string_pretty(&index)?)?;
    }

    fs::write(&path, doc.to_string())?;

    println!("✔ config updated");

    Ok(())
}
