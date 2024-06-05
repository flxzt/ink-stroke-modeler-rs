use std::path::PathBuf;

use anyhow::Context;
use path_slash::PathBufExt;

fn main() -> anyhow::Result<()> {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let install_lib_dir = out_dir.join("lib");
    let install_include_dir = out_dir.join("include");

    let bindings_files = [
        PathBuf::from("build.rs"),
        PathBuf::from("src/lib.rs"),
        PathBuf::from("include/extras.h"),
    ];
    let bindings_cpp_sources = [PathBuf::from("src/extras.cc")];

    if cfg!(feature = "build_absl") {
        eprintln!("### building absl-cpp ###");

        let _absl_cmake_install_dir = cmake::Config::new("abseil-cpp")
            // This avoids having to link to `dbghelp` on windows-mingw
            .profile("Release")
            .define("CMAKE_CXX_STANDARD", "20")
            // Rust needs -fPIE or -fPIC
            .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
            .define("CMAKE_PREFIX_PATH", &out_dir.to_slash_lossy().to_string())
            .define(
                "CMAKE_INSTALL_PREFIX",
                &out_dir.to_slash_lossy().to_string(),
            )
            .define(
                "CMAKE_INSTALL_LIBDIR",
                &install_lib_dir.to_slash_lossy().to_string(),
            )
            .define(
                "CMAKE_INSTALL_INCLUDEDIR",
                &install_include_dir.to_slash_lossy().to_string(),
            )
            .define("BUILD_TESTING", "OFF")
            .define("BUILD_SHARED_LIBS", "OFF")
            .define("ABSL_PROPAGATE_CXX_STD", "ON")
            // This forces absl stdcpp waiter implementation (see `absl/synchronization/internal/waiter.h`).
            // It possibly circumvents build failure with mingw. see: https://github.com/abseil/abseil-cpp/issues/1510
            .cxxflag("-DABSL_FORCE_WAITER_MODE=4")
            .build();
    }

    eprintln!("### building ink-stroke-modeler ###");

    let mut ink_stroke_modeler_config = cmake::Config::new("ink-stroke-modeler");
    // This avoids having to link to `dbghelp` on windows-mingw
    ink_stroke_modeler_config
        .profile("Release")
        .define("CMAKE_CXX_STANDARD", "20")
        // Rust needs -fPIE or -fPIC
        .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
        .define(
            "CMAKE_INSTALL_PREFIX",
            &out_dir.to_slash_lossy().to_string(),
        )
        .define(
            "CMAKE_INSTALL_LIBDIR",
            &install_lib_dir.to_slash_lossy().to_string(),
        )
        .define(
            "CMAKE_INSTALL_INCLUDEDIR",
            &install_include_dir.to_slash_lossy().to_string(),
        )
        .define("INK_STROKE_MODELER_ENABLE_INSTALL", "ON")
        .define("INK_STROKE_MODELER_BUILD_TESTING", "OFF");

    if cfg!(feature = "build_absl") {
        ink_stroke_modeler_config
            // This takes priority in cmake's find_package() when searching for absl to use our compiled version
            // instead of the system-provided package
            .define("CMAKE_PREFIX_PATH", &out_dir.to_slash_lossy().to_string());
    } else {
        ink_stroke_modeler_config.define("INK_STROKE_MODELER_FIND_DEPENDENCIES", "ON");
    }

    let _ink_stroke_modeler_cmake_install_dir = ink_stroke_modeler_config.build();

    let include_paths = [
        PathBuf::from("include"),
        #[cfg(feature = "build_absl")]
        PathBuf::from("absl-cpp"),
        PathBuf::from("ink-stroke-modeler"),
        install_include_dir,
    ];

    eprintln!("### building ink-stroke-modeler-rs autocxx bindings rust code ###");

    let mut builder =
        autocxx_build::Builder::new(PathBuf::from("src/lib.rs"), include_paths.iter())
            .extra_clang_args(&["-std=c++20", "-v"])
            .build()?;

    eprintln!("### building ink-stroke-modeler-rs autocxx bindings cpp code ###");

    builder
        //.flag_if_supported("-v")
        .flag_if_supported("-std=gnu++20")
        // These include paths are already passed in by the autocxx builder
        //.includes(include_paths.iter())
        //.cpp_set_stdlib(Some("stdc++"))
        .files(bindings_cpp_sources.iter())
        .try_compile("ink-stroke-modeler-rs")?;

    // Linking

    println!(
        "cargo:rustc-link-search=native={}",
        install_lib_dir.display()
    );

    if cfg!(feature = "build_absl") {
        for lib in std::fs::read_dir(install_lib_dir)? {
            let lib = lib?;
            let lib_name = lib.file_name().to_string_lossy().to_string();
            let starts_with_libabsl =
                cfg!(feature = "build_absl") && lib_name.starts_with("libabsl");
            let starts_with_liblink_stroke_modeler = lib_name.starts_with("libink_stroke_modeler");

            if starts_with_libabsl || starts_with_liblink_stroke_modeler {
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
    } else {
        let absl_lib = pkg_config::probe_library("absl")
            .context("Could not fetch pkg-config info about absl system lib")?;
        for lib in absl_lib.libs {
            println!("cargo:rustc-link-lib=static={}", lib);
        }
    }

    // Re-run when files are modified
    for source in bindings_files.iter().chain(bindings_cpp_sources.iter()) {
        println!("cargo:rerun-if-changed={}", source.display());
    }

    Ok(())
}
