use std::io::Write;

use mmp_server::openapi::ApiDoc;

fn main() -> std::io::Result<()> {
    let json = ApiDoc::openapi_with_routes();
    match std::env::args().nth(1) {
        Some(path) => {
            let path = std::path::PathBuf::from(path);
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, json)
        }
        None => std::io::stdout().write_all(json.as_bytes()),
    }
}
