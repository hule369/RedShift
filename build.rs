use winres::WindowsResource;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = WindowsResource::new();
        res.set_icon("G:/Cursor Projects/RedShiftBundle/redshiftbundle/assets/RSICONICO.ico")
            .set_manifest_file("app.manifest")
            .set_version_info(winres::VersionInfo::PRODUCTVERSION, 0x0001000000000000)
            .set_version_info(winres::VersionInfo::FILEVERSION, 0x0001000000000000);

        res.set("FileDescription", "RedShift Blue light blocker")
            .set("ProductName", "RedShift")
            .set("FileVersion", "1.0.0")
            .set("LegalCopyright", "© 2024")
            .set("CompanyName", "RedShift");

        res.compile().expect("Failed to compile Windows resource");
    }
}