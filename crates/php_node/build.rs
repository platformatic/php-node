use std::env;

extern crate napi_build;

// use std::{env, fs};
// use std::path::{PathBuf, Path};

fn main() {
  napi_build::setup();

  // Check for manual PHP_RPATH override, otherwise try LD_PRELOAD_PATH,
  // and finally fallback to hard-coded /usr/local/lib path.
  //
  // PHP_RPATH may also be $ORIGIN to instruct the build to search for libphp
  // in the same directory as the *.node bindings file.
  let php_rpath = env::var("PHP_RPATH")
    .or_else(|_| env::var("LD_PRELOAD_PATH"))
    .unwrap_or("/usr/local/lib".to_string());

  println!("cargo:rustc-link-search={}", php_rpath);
  println!("cargo:rustc-link-lib=dylib=php");
  println!("cargo:rustc-link-arg=-Wl,-rpath,{}", php_rpath);

  //   let out_dir = env::var("OUT_DIR").unwrap();

  //   // Tell cargo to look for shared libraries in the specified directory
  //   let php_dir = PathBuf::from("../../php-src/lib")
  //     .canonicalize()
  //     .expect("cannot canonicalize php-src path");
  //   println!("cargo:rustc-link-search={}", php_dir.to_str().unwrap());
  //   println!("cargo:rustc-link-search={}", out_dir);

  //   // If on Linux or MacOS, tell the linker where the shared libraries are
  //   // on runtime (i.e. LD_LIBRARY_PATH)
  //   println!("cargo:rustc-link-arg=-Wl,-rpath,{}", out_dir);

  //   // Tell cargo to link against the shared library for the specific platform.
  //   // IMPORTANT: On macOS and Linux the shared library must be linked without
  //   // the "lib" prefix and the ".so" suffix. On Windows the ".dll" suffix must
  //   // be omitted.
  //   let lib_file = match target_and_arch() {
  //       (Target::Linux, Arch::X86_64) => "libphp_linx64.so",
  //       (Target::Linux, Arch::AARCH64) => "libphp_linarm64.so",
  //       (Target::MacOS, Arch::X86_64) => "libphp_macx64.dylib",
  //       (Target::MacOS, Arch::AARCH64) => "libphp_macarm64.dylib"
  //   };

  //   copy_dylib_to_target_dir(lib_file);
  // }

  // fn copy_dylib_to_target_dir(dylib: &str) {
  //     let out_dir = env::var("OUT_DIR").unwrap();
  //     let src = Path::new("../php-src/lib");
  //     let dst = Path::new(&out_dir);
  //     let _ = fs::copy(src.join(dylib), dst.join(dylib));
  // }

  // enum Target {
  //     Linux,
  //     MacOS
  // }

  // enum Arch {
  //     X86_64,
  //     AARCH64,
  // }

  // fn target_and_arch() -> (Target, Arch) {
  //     let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
  //     let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
  //     match (os.as_str(), arch.as_str()) {
  //         // Linux targets
  //         ("linux", "x86_64") => (Target::Linux, Arch::X86_64),
  //         ("linux", "aarch64") => (Target::Linux, Arch::AARCH64),
  //         // MacOS targets
  //         ("macos", "x86_64") => (Target::MacOS, Arch::X86_64),
  //         ("macos", "aarch64") => (Target::MacOS, Arch::AARCH64),
  //         _ => panic!("Unsupported operating system {} and architecture {}", os, arch),
  //     }
}
