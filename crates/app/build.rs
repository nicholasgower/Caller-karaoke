//! Build helper for the optional Kokoro neural voice.
//!
//! ONNX Runtime is loaded dynamically on Windows so the application can keep
//! its self-contained static-CRT build. The runtime DLL is fetched at build
//! time, verified, and embedded in the executable; it is written to the local
//! cache on first use. Linux downloads its runtime at first use, and Android
//! packages it from the pinned ONNX Runtime AAR in `packaging/android`, so this
//! script does nothing there.

use std::env;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

const ONNX_RUNTIME_ZIP: &str = "https://github.com/microsoft/onnxruntime/releases/download/v1.31.0/onnxruntime-win-x64-1.31.0.zip";
const ONNX_RUNTIME_ZIP_SHA256: &str =
    "3958d8a44984160692c1b4ffd0de3cb48f8c39f9d34bb31e433db06e30b82195";
const ONNX_RUNTIME_DLL: &str = "onnxruntime-win-x64-1.31.0/lib/onnxruntime.dll";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/icons/caller-karaoke.ico");
    println!("cargo:rerun-if-env-changed=CALLER_ONNX_RUNTIME_DLL");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_os == "android" {
        // Android 15+ devices may use 16 KB pages. The ONNX Runtime binary from
        // the AAR is built for that page size; align the app library to match.
        println!("cargo:rustc-link-arg=-Wl,-z,max-page-size=16384");
        println!("cargo:rustc-cdylib-link-arg=-Wl,-z,max-page-size=16384");
    }
    if target_os == "windows" {
        embed_windows_resources();
    }
    if target_os != "windows" || target_arch != "x86_64" {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set"));
    let destination = out_dir.join("onnxruntime.dll");
    if destination.exists() {
        return;
    }

    if let Ok(path) = env::var("CALLER_ONNX_RUNTIME_DLL") {
        let source = PathBuf::from(path);
        fs::copy(&source, &destination).unwrap_or_else(|error| {
            panic!(
                "could not copy CALLER_ONNX_RUNTIME_DLL from {}: {error}",
                source.display()
            )
        });
        return;
    }

    let bytes = download().unwrap_or_else(|error| {
        panic!(
            "could not download ONNX Runtime {version} for the Kokoro voice: {error}\n\
             Set CALLER_ONNX_RUNTIME_DLL to a local onnxruntime.dll to build offline.",
            version = "1.31.0"
        )
    });
    if sha256_hex(&bytes) != ONNX_RUNTIME_ZIP_SHA256 {
        panic!("ONNX Runtime download did not match the pinned SHA-256");
    }
    extract_dll(&bytes, &destination)
        .unwrap_or_else(|error| panic!("could not extract {}: {error}", ONNX_RUNTIME_DLL));
}

fn download() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut response = ureq::get(ONNX_RUNTIME_ZIP)
        .header("User-Agent", "caller-karaoke-build")
        .call()?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(128 * 1024 * 1024)
        .read_to_vec()?;
    Ok(bytes)
}

fn extract_dll(bytes: &[u8], destination: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut file = archive.by_name(ONNX_RUNTIME_DLL)?;
    let mut output = fs::File::create(copy_path(destination))?;
    std::io::copy(&mut file, &mut output)?;
    output.flush()?;
    fs::rename(copy_path(destination), destination)?;
    Ok(())
}

fn copy_path(destination: &Path) -> PathBuf {
    let mut path = destination.as_os_str().to_owned();
    path.push(".part");
    PathBuf::from(path)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Embed the app icon and version metadata in the Windows executable.
///
/// Explorer and the taskbar read the icon from the binary itself, so a pinned
/// or portable copy shows the mark even though the running window also sets
/// its own. `rc.exe` ships with the Windows SDK that rustc links against; if
/// it cannot be found the build continues without the resources rather than
/// failing, so a minimal toolchain still produces a working binary.
#[cfg(windows)]
fn embed_windows_resources() {
    let mut resource = winres::WindowsResource::new();
    resource.set_icon("../../assets/icons/caller-karaoke.ico");
    resource.set("FileDescription", "Contra dance caller trainer");
    resource.set("ProductName", "Caller Karaoke");
    resource.set("LegalCopyright", "MIT licensed");
    if let Err(error) = resource.compile() {
        println!("cargo:warning=Windows icon and metadata not embedded: {error}");
    }
}

/// Cross-compiling to Windows from another host leaves the resources to the
/// toolchain that links the binary.
#[cfg(not(windows))]
fn embed_windows_resources() {}
