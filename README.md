# RedShift

A Windows desktop application that provides a customizable red overlay for reduced eye strain.

If you just want to use the application, you can download the latest release from the [Releases](https://github.com/The-Red-Shift/RedShift/releases) page.
Since this application is not digitally signed you will need to expaned 'more info' when installing to continue
This app is 100% open source and the bulk of the application resides in /src

!!! This app will only run on windows due to the usage of the windows api

## Features
- Adjustable red overlay intensity
- System tray integration
- Launch on startup option
- Minimal GUI interface

## Requirements
- Windows OS
- RUST (for building from source)

## Building from Source
1. Clone the repository
2. Ensure you have Rust installed
3. Run `cargo build --release`
4. The executable will be in `target/release/RedShift.exe`

## License
[MIT License](LICENSE)
