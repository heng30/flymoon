<div style="display: flex, margin: 8px">
    <img src="./screenshot/0-cn.png" width="100"/>
    <img src="./screenshot/1-cn.png" width="100"/>
    <img src="./screenshot/2-cn.png" width="100"/>
    <img src="./screenshot/3-cn.png" width="100"/>
    <img src="./screenshot/4-cn.png" width="100"/>
    <img src="./screenshot/5-cn.png" width="100"/>
</div>

[English Documentation](./README.md)
[视频介绍](https://www.bilibili.com/video/BV1i7G2z6ELL)

### 简介
这是一个基于Rust和Slint GUI构建的大语言模型聊天客户端，兼容OpenAI的API。

- [x] 支持聊天和深度思考两种聊天模式,
- [x] 支持联网谷歌搜索,
- [x] 自动保存历史聊天记录,
- [x] 自定义系统提示词
- [x] 支持MCP

### 如何构建?
- 安装 `Rust` 和 `Cargo`
- 运行 `make desktop-debug` 调试桌面平台程序
- 运行 `make desktop-build-release` 编译桌面平台程序
- 参考 [Makefile](./Makefile) 了解更多信息

### 问题排查
- 使用`Qt后端`能解决windows平台字体发虚的问题。也推荐优先使用`Qt后端`保持和开发者相同的构建环境

### 参考
- [Slint Language Documentation](https://slint-ui.com/releases/1.0.0/docs/slint/)
- [slint::android](https://snapshots.slint.dev/master/docs/rust/slint/android/#building-and-deploying)
- [Running In A Browser Using WebAssembly](https://releases.slint.dev/1.7.0/docs/slint/src/quickstart/running_in_a_browser)
- [github/slint-ui](https://github.com/slint-ui/slint)
- [Viewer for Slint](https://github.com/slint-ui/slint/tree/master/tools/viewer)
- [LSP (Language Server Protocol) Server for Slint](https://github.com/slint-ui/slint/tree/master/tools/lsp)
- [developer.android.com](https://developer.android.com/guide)
- [color4bg](https://www.color4bg.com/zh-hans/)
- [How to Deploy Rust Binaries with GitHub Actions](https://dzfrias.dev/blog/deploy-rust-cross-platform-github-actions/)
