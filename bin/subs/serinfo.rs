use crate::subs::runnable::RunnableSubcommand;
use anyhow::Result;
use clap::Parser;
use sciimg::path;
use solhat::ser;

#[derive(Parser)]
#[command(author, version, about = "Print information from a SER file", long_about = None)]
pub struct SerInfo {
    #[clap(long, short, help = "Input ser file")]
    input_file: String,
}

#[async_trait::async_trait]
impl RunnableSubcommand for SerInfo {
    async fn run(&self) -> Result<()> {
        if path::file_exists(self.input_file.as_str()) {
            let ser_file =
                ser::SerFile::load_ser(&self.input_file).expect("Unable to load SER file");
            ser_file.validate();

            ser_file.print_header_details();
        }
        Ok(())
    }
}
