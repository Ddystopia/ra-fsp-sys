use std::{
    collections::HashMap,
    fs,
    io::{BufRead, Write},
    path::{Path, PathBuf},
};

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
        "system",
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

    // add custom init.c
    let system_c = src_c.join("init.c");
    build.file(&system_c);
    println!("cargo:rerun-if-changed={}", system_c.to_str().unwrap());

    let mut rust_codegen = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(out_path.join("ra-fsp.rs"))
        .unwrap();

    if let Some(mcu_group) = mcu_group() {
        build
            .includes(&include_dirs)
            .define(mcu_group, Some("1"))
            .compile("bare-fsp");
    } else {
        write!(rust_codegen, "compile_error!(\"No MCU group defined\");").unwrap();
    }

    println!("cargo:rustc-link-lib=static=bare-fsp");

    let linker_script = fs::read_to_string("ra-fsp-sys.x.in").unwrap();
    println!("cargo:rerun-if-changed=ra-fsp-sys.x.in");

    // Put the linker script somewhere the linker can find it
    fs::write(out_path.join("ra-fsp-sys.x"), linker_script).unwrap();
    println!("cargo:rustc-link-search={}", out_path.display());
}

fn mcu_group() -> Option<&'static str> {
    Some(if cfg!(feature = "ra0e1") {
        "BSP_MCU_GROUP_RA0E1"
    } else if cfg!(feature = "ra2a1") {
        "BSP_MCU_GROUP_RA2A1"
    } else if cfg!(feature = "ra2a2") {
        "BSP_MCU_GROUP_RA2A2"
    } else if cfg!(feature = "ra2e1") {
        "BSP_MCU_GROUP_RA2E1"
    } else if cfg!(feature = "ra2e2") {
        "BSP_MCU_GROUP_RA2E2"
    } else if cfg!(feature = "ra2e3") {
        "BSP_MCU_GROUP_RA2E3"
    } else if cfg!(feature = "ra2l1") {
        "BSP_MCU_GROUP_RA2L1"
    } else if cfg!(feature = "ra4e1") {
        "BSP_MCU_GROUP_RA4E1"
    } else if cfg!(feature = "ra4e2") {
        "BSP_MCU_GROUP_RA4E2"
    } else if cfg!(feature = "ra4m1") {
        "BSP_MCU_GROUP_RA4M1"
    } else if cfg!(feature = "ra4m2") {
        "BSP_MCU_GROUP_RA4M2"
    } else if cfg!(feature = "ra4m3") {
        "BSP_MCU_GROUP_RA4M3"
    } else if cfg!(feature = "ra4t1") {
        "BSP_MCU_GROUP_RA4T1"
    } else if cfg!(feature = "ra4w1") {
        "BSP_MCU_GROUP_RA4W1"
    } else if cfg!(feature = "ra6e1") {
        "BSP_MCU_GROUP_RA6E1"
    } else if cfg!(feature = "ra6e2") {
        "BSP_MCU_GROUP_RA6E2"
    } else if cfg!(feature = "ra6m1") {
        "BSP_MCU_GROUP_RA6M1"
    } else if cfg!(feature = "ra6m2") {
        "BSP_MCU_GROUP_RA6M2"
    } else if cfg!(feature = "ra6m3") {
        "BSP_MCU_GROUP_RA6M3"
    } else if cfg!(feature = "ra6m4") {
        "BSP_MCU_GROUP_RA6M4"
    } else if cfg!(feature = "ra6m5") {
        "BSP_MCU_GROUP_RA6M5"
    } else if cfg!(feature = "ra6t1") {
        "BSP_MCU_GROUP_RA6T1"
    } else if cfg!(feature = "ra6t2") {
        "BSP_MCU_GROUP_RA6T2"
    } else if cfg!(feature = "ra6t3") {
        "BSP_MCU_GROUP_RA6T3"
    } else if cfg!(feature = "ra8m1") {
        "BSP_MCU_GROUP_RA8M1"
    } else if cfg!(feature = "ra8d1") {
        "BSP_MCU_GROUP_RA8D1"
    } else if cfg!(feature = "ra8t1") {
        "BSP_MCU_GROUP_RA8T1"
    } else {
        return None;
    })
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
