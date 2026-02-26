# Lia, more than an agent

Lia is not a traditional chatbot; it is a **Proactive and Multimodal Programming Companion**. Designed to break the barrier between the browser and the development environment, Lia acts as a "Pair Programmer" that can see your screen, hear your voice, and read your code in real-time to offer deep contextual assistance.

## Monorepo Architecture

The system is divided into three fundamental pillars that communicate asynchronously:

1. **`lia-client` (The Body - Rust/Tauri & React):** Heavy desktop application. Manages the floating interface (HUD) and Lia's senses: screen capture (`xcap`) and audio (`cpal`).
2. **`lia-vscode` (The Touch - TypeScript):** Visual Studio Code extension that extracts code context in real-time and sends it to the local client via WebSockets.
3. **`lia-cloud` (The Brain - Python/FastAPI):** *(In development)* Cloud backend that processes the multimodal request using Gemini.

---

## Prerequisites

To run Lia's local infrastructure, you need to have installed:

* [Node.js](https://nodejs.org/) (v18 or higher)
* [Rust and Cargo](https://rustup.rs/)
* **Operating System Dependencies (Linux/Ubuntu):**
  Lia interacts directly with video and audio hardware. You must install the underlying C libraries:
  ```bash
  sudo apt update
  sudo apt install pkg-config libasound2-dev libx11-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
(Note: If you use Windows or macOS, the Rust compilation will handle native dependencies automatically through system APIs).

---

## Installation

1. Clone the repository:
    ```bash
    git clone https://github.com/alexis-campos/LIA-more-than-an-agent.git
    cd lia-monorepo
2. Install client (Desktop) dependencies:
    ```bash
    cd lia-client
    npm install
    # Install the Tauri CLI locally
    npm install --save-dev @tauri-apps/cli
3. Install Extension (VS Code) dependencies:
    ```bash
    cd ../lia-vscode
    npm install

## How to run the proyect (Development environment)
To make the system work, you must start both parts (The Body and The Touch) simultaneously.
1. Start the Local Brain (Tauri/Rust):
Open a terminal, navigate to the client folder, and run the engine:
    ```bash
    cd lia-client
    npm run tauri dev
The first time will take several minutes while Rust compiles the audio and video libraries. Upon completion, you will see in the console that the WebSocket server is listening on port 3333.

2. Connect the Editor (VS Code)
Open a new VS Code window and load only the extension folder:
    1. In VS Code: File -> Open Folder -> Select lia-vscode.
    2. Press the F5 key to start debugging.
    3. A secondary VS Code window ("Extension Development Host") will open.
    4. Check your Tauri terminal: you should see the confirmation message that the extension successfully connected to the WebSocket bridge.

## Current Roadmap
- [x] Phase 0: Monorepo setup and local WebSocket bridge.
- [x] Phase 1: The Senses (Screen capture with xcap and microphone with cpal).
- [ ] Phase 2: The Touch (Dynamic context extraction in VS Code).
- [ ] Phase 3: Sentinel (Privacy filter and Regex sanitization).
- [ ] Phase 4: The Brain (FastAPI and Vertex AI).
