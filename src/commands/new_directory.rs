use std::path;

use crate::commands::{JoshutoCommand, JoshutoRunnable};
use crate::context::JoshutoContext;
use crate::error::JoshutoResult;
use crate::history::DirectoryHistory;
use crate::ui::TuiBackend;
use crate::util::load_child::LoadChild;

#[derive(Clone, Debug)]
pub struct NewDirectory {
    path: path::PathBuf,
}

impl NewDirectory {
    pub fn new(path: path::PathBuf) -> Self {
        NewDirectory { path }
    }
    pub const fn command() -> &'static str {
        "mkdir"
    }
}

impl JoshutoCommand for NewDirectory {}

impl std::fmt::Display for NewDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(Self::command())
    }
}

impl JoshutoRunnable for NewDirectory {
    fn execute(&self, context: &mut JoshutoContext, _: &mut TuiBackend) -> JoshutoResult<()> {
        std::fs::create_dir_all(&self.path)?;

        let options = &context.config_t.sort_option;
        let curr_path = context.tabs[context.curr_tab_index].curr_path.clone();
        for tab in context.tabs.iter_mut() {
            tab.history.reload(&curr_path, options)?;
        }

        LoadChild::load_child(context)?;
        Ok(())
    }
}
