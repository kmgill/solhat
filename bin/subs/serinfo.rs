use anyhow::Result;
use clap::Parser;
use sciimg::path;

use solhat::datasource::DataSource;
use solhat::ser;

use crate::subs::runnable::RunnableSubcommand;

#[derive(Parser)]
#[command(author, version, about = "Print information from a SER file", long_about = None)]
pub struct SerInfo {
    #[clap(long, short, help = "Input ser file")]
    input_file: String,
}

fn do_validation<F: DataSource>(ser_file: &F) -> Result<()> {
    ser_file.validate()?;
    ser_file.print_header_details();
    Ok(())
}

#[async_trait::async_trait]
impl RunnableSubcommand for SerInfo {
    async fn run(&self) -> Result<()> {
        if path::file_exists(self.input_file.as_str()) {
            let ser_file =
                ser::SerFile::load_ser(&self.input_file).expect("Unable to load SER file");
            do_validation(&ser_file)?;
        }
        Ok(())
    }
}
