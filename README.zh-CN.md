# 前端项目管理器 (Frontend Project Manager)

一个现代化的跨平台桌面应用，用于管理前端项目和 Node.js 版本。基于 Tauri + Vue 3 + TypeScript 构建。

## 功能特性

- **项目管理**：轻松组织和访问您的前端项目。
- **Node 版本管理**：为每个项目切换 Node.js 版本（支持 NVM）。
- **脚本运行器**：直接从 UI 运行 `npm` 脚本。
- **控制台视图**：在干净、可滚动的界面中查看日志。
- **主题与国际化**：自定义主题颜色和多语言支持（中文/英文）。

## 技术栈

- **核心**：Tauri v2, Rust
- **前端**：Vue 3, TypeScript, Vite
- **UI**：Element Plus, Tailwind CSS (UnoCSS)
- **状态管理**：Pinia

## 快速开始

1. 克隆仓库。
2. 安装依赖：`npm install`。
3. 运行开发服务器：`npm run tauri dev`。
4. 构建生产版本：`npm run tauri build`。

## 开源协议

MIT
