use std::{fs::File, io::BufReader, path::PathBuf};

fn main() {
    sails_rs::build_wasm();

    if let Ok(bin_path_file) = File::open(".binpath") {
        let bin_path_reader = BufReader::new(bin_path_file);
        let wasm_path: PathBuf = serde_json::from_reader(bin_path_reader).unwrap();
        let idl_path = wasm_path.with_extension("idl");

        sails_idl_gen::generate_idl_to_file::<launchpad_app::LaunchpadProgram>(&idl_path).unwrap();
    }
}
