use anyhow::Result;

use crate::app::context::AppContext;
use crate::doctor::service::collect_issues;

pub fn run() -> Result<()> {
    let context = AppContext::from_env()?;
    let issues = collect_issues(&context.store)?;

    if issues.is_empty() {
        println!("no issues");
        return Ok(());
    }

    for issue in issues {
        println!("{issue}");
    }

    std::process::exit(1);
}
