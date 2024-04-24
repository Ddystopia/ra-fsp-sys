use std::collections::HashMap;
use std::env;
use std::io::BufRead;
use std::path::PathBuf;

pub fn load_env<P: AsRef<std::path::Path>>(path: P) -> HashMap<String, String> {
    let mut res = HashMap::new();

    let env_file = std::fs::File::open(path.as_ref())
        .expect(format!("file '{}' must exist", path.as_ref().to_str().unwrap()).as_str());
    let reader = std::io::BufReader::new(env_file);
    for line in reader.lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.splitn(2, "=").collect();
        if parts.len() == 2 {
            res.insert(
                parts[0].to_string(),
                parts[1].replace(";;", ";").to_string(),
            );
        } else {
            eprintln!("Skip invalid line '{}' with {} parts", line, parts.len());
        }
    }

    res
}

pub fn wrap_component(
    env: &HashMap<String, String>,
    component: &str,
    // allow_prefixes: &[&str],
    // derive_const_default: Option<&[&str]>,
    // autoselect_link_variant: bool,
) {
    let wrapper = format!("{component}_wrapper.h");

    // Tell cargo to look for shared libraries in the specified directory
    //println!("cargo:rustc-link-search=/path/to/lib");

    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    //println!("cargo:rustc-link-lib=bz2");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed={wrapper}");

    let include_dirs = env
        .get("ra_fsp_include_dirs")
        .expect("env var 'ra_fsp_include_dirs' required")
        .replace(";;", ";");
    let compile_defs = env
        .get("ra_fsp_compile_defs")
        .expect("env var 'ra_fsp_compile_defs' required");

    // let mut re_derive_const_default = bindgen::RegexSet::new();
    // for prefix in allow_prefixes {
    //     re_derive_const_default.insert(String::from(format!("{}_.*", prefix.to_uppercase())));
    // }

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let mut builder = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(wrapper)
        .use_core()
        .clang_args(
            include_dirs
                .trim_start_matches("[")
                .trim_end_matches("]")
                .split(";")
                .map(|e| format!("-I{e}")),
        )
        .clang_args(
            compile_defs
                .trim_start_matches("[")
                .trim_end_matches("]")
                .split(';')
                .map(|e| format!("-D{e}")),
        );

    // if !allow_prefixes.is_empty() {
    //     for prefix in allow_prefixes {
    //         builder = builder
    //             .allowlist_function(format!("^_?{}[de]?_.*", prefix.to_lowercase()))
    //             .allowlist_var(format!("^{}_.*", prefix.to_uppercase()));
    //     }
    // }

    builder = builder
        .derive_default(true)
        // .derive_debug(true)
        // .derive_partialeq(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    // if autoselect_link_variant {
    //     builder = builder.parse_callbacks(Box::new(FuncRenameCallback::new()));
    // }

    // if let Some(seed) = derive_const_default {
    //     seed.iter()
    //         .for_each(|x| re_derive_const_default.insert(format!("^{x}.*")));

    //     re_derive_const_default.build(true);

    //     builder = builder.parse_callbacks(Box::new(DeriveCallback::new(
    //         vec![String::from("const_default::ConstDefault")],
    //         Some(callbacks::TypeKind::Struct),
    //         re_derive_const_default,
    //     )));
    // }
    // Tell cargo to invalidate the built crate whenever any of the
    // included header files changed.

    let bindings = builder
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join(format!("{component}.rs")))
        .expect("Couldn't write bindings!");
}

fn main() {
    let env =
        load_env(env::var("RA_FSP_ENV").expect("RA_FSP_ENV environment variable must be set"));
    wrap_component(&env, "ra_fsp");
}
