macro_rules! test_lib {
    ($test_name:ident) => {
        #[test]
        fn $test_name() {
            // Ensures the test directory is present
            std::fs::create_dir_all("./test/out").expect("Could not setup the test env");
            // Builds the backend if neceasry
            std::process::Command::new("cargo")
                .args(["build"])
                .output()
                .expect("could not build the backend");
            // Compiles the test project
            let out = std::process::Command::new("rustc")
                .current_dir("./test/out")
                .args([
                    "-O",
                    "--crate-type=lib",
                    "-Z",
                    backend_path(),
                    concat!("../", stringify!($test_name), ".rs"),
                    "-o",
                    concat!("./", stringify!($test_name), ".dll"),
                ])
                .output()
                .expect("failed to execute process");
            // If stderr is not empty, then something went wrong, so print the stdout and stderr for debuging.
            if !out.stderr.is_empty() {
                let stdout = String::from_utf8(out.stdout)
                    .expect("rustc error contained non-UTF8 characters.");
                let stderr = String::from_utf8(out.stderr)
                    .expect("rustc error contained non-UTF8 characters.");
                panic!("stdout:\n{stdout}\nstderr:\n{stderr}");
            }
        }
    };
}

#[cfg(test)]
fn backend_path() -> &'static str {
    if cfg!(target_os = "linux") {
        "codegen-backend=../../target/debug/librustc_codegen_clr.so"
    } else if cfg!(target_os = "windows") {
        "codegen-backend=../../target/debug/rustc_codegen_clr.dll"
    } else if cfg!(target_os = "macos") {
        "codegen-backend=../../target/debug/librustc_codegen_clr.dylib"
    } else {
        panic!("Unsupported target OS");
    }
}

test_lib! {binops}
test_lib! {branches}
test_lib! {calls}
test_lib! {casts}
test_lib! {identity}
test_lib! {libc}
test_lib! {nbody}
test_lib! {references}
test_lib! {structs}

test_lib! {types}