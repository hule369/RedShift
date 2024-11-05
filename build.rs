use winres::WindowsResource;

fn main() {
    if cfg!(target_os = "windows") {
        WindowsResource::new()
            .set_icon("G:/Cursor Projects/RedShiftBundle/redshiftbundle/assets/RSICONICO.ico")  // Using forward slashes
            // Alternative using backslashes:
            // .set_icon(r"G:\Cursor Projects\RedShiftBundle\redshiftbundle\assets\RSICONICO.ico")
            .compile()
            .expect("Failed to compile Windows resource");
    }
}