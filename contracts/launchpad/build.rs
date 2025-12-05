use std::{fs, path::PathBuf};

fn main() {
    sails_rs::build_wasm();

    if let Ok(content) = fs::read_to_string(".binpath") {
        let wasm_path = PathBuf::from(content.trim());
        let idl_path = wasm_path.with_extension("idl");

        sails_idl_gen::generate_idl_to_file::<launchpad_app::LaunchpadProgram>(&idl_path).unwrap();
    }
}
