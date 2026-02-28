# Lia — More Than an Agent

**Lia** is a **Proactive and Multimodal Programming Companion**. She acts as a real-time "Pair Programmer" that can see your screen, hear your voice, and read your code — offering deep contextual assistance powered by Gemini 1.5 Pro.

Unlike traditional chatbots, Lia bridges the gap between the IDE and the AI by combining screen capture, live code analysis, voice interaction, and a transparent floating HUD — all running locally on your desktop while offloading inference to the cloud.

---

## Architecture

The system follows a three-pillar architecture communicating asynchronously via WebSockets:

| Pillar | Technology | Role |
|---|---|---|
| **`lia-client`** | Rust / Tauri + React | Desktop app. Screen capture (`xcap`), microphone (`cpal`/`hound`), audio playback (`rodio`), privacy guard (Sentinel DLP), smart caching (SHA-256), state machine, and floating HUD with glassmorphism |
| **`lia-vscode`** | TypeScript | VS Code extension. Extracts live code context (±50 lines around cursor) with debounce, exponential backoff reconnection, and dynamic port discovery |
| **`lia-cloud`** | Python / FastAPI | Cloud backend. Receives multimodal requests, transcribes speech (Google STT), resolves an LRU cache, streams responses from Gemini 1.5 Pro (Vertex AI), and synthesizes voice replies (Google TTS / WaveNet) |

### Data Flow

```
VS Code ──(Contract A)──► Rust Client ──(Contract B)──► Cloud Python
                              │                              │
                              │  ◄──────(Contract C)─────────┘
                              │       Streaming response
                              ▼
                          React HUD (Glassmorphism)
```

- **Contract A:** Real-time editor context (file, cursor, ±50 lines)
- **Contract B:** Multimodal request (sanitized code + screen capture + audio, with smart caching)
- **Contract C:** Streaming response from Gemini (text chunks + optional TTS audio)

---

## Key Features

- **Multimodal Input** — Analyzes code, screen capture, and voice simultaneously 
- **Real-Time Streaming** — Gemini responses appear word-by-word in the floating HUD
- **Privacy First (Sentinel)** — Regex-based DLP sanitizes API keys, passwords, IPs, and database URIs before they leave the machine
- **Smart Caching** — SHA-256 hashing avoids re-sending unchanged code and screenshots
- **Floating HUD** — Transparent, always-on-top, borderless window with animated state indicator
- **Echo Cancellation** — Microphone silences during audio playback to prevent feedback loops
- **Voice Activity Detection** — RMS energy-based VAD for hands-free activation
- **Exponential Backoff** — VS Code extension reconnects intelligently (1s → 2s → 4s → ... → 30s max, with ±20% jitter)
- **Dynamic Port Discovery** — No hardcoded ports; Rust writes `~/.lia/port` for the extension to discover

---

## Prerequisites

