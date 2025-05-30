<div style="display: flex, margin: 8px">
    <img src="./screenshot/0-en.png" width="100"/>
    <img src="./screenshot/1-en.png" width="100"/>
    <img src="./screenshot/2-en.png" width="100"/>
    <img src="./screenshot/3-en.png" width="100"/>
    <img src="./screenshot/4-en.png" width="100"/>
    <img src="./screenshot/5-en.png" width="100"/>
</div>

[中文文档](./README.zh-CN.md)
[video](https://youtu.be/cDYzmzq-LKw)

### Introduction
This is a large language model chat client built with Rust and Slint GUI, compatible with OpenAI's API.

- [x] Supports both chat and deep thinking modes
- [x] Supports online Google search
- [x] Automatically saves chat history
- [x] Customizable system prompts
- [x] Supports MCP

### How to build?
- Install `Rust`, `Cargo`, `cargo-apk` and `wasm-pack`
- Run `make desktop-debug` to run it on desktop platform
- Run `make desktop-build-release` to build a release version desktop application
- Refer to [Makefile](./Makefile) for more information

### Troubleshooting
- Using the `Qt backend` can resolve the issue of fuzzy fonts on the Windows platform. It is also recommended to prioritize the `Qt backend` to maintain a consistent build environment with the developers.

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
