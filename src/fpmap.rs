use std::collections::HashMap;

use anyhow::{Error, Result};
use sciimg::path;

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

    pub fn get_dont_open(&self, path: &String) -> Option<&F> {
        self.map.get(path)
    }

    pub fn get(&mut self, path: &String) -> Option<&F> {
        if !self.contains(path) {
            match self.open(path) {
                Ok(_) => {}
                Err(e) => {
                    panic!("Failed to open file: {}", e);
                }
            };
        }

        self.map.get(path)
    }

    pub fn open(&mut self, path: &String) -> Result<()> {
        if self.contains(path) {
            return Err(Error::msg("File already open"));
        }

        info!("Opening file in fpmap: {}", path);

        if !path::file_exists(path) {
            panic!("File not found: {}", path);
        }

        match F::open(path) {
            Ok(ser_file) => {
                ser_file.validate()?;
                self.map.insert(path.clone(), ser_file);
                Ok(())
            }
            Err(e) => Err(Error::msg(e)),
        }
    }
}
