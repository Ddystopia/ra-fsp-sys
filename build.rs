use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::{fs, io::Write as _, path};

use build_cfg::{build_cfg, build_cfg_main};
use tera::Tera;

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

fn configure_bsp(ctx: &mut tera::Context) {
    let bsp_cfg = HashMap::<&'static str, u32>::from_iter(
        [
            (
                "mcu_vcc_mv",
                if build_cfg!(feature = "bsp-mcu_vcc_mv_2700") {
                    2700
                } else if build_cfg!(feature = "bsp-mcu_vcc_mv_3300") {
                    3300
                } else if build_cfg!(feature = "bsp-mcu_vcc_mv_5000") {
                    5000
                } else {
                    panic!("Set one of bsp-mcu_vcc_mv_xxxx features!");
                },
            ),
            (
                "stack_main_bytes",
                if build_cfg!(feature = "bsp-stack_main_bytes_4096") {
                    4096
                } else if build_cfg!(feature = "bsp-stack_main_bytes_8192") {
                    8192
                } else if build_cfg!(feature = "bsp-stack_main_bytes_16384") {
                    16384
                } else {
                    panic!("Set one of bsp-stack_main_bytes_xxx features!");
                },
            ),
        ]
        .into_iter(),
    );

    ctx.insert("bsp", &bsp_cfg);
}

fn configure_modules(ctx: &mut tera::Context) {
    let r_flash_hp_cfg = HashMap::<&'static str, u8>::from_iter(
        [
            (
                "code_flash_programming_enable",
                build_cfg!(feature = "mod-r_flash_hp-code_flash_programming_enable") as u8,
            ),
            (
                "data_flash_programming_enable",
                build_cfg!(feature = "mod-r_flash_hp-data_flash_programming_enable") as u8,
            ),
        ]
        .into_iter(),
    );

    ctx.insert("r_flash_hp", &r_flash_hp_cfg);
}

pub fn wrap_component(modules: &[&str]) {
    let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let fsp_cfg_out = out_path.join("fsp_cfg");
    let _ = fs::create_dir(&fsp_cfg_out);

    let src_c = Path::new("src_c");
    let fsp_dir = Path::new("ra-fsp/ra/fsp");
    let fsp_src = fsp_dir.join("src");
    let include_dirs = vec![
        Path::new("fsp_cfg/bsp").to_path_buf(),
        Path::new("fsp_cfg").to_path_buf(),
        fsp_cfg_out.to_path_buf(),
        Path::new("ra_gen").to_path_buf(),
        Path::new("cmsis/CMSIS/Core/Include/").to_path_buf(),
        fsp_dir.join("inc"),
        fsp_dir.join("inc/api"),
        fsp_dir.join("inc/instances"),
    ];

    // render configuration files

    let tera = Tera::new("fsp_cfg/**/*.h.in").expect("Can't parse templates in fsp_cfg/*.h.in");
    let mut ctx = tera::Context::new();

    configure_bsp(&mut ctx);
    configure_modules(&mut ctx);

    eprintln!("ctx = {ctx:?}");
    for tmpl in tera.get_template_names() {
        eprintln!("templates = {tmpl}");
    }

    for module in ["bsp"].iter().chain(modules.iter()) {
        let tmpl_prefix = if module.starts_with("bsp") {
            "bsp/"
        } else {
            ""
        };
        let tmpl = format!("{tmpl_prefix}{module}_cfg.h.in",);

        // not all modules have configuration templates
        if tera.get_template_names().find(|t| **t == tmpl).is_some() {
            let tmpl_out = format!("{module}_cfg.h");

            let cfg = tera
                .render(&tmpl, &ctx)
                .expect(&format!("Error rendering {tmpl}"));

            fs::write(fsp_cfg_out.join(tmpl_out), cfg)
                .expect("Error writing module configuration file");
        }
    }

    // compile fsp library

    let bsp_stems = [
        "bsp_clocks",
        "bsp_delay",
        "bsp_irq",
        "bsp_common",
        "bsp_register_protection",
    ];
    let mut build = cc::Build::new();

    walkdir::WalkDir::new(&fsp_src)
        .follow_links(true)
        .into_iter()
        .map(|e| e.expect("Can't walk in fsp src"))
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.file_name().to_str().unwrap().ends_with(".c"))
        .filter(|e| {
            let stem = e.path().file_stem().unwrap().to_str().unwrap();
            bsp_stems.contains(&stem) || modules.iter().find(|m| stem == **m).is_some()
        })
        .for_each(|e| {
            build.file(e.path());
        });

    // add custom system.c
    let system_c = src_c.join("system.c");
    build.file(&system_c);
    println!("cargo:rerun-if-changed={}", system_c.to_str().unwrap());

    build.includes(&include_dirs).compile("fsp");

    // summarize modules enabled by the features to wrapper header

    // let mut re_derive_const_default = bindgen::RegexSet::new();
    // for prefix in allow_prefixes {
    //     re_derive_const_default.insert(String::from(format!("{}_.*", prefix.to_uppercase())));
    // }
    let wrapper = out_path.join("fsp_wrapper.h");

    let mut wrapper_h = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&wrapper)
        .expect("Can't create fsp_wrapper.h");

    for header in ["bsp_api", "fsp_common_api"] {
        writeln!(wrapper_h, "#include <{header}.h>").expect("Error writing fsp_wrapper.h");

        // Tell cargo to invalidate the built crate whenever the wrapper changes
        // println!("cargo:rerun-if-changed={}", wrapper.to_str().unwrap());
    }

    for module in modules {
        writeln!(wrapper_h, "#include <{module}.h>").expect("Error writing fsp_wrapper.h");

        // Tell cargo to invalidate the built crate whenever the wrapper changes
        // println!("cargo:rerun-if-changed={}", wrapper.to_str().unwrap());
    }

    writeln!(wrapper_h, "void fsp_init();").unwrap();

    drop(wrapper_h);

    // generate bindings
    let mut builder = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header(wrapper.to_str().unwrap())
        .use_core()
        .clang_args(
            include_dirs
                .iter()
                .map(|d| path::absolute(d).expect("Can't resolve absolute path"))
                .map(|e| format!("-I{}", e.as_os_str().to_str().unwrap())),
        )
        /*
        .clang_args(
            compile_defs
                .trim_start_matches("[")
                .trim_end_matches("]")
                .split(';')
                .map(|e| format!("-D{e}")),
        )
        */
        ;

    builder = builder
        .derive_default(true)
        // .derive_debug(true)
        // .derive_partialeq(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    // Tell cargo to invalidate the built crate whenever any of the
    // included header files changed.

    let bindings = builder
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_path.join(format!("ra-fsp.rs")))
        .expect("Couldn't write bindings!");
}

macro_rules! add_module {
    ($modules:expr,$module_feat:expr) => {
        if build_cfg!(feature = $module_feat) {
            $modules.push(&$module_feat[4..]);
        }
    };
}

#[build_cfg_main]
fn main() {
    let mut modules = Vec::<&'static str>::new();

    add_module!(modules, "mod-r_icu");
    add_module!(modules, "mod-r_flash_hp");

    wrap_component(&modules);
}
