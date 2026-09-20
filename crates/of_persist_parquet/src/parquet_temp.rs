use super::*;

pub(crate) struct TemporaryExport {
    path: PathBuf,
    published: bool,
}

impl TemporaryExport {
    pub(crate) fn create(path: &Path) -> MarketDataParquetResult<(Self, File)> {
        let file = File::options()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    MarketDataParquetError::InvalidMetadata(format!(
                        "temporary export already exists: {}",
                        path.display()
                    ))
                } else {
                    MarketDataParquetError::Io(error)
                }
            })?;
        Ok((
            Self {
                path: path.to_owned(),
                published: false,
            },
            file,
        ))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn link_to(&mut self, destination: &Path) -> MarketDataParquetResult<()> {
        fs::hard_link(&self.path, destination).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                MarketDataParquetError::InvalidMetadata(format!(
                    "export destination already exists: {}",
                    destination.display()
                ))
            } else {
                MarketDataParquetError::Io(error)
            }
        })?;
        if let Err(error) = fs::remove_file(&self.path) {
            let _ = fs::remove_file(destination);
            return Err(MarketDataParquetError::Io(error));
        }
        self.path = destination.to_owned();
        Ok(())
    }

    pub(crate) fn publish(&mut self) {
        self.published = true;
    }
}

impl Drop for TemporaryExport {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_file(&self.path);
        }
    }
}
