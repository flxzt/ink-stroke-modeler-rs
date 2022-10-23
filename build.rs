use path_slash::PathBufExt;
use std::path::PathBuf;

#[allow(unused)]
macro_rules! build_print {
    ($($tokens: tt)*) => {
        println!("cargo:warning=DEBUG: {}", format!($($tokens)*))
    }
}

fn main() -> anyhow::Result<()> {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    build_print!("{out_dir:?}");

    let install_lib_dir = if out_dir.join("lib64").exists() {
        out_dir.join("lib64")
    } else {
        out_dir.join("lib")
    };

    let install_include_dir = out_dir.join("include");
    let cmake_config_dir = install_lib_dir.join("cmake");

    let bindings_files = vec![
        PathBuf::from("build.rs"),
        PathBuf::from("src/lib.rs"),
        PathBuf::from("include/extras.h"),
    ];

    let bindings_cpp_sources = vec![PathBuf::from("src/extras.cc")];

    let _absl_cmake_install_dir = cmake::Config::new("abseil-cpp")
        .define("ABSL_PROPAGATE_CXX_STD", "ON")
        .define("BUILD_TESTING", "OFF")
        .define(
            "CMAKE_INSTALL_PREFIX",
            &out_dir.to_slash_lossy().to_string(),
        )
        .define(
            "CMAKE_MODULE_PATH",
            &cmake_config_dir.to_slash_lossy().to_string(),
        )
        .define(
            "CMAKE_INSTALL_LIBDIR",
            &install_lib_dir.to_slash_lossy().to_string(),
        )
        .define(
            "CMAKE_INSTALL_INCLUDEDIR",
            &install_include_dir.to_slash_lossy().to_string(),
        )
        .build();

    let _ink_stroke_modeler_cmake_install_dir = cmake::Config::new("ink-stroke-modeler")
        .define("INK_STROKE_MODELER_FIND_DEPENDENCIES", "ON")
        .define("INK_STROKE_MODELER_BUILD_TESTING", "OFF")
        .define("INK_STROKE_MODELER_ENABLE_INSTALL", "ON")
        .define(
            "CMAKE_INSTALL_PREFIX",
            &out_dir.to_slash_lossy().to_string(),
        )
        .define(
            "CMAKE_MODULE_PATH",
            &cmake_config_dir.to_slash_lossy().to_string(),
        )
        .define(
            "CMAKE_INSTALL_LIBDIR",
            &install_lib_dir.to_slash_lossy().to_string(),
        )
        .define(
            "CMAKE_INSTALL_INCLUDEDIR",
            &install_include_dir.to_slash_lossy().to_string(),
        )
        .build();

    let include_paths = vec![
        PathBuf::from("include"),
        PathBuf::from("absl-cpp"),
        PathBuf::from("ink-stroke-modeler"),
        install_include_dir,
    ];

    let mut builder = autocxx_build::Builder::new(
        "src/lib.rs",
        include_paths.iter(),
    )
    .extra_clang_args(&["-std=c++17"])
    .build()?;
    builder
        .compiler("clang++")
        .flag_if_supported("-v -std=c++17")
        .includes(include_paths.iter())
        .cpp_link_stdlib(Some("stdc++"))
        .files(bindings_cpp_sources.iter())
        .compile("ink-stroke-modeler-rs");

    // Linking
    println!(
        "cargo:rustc-link-search=native={}",
        install_lib_dir.display()
    );

    for lib in std::fs::read_dir(install_lib_dir)? {
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

    // Files
    for source in bindings_files.iter().chain(bindings_cpp_sources.iter()) {
        println!("cargo:rerun-if-changed={}", source.display());
    }

    Ok(())
}
