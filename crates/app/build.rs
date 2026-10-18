use std::path::PathBuf;

fn main() {
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=tapwheel.manifest");
	// the icon is drawn by the ui crate; redraw it whenever the mark changes
	println!("cargo:rerun-if-changed=../ui/src/icon.rs");
	if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
		return;
	}
	let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
	let ico = out.join("tapwheel.ico");
	std::fs::write(&ico, tapwheel_ui::icon::ico(&[16, 20, 24, 32, 40, 48, 64, 256])).unwrap();

	let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("tapwheel.manifest");
	let version = std::env::var("CARGO_PKG_VERSION").unwrap();
	let parts: Vec<&str> = version.split('.').collect();
	let comma = format!("{},{},{},0", parts[0], parts[1], parts[2]);
	let rc = format!(
		r#"#include <winver.h>
1 ICON "{ico}"
1 24 "{manifest}"
VS_VERSION_INFO VERSIONINFO
FILEVERSION {comma}
PRODUCTVERSION {comma}
FILEOS VOS_NT_WINDOWS32
FILETYPE VFT_APP
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904b0"
    BEGIN
      VALUE "CompanyName", "Jax (muxover)"
      VALUE "FileDescription", "TapWheel"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "tapwheel"
      VALUE "LegalCopyright", "Jax (muxover), GPL-3.0-or-later"
      VALUE "OriginalFilename", "tapwheel.exe"
      VALUE "ProductName", "TapWheel"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
		ico = ico.display().to_string().replace('\\', "\\\\"),
		manifest = manifest.display().to_string().replace('\\', "\\\\"),
	);
	let rc_path = out.join("tapwheel.rc");
	std::fs::write(&rc_path, rc).unwrap();
	embed_resource::compile(&rc_path, embed_resource::NONE)
		.manifest_required()
		.unwrap();
}
