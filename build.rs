use regex::Regex;
use std::os::unix::prelude::{CommandExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, io};

fn visit_files(dir: &Path, cb: &dyn Fn(&Path)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_files(&path, cb)?;
            } else {
                cb(&path);
            }
        }
    }

    Ok(())
}

fn get_naga_bin_path() -> Option<PathBuf> {
    let home_dir = env::var_os("CARGO_HOME").unwrap();
    let mut naga_bin_path = Path::new(&home_dir).join("bin").join("naga");

    if !naga_bin_path.is_file() {
        let root_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
        let root_path = Path::new(&root_dir);
        naga_bin_path = root_path.join("bin").join("naga");
    }

    if !naga_bin_path.is_file() {
        println!("Naga not founded! Auto install, auto install...`");
        println!("cargo install naga-cli --root . --no-track");
        Command::new("cargo")
            .args(&["install", "naga-cli"])
            .args(&["--root", ".", "--no-track"])
            .exec();
        println!("Naga installed! {}", naga_bin_path.to_str().unwrap());
    }

    let permissions = naga_bin_path.metadata().unwrap().permissions();
    let mode = permissions.mode();
    let is_executable = mode & 0o111 != 0;
    if !is_executable {
        return None;
    }

    Some(naga_bin_path)
}

fn main() {
    println!("build.rs is running!");

    let root_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let root_path = Path::new(&root_dir);
    let out_path = Path::new(&out_dir);
    let wgsl_ext_reg = Regex::new(r"\.wgsl$").unwrap();

    let shader_dir = root_path.join("src").join("shaders");
    let shader_out_dir = out_path.join("shaders");
    fs::create_dir_all(&shader_out_dir).unwrap();
    visit_files(&shader_dir, &|path| {
        let relative = path.strip_prefix(&shader_dir).unwrap().to_str().unwrap();
        let result = wgsl_ext_reg.replace(&relative, ".spv");
        let output_path = shader_out_dir.join(result.as_ref());

        let input = path.to_str().unwrap();
        let output = output_path.to_str().unwrap();

        let naga_bin_path = get_naga_bin_path().unwrap();
        Command::new(&naga_bin_path)
            .args(&[input, output, "--keep-coordinate-space"])
            .exec();

        println!("Shader Output: {}", output);
    })
    .unwrap();

    println!("cargo:rerun-if-changed={}", &shader_dir.to_str().unwrap());
}
