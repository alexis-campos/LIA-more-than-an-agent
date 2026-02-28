# Lia — More Than an Agent

**Lia** is a **Proactive and Multimodal Programming Companion**. She acts as a real-time "Pair Programmer" that can see your screen, hear your voice, and read your code — offering deep contextual assistance powered by Gemini 2.0 Flash via Vertex AI.

Unlike traditional chatbots, Lia bridges the gap between the IDE and the AI by combining screen capture, live code analysis, voice interaction, and a transparent floating HUD — all running locally on your desktop while offloading inference to the cloud.

---

## Architecture

The system follows a three-pillar architecture communicating asynchronously via WebSockets:

| Pillar | Technology | Role |
|---|---|---|
| **`lia-client`** | Rust / Tauri 2 + React | Desktop app. Screen capture (`xcap`), microphone recording with echo cancellation (`cpal`/`hound`), TTS audio playback (`rodio`), privacy guard (Sentinel DLP), smart caching (SHA-256), state machine, and floating HUD with glassmorphism |
| **`lia-vscode`** | TypeScript | VS Code extension. Extracts live code context (±50 lines around cursor) with debounce, exponential backoff reconnection, and dynamic port discovery |
| **`lia-cloud`** | Python / FastAPI | Cloud backend. Receives multimodal requests, transcribes speech (Google STT), resolves a volatile LRU cache, streams responses from Gemini 2.0 Flash (Vertex AI), and synthesizes voice replies (Google TTS / WaveNet) |

### Data Flow

```
                         ┌─────────────────────────────────────────────┐
                         │              Lia Desktop (Rust)             │
                         │                                             │
VS Code ──(Contract A)──►│  Context ─► Mic Record ─► Screen Capture   │
  (context_update)       │      │          │               │           │
                         │      └──────────┼───────────────┘           │
                         │                 ▼                           │
                         │      Sentinel DLP + Smart Cache             │
                         │                 │                           │
                         │          Contract B (multimodal)            │
                         │                 │                           │
                         │                 ▼                           │
                         │     ┌───── Cloud Python ──────┐            │
                         │     │  STT → Gemini → TTS     │            │
                         │     └─────────────────────────┘            │
                         │                 │                           │
                         │          Contract C (streaming)             │
                         │           ╱            ╲                    │
                         │     Text chunks    Audio chunks             │
                         │         │               │                   │
                         │    React HUD      AudioPlayer               │
                         │   (streaming)    (echo cancel)              │
                         └─────────────────────────────────────────────┘
```

- **Contract A:** Real-time editor context (file, cursor, ±50 lines of code)
- **Contract B:** Multimodal request (sanitized code + screen capture + audio WAV, with smart caching via SHA-256)
- **Contract C:** Streaming response from Gemini (text chunks + TTS audio)

### State Machine

```
IDLE → LISTENING → THINKING → RESPONDING → IDLE
 │        │           │            │
 │   Records mic   Captures     Streams
 │   (4s, echo     screen +     Gemini
 │    cancel)      Sentinel +   response
 │                 builds       to HUD +
 │                 Contract B   plays TTS
 │
 └── Waiting for user to click "Preguntar a Lia"
```

---

## Key Features

- **True Multimodal Input** — Analyzes code, screen capture, and voice simultaneously in a single request
- **Real-Time Streaming** — Gemini responses appear word-by-word in the floating HUD
- **Voice I/O** — Records user speech (STT via Google Cloud Speech), responds with synthesized voice (TTS via Google Cloud WaveNet)
- **Echo Cancellation** — Shared `PlayingFlag` between mic and speaker: microphone automatically silences during TTS playback to prevent feedback loops
- **Privacy First (Sentinel DLP)** — 9 regex patterns sanitize API keys, passwords, private IPs, database URIs, and more before data leaves the machine
- **Smart Caching** — SHA-256 hashing detects unchanged code and screenshots, avoiding redundant data transfer
- **Floating HUD** — Transparent, always-on-top, borderless glassmorphism window with animated state orb (Framer Motion)
- **Voice Activity Detection** — RMS energy-based VAD infrastructure for hands-free activation
- **Exponential Backoff** — VS Code extension reconnects intelligently (1s → 2s → 4s → ... → 30s max, with ±20% jitter)
- **Dynamic Port Discovery** — No hardcoded ports; Rust writes `~/.lia/port` for the extension to discover automatically
- **Volatile LRU Cache** — RAM-only cache with 15min TTL avoids redundant Gemini calls for identical requests

---

## Prerequisites

