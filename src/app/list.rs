use crate::{cli::list::ListCmd, config::model, core::list};

pub fn run(cmd: ListCmd) -> anyhow::Result<()> {
    list::run(model::STORAGE_ROOT.to_str().unwrap(), cmd.json);

    Ok(())
}
