use std::{
    env::var,
    path::PathBuf,
};
use winres::WindowsResource;

fn main() {
    let target_os = var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let path_base: PathBuf = var("CARGO_MANIFEST_DIR").unwrap().into();
    let path_icon = path_base.join("resources").join("app.ico");
    let path_manifest = path_base.join("resources").join("app.manifest");
    let mut res = WindowsResource::new();
    res.set_icon(path_icon.to_string_lossy().as_ref());
    res.set_manifest_file(path_manifest.to_string_lossy().as_ref());

    let lang_english: u16 = 0x09;                                 //primary language
    let sublang_english_us: u16 = 0x01;                           //sublanguage
    let langid: u16 = (sublang_english_us << 10) | lang_english;  // 0x0409 = English (United States) - C:\Program Files (x86)\Windows Kits\10\Include\10.0.28000.0\um\winnt.h
    res.set_language(langid);

    //-- building VERSIONINFO
    res.set("Comments", "https://github.com/eladkarako/json_formatter");
    res.set("FileDescription", "json in, json out, beautified-minified, natural deep sort. stdin/file-stdout-file.");
    res.set("InternalName", "json_formatter.exe");
    res.set("OriginalFilename", "json_formatter.exe");
    res.set("CompanyName", "Elad Karako");
    res.set("LegalCopyright", "https://github.com/eladkarako/json_formatter/LICENSE");
    res.set("ProductName", "json_formatter");

    //--------------------------  remember to change version in Cargo.toml's version under [package]
    res.set("FileVersion", "26.8.31.0");
    res.set("ProductVersion", "26.8.31.0");

    let major: u64 = 26;
    let minor: u64 = 8;
    let patch: u64 = 31;
    let release: u64 = 0;
    let packed = (major << 48) | (minor << 32) | (patch << 16) | release;
    res.set_version_info(winres::VersionInfo::FILEVERSION, packed);
    res.set_version_info(winres::VersionInfo::PRODUCTVERSION, packed);


    // note: only if you need '#include <windows.h>' in the .rc file, uncomment the following lines (modify path to where to find windows.h on your computer). needs to be before .compile() or calls to 'rc.exe' .
    // let include_value = r"C:\Program Files (x86)\Windows Kits\10\Include\10.0.28000.0\um;C:\Program Files (x86)\Windows Kits\10\Include\10.0.28000.0\shared";
    // unsafe { set_var("INCLUDE", include_value); }
    // println!("cargo:rustc-link-search=native={}", include_value);

    res.compile().unwrap();
}
