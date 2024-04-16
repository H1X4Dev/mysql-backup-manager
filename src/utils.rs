use std::fs;
use std::path::Path;

pub fn get_size<P: AsRef<Path>>(path: P) -> Result<u64, std::io::Error> {
    let path = path.as_ref();
    let metadata = fs::metadata(path)?;

    if metadata.is_file() {
        Ok(metadata.len())
    } else if metadata.is_dir() {
        let mut total_size = 0;
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            total_size += get_size(entry.path())?;
        }
        Ok(total_size)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Not a file or directory",
        ))
    }
}