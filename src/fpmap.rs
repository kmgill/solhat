use std::collections::HashMap;

use anyhow::{Error, Result};

use crate::datasource::DataSource;

/** file pointer map */
pub struct FpMap<F: DataSource> {
    pub map: HashMap<String, F>,
}

impl<F: DataSource> Default for FpMap<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: DataSource> FpMap<F> {
    pub fn new() -> Self {
        FpMap {
            map: HashMap::new(),
        }
    }

    pub fn get_map(&self) -> &HashMap<String, F> {
        &self.map
    }

    pub fn contains(&self, path: &String) -> bool {
        self.map.contains_key(path)
    }

    // pub fn get_dont_open(&self, path: &String) -> Option<&F> {
    //     self.map.get(path)
    // }

    // pub fn get(&mut self, file_key: &String) -> Option<&F> {
    //     if !self.contains(file_key) {
    //         match self.open(&file_key) {
    //             Ok(_) => {}
    //             Err(e) => {
    //                 panic!("Failed to open file: {}", e);
    //             }
    //         };
    //     }
    //
    //     self.map.get(path)
    // }

    pub fn open(&mut self, paths: &[String]) -> Result<()> {
        let ser_file = F::open(paths)?;
        ser_file.validate()?;

        if !self.contains(&ser_file.file_hash()) {
            self.map.insert(ser_file.file_hash(), ser_file);
            Ok(())
        } else {
            Err(Error::msg("File already opened"))
        }
    }
}
