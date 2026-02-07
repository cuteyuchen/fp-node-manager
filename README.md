# Frontend Project Manager

A modern, cross-platform desktop application for managing frontend projects and Node.js versions. Built with Tauri + Vue 3 + TypeScript.

## Features

### Node Version Management

Visualize your NVM versions. Install, uninstall, and switch Node.js versions with ease.
![Node Management](docs/images/image-1.png)

Set your system's default Node.js version with a single click. It automatically handles environment variables for you.
![Set Default](docs/images/image-2.png)

### Project Management

Organize all your frontend projects in one place. Run scripts (`dev`, `build`, etc.) directly from the UI.
![Project List](docs/images/image-3.png)

Easily add projects by selecting their folder. The app automatically detects `package.json` scripts.
![Add Project](docs/images/image-4.png)

### Settings & Customization

Customize your experience:

- **Theme**: Dark / Light / System
- **Language**: English / Chinese
- **Editor**: Configure your preferred editor (e.g., VS Code) to open projects.
- **Terminal**: Choose between CMD, PowerShell, or Git Bash.
![Settings](docs/images/image-5.png)

## Tech Stack

- **Core**: Tauri v2, Rust
- **Frontend**: Vue 3, TypeScript, Vite
- **UI**: Element Plus, Tailwind CSS (UnoCSS)
- **State**: Pinia

## Getting Started

1. Clone the repository.
2. Install dependencies: `npm install`.
3. Run development server: `npm run tauri dev`.
4. Build for production: `npm run tauri build`.

## License

MIT
