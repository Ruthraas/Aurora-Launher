pub mod archive;
pub mod batch;
pub mod file;
pub mod http;

pub use archive::{ensure_manifest_attribute, extract_prefixed, extract_zip, extract_zip_entry_to_file, read_zip_entry};
pub use batch::{download_all, DownloadTask};
pub use file::download_file;
