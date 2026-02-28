# Lia, more than an agent

Lia is not a traditional chatbot; it is a **Proactive and Multimodal Programming Companion**. Designed to break the barrier between the browser and the development environment, Lia acts as a "Pair Programmer" that can see your screen, hear your voice, and read your code in real-time to offer deep contextual assistance.

## Monorepo Architecture

The system is divided into three fundamental pillars that communicate asynchronously:

1. **`lia-client` (The Body - Rust/Tauri & React):** Heavy desktop application. Manages the floating interface (HUD), Lia's senses (screen capture with `xcap`, microphone recording with `cpal`/`hound`), audio playback (`rodio`), the privacy guard (Sentinel), and smart caching (SHA-256 hashing).
2. **`lia-vscode` (The Touch - TypeScript):** Visual Studio Code extension that extracts code context in real-time (±50 lines around the cursor) and sends it to the local client via WebSockets with debounce.
3. **`lia-cloud` (The Brain - Python/FastAPI):** Cloud backend that receives multimodal requests from the Rust client, transcribes speech (Google STT), resolves a volatile LRU cache, processes them through Gemini 1.5 Pro via Vertex AI with streaming responses, and synthesizes voice replies (Google TTS).

---

## Prerequisites

To run Lia's full infrastructure, you need:

* [Node.js](https://nodejs.org/) (v18 or higher)
* [Rust and Cargo](https://rustup.rs/)
* [Python 3.12+](https://www.python.org/)
* [Google Cloud CLI](https://cloud.google.com/sdk/docs/install) (with Vertex AI API enabled)
* **Operating System Dependencies (Linux/Ubuntu):**
  Lia interacts directly with video and audio hardware. You must install the underlying C libraries:
  ```bash
  sudo apt update
  sudo apt install pkg-config libasound2-dev libx11-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
  ```
  (Note: If you use Windows or macOS, the Rust compilation will handle native dependencies automatically through system APIs).

---

## Installation

1. Clone the repository:
    ```bash
    git clone https://github.com/alexis-campos/LIA-more-than-an-agent.git
    cd lia-monorepo
    ```
2. Install client (Desktop) dependencies:
    ```bash
    cd lia-client
    npm install
    npm install --save-dev @tauri-apps/cli
    ```
3. Install Extension (VS Code) dependencies:
    ```bash
    cd ../lia-vscode
    npm install
    ```
4. Install Cloud (Python) dependencies:
    ```bash
    cd ../lia-cloud
    python3 -m venv venv
    source venv/bin/activate
    pip install -r requirements.txt
    ```
5. Configure Google Cloud credentials:
    ```bash
    gcloud auth login
    gcloud config set project YOUR_PROJECT_ID
    gcloud services enable aiplatform.googleapis.com
    gcloud auth application-default login
    ```

---

## How to Run (Development)

To make the full system work, you must start all three parts simultaneously.

### 1. Start the Cloud Brain (Python/FastAPI)
```bash
cd lia-cloud
source venv/bin/activate
python main.py
```
The server will start on `http://0.0.0.0:8000`. Verify with `curl http://127.0.0.1:8000/health`.

### 2. Start the Local Body (Tauri/Rust)
```bash
cd lia-client
npm run tauri dev
```
The first time will take several minutes while Rust compiles. Upon completion, you will see in the console that the WebSocket server is listening on `ws://127.0.0.1:3333/ws`.

### 3. Connect the Editor (VS Code Extension)
1. Open a new VS Code window: **File -> Open Folder -> Select `lia-vscode`**.
2. Press **F5** to start debugging.
3. A secondary VS Code window ("Extension Development Host") will open.
4. The extension will automatically connect to the Rust client and begin sending real-time context updates (file name, cursor line, ±50 lines of code).

---

## Project Structure

```
lia-monorepo/
├── lia-client/              # Desktop app (Rust/Tauri + React)
│   ├── src-tauri/src/
│   │   ├── main.rs          # WebSocket server + Tauri event bridge + dynamic port
│   │   ├── context.rs       # Shared state (Arc<Mutex>) for VS Code context
│   │   ├── orchestrator.rs  # Global state machine (IDLE/LISTENING/THINKING/RESPONDING)
│   │   ├── wakeword.rs      # Voice Activity Detection (RMS energy)
│   │   ├── sentinel.rs      # DLP: regex-based secret sanitization
│   │   ├── hasher.rs        # SHA-256 for smart caching
│   │   ├── request.rs       # Contract B builder (multimodal request)
│   │   ├── vision.rs        # Screen capture + multi-monitor (xcap)
│   │   ├── audio.rs         # Mic recording + WAV + echo cancellation (cpal/hound)
│   │   └── playback.rs      # Audio playback for TTS with echo flag (rodio)
│   └── src/
│       ├── App.tsx          # Root HUD component (Tauri event listeners)
│       ├── App.css          # Glassmorphism styles
│       └── components/
│           ├── StatusOrb.tsx    # Animated state indicator (Framer Motion)
│           ├── StreamingText.tsx # Real-time Gemini response display
│           └── ContextBar.tsx   # Current file/line/language bar
├── lia-vscode/              # VS Code extension (TypeScript)
│   └── src/
│       └── extension.ts     # Context extraction + debounce + exponential backoff + port discovery
├── lia-cloud/               # Cloud backend (Python/FastAPI)
│   ├── main.py              # FastAPI + WebSocket /ws/lia
│   ├── config.py            # Environment variables
│   ├── cache.py             # Volatile LRU cache (RAM only, 15min TTL)
│   ├── inference.py         # Gemini 1.5 Pro via Vertex AI (streaming)
│   ├── stt.py               # Speech-to-Text (Google Cloud Speech)
│   └── tts.py               # Text-to-Speech (Google Cloud TTS / WaveNet)
└── README.md
```

---

## Current Roadmap
- [x] Phase 0: Monorepo setup and local WebSocket bridge.
- [x] Phase 1: The Senses (Screen capture with xcap and microphone with cpal).
- [x] Phase 2: The Touch (Dynamic context extraction in VS Code with debounce).
- [x] Phase 3: Sentinel (Privacy filter, Regex sanitization, SHA-256 hashing, Contract B packaging).
- [x] Phase 4: The Brain (FastAPI backend with Vertex AI / Gemini 1.5 Pro streaming).
- [x] Phase 5: The Voice (Real-time STT/TTS audio pipeline).
- [x] Phase 6: The HUD (Transparent floating UI with React + Framer Motion).
- [x] Phase 7: Advanced Magic (Echo cancellation, Wake Word, Multi-monitor, Resilience).