- [Node.js](https://nodejs.org/) v18+
- [Rust and Cargo](https://rustup.rs/)
- [Python 3.12+](https://www.python.org/)
- [Google Cloud CLI](https://cloud.google.com/sdk/docs/install) with the following APIs enabled:
  - Vertex AI (`aiplatform.googleapis.com`)
  - Speech-to-Text (`speech.googleapis.com`)
  - Text-to-Speech (`texttospeech.googleapis.com`)

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

Start all three components simultaneously in separate terminals:

### Terminal 1: Cloud Brain
```bash
cd lia-cloud && source venv/bin/activate && python main.py
```
Verify: `curl http://127.0.0.1:8000/health` → `{"status": "ok"}`

### Terminal 2: Desktop Client
```bash
cd lia-client && npm run tauri dev
```
First run compiles Rust (~3-5 min). The floating HUD appears with an animated status orb.

Expected output:
```
╔══════════════════════════════════════╗
║        Lia Desktop v0.1.0            ║
║     More Than an Agent               ║
╚══════════════════════════════════════╝
Sentinel DLP activo
Echo cancellation listo
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  WS: ws://127.0.0.1:3333/ws
  Cloud: ws://127.0.0.1:8000/ws/lia
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### Terminal 3: VS Code Extension
1. Open VS Code → **File → Open Folder → `lia-vscode`**
2. Press **F5** (starts Extension Development Host)
3. Open any code file in the new window

### Using Lia
1. Open a code file in the Extension Development Host
2. Click **"Preguntar a Lia"** in the floating HUD
3. Lia records 4 seconds of audio (speak or stay silent for proactive analysis)
4. Screen capture + code sanitization happens automatically
5. Gemini streams its response word-by-word in the HUD
6. TTS audio plays the response through your speakers

---

## Project Structure

```
lia-monorepo/
├── lia-client/                     # Desktop App (Rust/Tauri 2 + React)
│   ├── src-tauri/src/
│   │   ├── main.rs                 # Entry point, pipeline orchestration, Tauri commands
│   │   ├── cloud_client.rs         # WebSocket client → Cloud (sends B, receives C)
│   │   ├── orchestrator.rs         # State machine (IDLE → LISTENING → THINKING → RESPONDING)
│   │   ├── context.rs              # Thread-safe shared state (Arc<Mutex>)
│   │   ├── sentinel.rs             # DLP: 9 regex patterns for secret sanitization
│   │   ├── hasher.rs               # SHA-256 hashing for smart caching
│   │   ├── request.rs              # Contract B builder (multimodal payload)
│   │   ├── vision.rs               # Screen capture with multi-monitor support (xcap)
│   │   ├── audio.rs                # Mic recording + WAV encoding + echo cancellation (cpal)
│   │   ├── playback.rs             # TTS audio playback with echo flag management (rodio)
│   │   └── wakeword.rs             # Voice Activity Detection (RMS energy, hands-free ready)
│   └── src/
│       ├── App.tsx                  # Root HUD component + "Preguntar a Lia" button
│       ├── App.css                  # Glassmorphism + dark theme + button styles
│       ├── index.css                # Global styles + Inter font
│       └── components/
│           ├── StatusOrb.tsx        # Animated state indicator (Framer Motion)
│           ├── StreamingText.tsx    # Real-time Gemini streaming display
│           └── ContextBar.tsx       # File / line / language / workspace bar
│
├── lia-vscode/                     # VS Code Extension (TypeScript)
│   └── src/
│       └── extension.ts            # Context extraction + debounce + backoff + port discovery
│
├── lia-cloud/                      # Cloud Backend (Python/FastAPI)
│   ├── main.py                     # FastAPI + WebSocket /ws/lia + auth + processing
│   ├── config.py                   # Centralized environment configuration
│   ├── cache.py                    # Volatile LRU cache (RAM only, 15min TTL, 50 entries)
│   ├── inference.py                # Gemini 2.0 Flash via Vertex AI (streaming)
│   ├── stt.py                      # Speech-to-Text (Google Cloud Speech, es-ES + en-US)
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
| Networking | tokio-tungstenite, futures-util |
| Frontend | React, TypeScript, Framer Motion |
| Extension | VS Code API, ws |
| Backend | Python 3.12, FastAPI, uvicorn |
| AI/ML | Vertex AI, Gemini 2.0 Flash, google-genai |
| Voice | Google Cloud Speech-to-Text, Google Cloud Text-to-Speech (WaveNet) |

---

## Testing

```bash
# Rust unit tests (23 tests: Sentinel, Hasher, Orchestrator, Audio, VAD, Request)
cd lia-client/src-tauri && cargo test

# Verify build (0 errors, 0 warnings)
cd lia-client/src-tauri && cargo check

# Python server health check
curl http://127.0.0.1:8000/health
```

---

## License

This project was built for the hackathon demo. All rights reserved.