- [Node.js](https://nodejs.org/) v18+
- [Rust and Cargo](https://rustup.rs/)
- [Python 3.12+](https://www.python.org/)
- [Google Cloud CLI](https://cloud.google.com/sdk/docs/install) with Vertex AI, Speech-to-Text, and Text-to-Speech APIs enabled

**Linux system libraries:**
```bash
sudo apt update && sudo apt install -y \
  pkg-config libasound2-dev libx11-dev libxcb1-dev \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

---

## Installation

```bash
# 1. Clone
git clone https://github.com/alexis-campos/LIA-more-than-an-agent.git
cd lia-monorepo

# 2. Desktop Client (Rust/Tauri + React)
cd lia-client && npm install && cd ..

# 3. VS Code Extension
cd lia-vscode && npm install && cd ..

# 4. Cloud Backend (Python)
cd lia-cloud
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
cd ..

# 5. Google Cloud Credentials
gcloud auth login
gcloud config set project YOUR_PROJECT_ID
gcloud services enable aiplatform.googleapis.com speech.googleapis.com texttospeech.googleapis.com
gcloud auth application-default login
```

---

## Running the Demo

Start all three components simultaneously:

### 1. Cloud Brain
```bash
cd lia-cloud && source venv/bin/activate && python main.py
```
Verify: `curl http://127.0.0.1:8000/health`

### 2. Desktop Client
```bash
cd lia-client && npm run tauri dev
```
First run compiles Rust (~3-5 min). The floating HUD will appear with an animated status orb.

### 3. VS Code Extension
1. Open VS Code → **File → Open Folder → `lia-vscode`**
2. Press **F5** (starts Extension Development Host)
3. Open any code file in the new window
4. Click **"Preguntar a Lia"** in the HUD → Gemini responds in real-time streaming

---

## Project Structure

```
lia-monorepo/
├── lia-client/                     # Desktop App (Rust/Tauri + React)
│   ├── src-tauri/src/
│   │   ├── main.rs                 # Entry point, WebSocket server, Tauri commands
│   │   ├── cloud_client.rs         # WebSocket client → Cloud Python 
│   │   ├── orchestrator.rs         # State machine (IDLE → LISTENING → THINKING → RESPONDING)
│   │   ├── context.rs              # Thread-safe shared state (Arc<Mutex>)
│   │   ├── sentinel.rs             # DLP: 9 regex patterns for secret sanitization
│   │   ├── hasher.rs               # SHA-256 hashing for smart caching
│   │   ├── request.rs              # Contract B builder (multimodal payload)
│   │   ├── vision.rs               # Screen capture with multi-monitor support
│   │   ├── audio.rs                # Mic recording + WAV encoding + echo cancellation
│   │   ├── playback.rs             # Audio playback with echo flag management
│   │   └── wakeword.rs             # Voice Activity Detection (RMS energy)
│   └── src/
│       ├── App.tsx                  # Root HUD component
│       ├── App.css                  # Glassmorphism + dark theme
│       ├── index.css                # Global styles + Inter font
│       └── components/
│           ├── StatusOrb.tsx        # Animated state indicator (Framer Motion)
│           ├── StreamingText.tsx    # Real-time streaming display
│           └── ContextBar.tsx       # File / line / language bar
│
├── lia-vscode/                     # VS Code Extension (TypeScript)
│   └── src/
│       └── extension.ts            # Context extraction + backoff + port discovery
│
├── lia-cloud/                      # Cloud Backend (Python/FastAPI)
│   ├── main.py                     # FastAPI server + WebSocket endpoint
│   ├── config.py                   # Environment configuration
│   ├── cache.py                    # Volatile LRU cache (RAM, 15min TTL)
│   ├── inference.py                # Gemini 1.5 Pro via Vertex AI (streaming)
│   ├── stt.py                      # Speech-to-Text (Google Cloud Speech)
│   └── tts.py                      # Text-to-Speech (Google Cloud TTS / WaveNet)
│
└── README.md
```

---

## Tech Stack

| Layer | Technologies |
|---|---|
| Desktop Runtime | Rust, Tauri 2, Tokio, Warp |
| Multimedia | xcap, cpal, hound, rodio |
| Security | regex (DLP), sha2, base64 |
| Networking | tokio-tungstenite, reqwest, futures-util |
| Frontend | React, TypeScript, Framer Motion |
| Extension | VS Code API, ws |
| Backend | Python, FastAPI, uvicorn |
| AI/ML | Vertex AI, Gemini 1.5 Pro, google-genai |
| Voice | Google Cloud Speech-to-Text, Google Cloud Text-to-Speech (WaveNet) |

---

## Testing

```bash
# Rust unit tests (23 tests)
cd lia-client/src-tauri && cargo test

# Python server health check
cd lia-cloud && source venv/bin/activate && python -c "import main; print('OK')"
```

---

## License

This project was built for the hackathon demo. All rights reserved.
