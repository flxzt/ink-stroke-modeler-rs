use std::path::PathBuf;

#[allow(unused)]
macro_rules! build_print {
    ($($tokens: tt)*) => {
        println!("cargo:warning=DEBUG: {}", format!($($tokens)*))
    }
}

fn main() -> anyhow::Result<()> {
    let _out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    let bindings_files = vec![
        PathBuf::from("build.rs"),
        PathBuf::from("src/lib.rs"),
        PathBuf::from("include/extras.h"),
    ];

    let bindings_cpp_sources = vec![
        PathBuf::from("src/extras.cc")
    ];

    let cmake_dst_path = cmake::Config::new("ink-stroke-modeler").build();

    let cmake_lib_paths = {
        let mut paths = vec![];

        let mut lib = cmake_dst_path.clone();
        lib.push("lib");

        let mut lib64 = cmake_dst_path.clone();
        lib64.push("lib64");

        if lib.exists() {
            paths.push(lib);
        }
        if lib64.exists() {
            paths.push(lib64);
        }

        paths
    };

    let mut cmake_incl_path = cmake_dst_path.clone();
    cmake_incl_path.push("include");

    // It's necessary to use an absolute path here because the
    // C++ codegen and the macro codegen appears to be run from different
    // working directories.
    let include_paths = vec![
        PathBuf::from("include"),
        PathBuf::from("ink-stroke-modeler"),
        cmake_incl_path,
    ];

    let mut builder = autocxx_build::Builder::new("src/lib.rs", &include_paths)
        .extra_clang_args(&["-std=c++17"])
        .build()?;
    builder
        .flag_if_supported("-std=c++17")
        .includes(include_paths.iter())
        .files(bindings_cpp_sources.iter())
        .compile("ink-stroke-modeler-rs");

    // Linking
    for cmake_lib_path in cmake_lib_paths {
        println!(
            "cargo:rustc-link-search=native={}",
            cmake_lib_path.display()
        );

        for lib in std::fs::read_dir(cmake_lib_path)? {
            let lib = lib?;
            let lib_name = lib.file_name().to_string_lossy().to_string();

            if lib_name.starts_with("libabsl") || lib_name.starts_with("libink_stroke_modeler") {
                println!(
                    "cargo:rustc-link-lib=static={}",
                    lib.path()
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .trim_start_matches("lib")
                );
            }
        }
    }

    // Files
    for source in bindings_files.iter().chain(bindings_cpp_sources.iter()) {
        println!("cargo:rerun-if-changed={}", source.display());
    }

    Ok(())
}
