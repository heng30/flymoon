<!-- <div style="display: flex, margin: 8px"> -->
    <!-- <img src="./screenshot/1-en.png" width="100"/> -->
    <!-- <img src="./screenshot/2-en.png" width="100"/> -->
    <!-- <img src="./screenshot/3-en.png" width="100"/> -->
    <!-- <img src="./screenshot/4-en.png" width="100"/> -->
    <!-- <img src="./screenshot/5-en.png" width="100"/> -->
    <!-- <img src="./screenshot/6-en.png" width="100"/> -->
    <!-- <img src="./screenshot/7-en.png" width="100"/> -->
<!-- </div> -->

[中文文档](./README.zh-CN.md)

### Introduction
It's a dead simple chatbot based on Rust and Slint-ui.

### How to build?
- Install `Rust`, `Cargo`, `cargo-apk` and `wasm-pack`
- Run `make desktop-debug` to run it on desktop platform
- Run `make desktop-build-release` to build a release version desktop application
- Refer to [Makefile](./Makefile) for more information

### Reference
- [Slint Language Documentation](https://slint-ui.com/releases/1.0.0/docs/slint/)
- [slint::android](https://snapshots.slint.dev/master/docs/rust/slint/android/#building-and-deploying)
- [Running In A Browser Using WebAssembly](https://releases.slint.dev/1.7.0/docs/slint/src/quickstart/running_in_a_browser)
- [github/slint-ui](https://github.com/slint-ui/slint)
- [Viewer for Slint](https://github.com/slint-ui/slint/tree/master/tools/viewer)
- [LSP (Language Server Protocol) Server for Slint](https://github.com/slint-ui/slint/tree/master/tools/lsp)
- [developer.android.com](https://developer.android.com/guide)
- [color4bg](https://www.color4bg.com/zh-hans/)
- [How to Deploy Rust Binaries with GitHub Actions](https://dzfrias.dev/blog/deploy-rust-cross-platform-github-actions/)
