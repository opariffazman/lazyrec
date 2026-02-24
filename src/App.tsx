import { useState, useRef, useCallback, useEffect } from "react";
import "./App.css";

type Screen = "welcome" | "recording" | "editor";

function App() {
  const [screen, setScreen] = useState<Screen>("welcome");

  return (
    <main className="app">
      {screen === "welcome" && (
        <WelcomeScreen
          onStartRecording={() => setScreen("recording")}
          onOpenEditor={() => setScreen("editor")}
        />
      )}
      {screen === "recording" && (
        <RecordingScreen
          onBack={() => setScreen("welcome")}
          onRecordingComplete={() => setScreen("editor")}
        />
      )}
      {screen === "editor" && (
        <EditorScreen onBack={() => setScreen("welcome")} />
      )}
    </main>
  );
}

// =============================================================================
// Welcome Screen
// =============================================================================

function WelcomeScreen({
  onStartRecording,
  onOpenEditor,
}: {
  onStartRecording: () => void;
  onOpenEditor: () => void;
}) {
  const [isDragging, setIsDragging] = useState(false);

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    const files = Array.from(e.dataTransfer.files);
    const file = files[0];
    if (file) {
      console.log("Dropped file:", file.name);
      onOpenEditor();
    }
  };

  return (
    <div className="welcome">
      <div className="welcome-logo">
        <svg width="64" height="64" viewBox="0 0 64 64" fill="none">
          <rect width="64" height="64" rx="14" fill="#1a1a2e" />
          <circle cx="32" cy="28" r="10" fill="#e94560" />
          <rect x="18" y="42" width="28" height="4" rx="2" fill="#e94560" opacity="0.6" />
          <rect x="22" y="50" width="20" height="3" rx="1.5" fill="#e94560" opacity="0.3" />
        </svg>
        <h1>LazyRec</h1>
        <p className="subtitle">Screen Recording & Timeline Editing</p>
      </div>

      <div className="action-cards">
        <ActionCard
          icon="●"
          title="Record"
          description="Record screen or window"
          color="#e94560"
          onClick={onStartRecording}
        />
        <ActionCard
          icon="▶"
          title="Open Video"
          description="Edit existing video"
          color="#4a90d9"
          onClick={onOpenEditor}
        />
        <ActionCard
          icon="📁"
          title="Open Project"
          description="Continue editing"
          color="#e09145"
          onClick={onOpenEditor}
        />
      </div>

      <div
        className={`drop-zone ${isDragging ? "dragging" : ""}`}
        onDragOver={(e) => { e.preventDefault(); setIsDragging(true); }}
        onDragLeave={() => setIsDragging(false)}
        onDrop={handleDrop}
      >
        <span className="drop-icon">↓</span>
        <span className="drop-text">Drop video or project here</span>
        <span className="drop-hint">.mp4, .mov, .lazyrec</span>
      </div>
    </div>
  );
}

// =============================================================================
// Recording Screen
// =============================================================================

type RecordingState = "idle" | "countdown" | "recording" | "paused";

function RecordingScreen({
  onBack,
  onRecordingComplete,
}: {
  onBack: () => void;
  onRecordingComplete: () => void;
}) {
  const [state, setState] = useState<RecordingState>("idle");
  const [elapsed, setElapsed] = useState(0);
  const [countdown, setCountdown] = useState(3);
  const timerRef = useRef<number | null>(null);
  const startTimeRef = useRef(0);

  const startCountdown = useCallback(() => {
    setState("countdown");
    setCountdown(3);
    let count = 3;
    const interval = window.setInterval(() => {
      count--;
      if (count <= 0) {
        clearInterval(interval);
        setState("recording");
        startTimeRef.current = Date.now();
        setElapsed(0);
      } else {
        setCountdown(count);
      }
    }, 1000);
  }, []);

  // Timer for elapsed recording time
  useEffect(() => {
    if (state === "recording") {
      timerRef.current = window.setInterval(() => {
        setElapsed(Math.floor((Date.now() - startTimeRef.current) / 1000));
      }, 200);
    } else {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    }
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [state]);

  const togglePause = () => {
    if (state === "recording") {
      setState("paused");
    } else if (state === "paused") {
      setState("recording");
      startTimeRef.current = Date.now() - elapsed * 1000;
    }
  };

  const stopRecording = () => {
    setState("idle");
    setElapsed(0);
    onRecordingComplete();
  };

  const formatTime = (seconds: number) => {
    const m = Math.floor(seconds / 60).toString().padStart(2, "0");
    const s = (seconds % 60).toString().padStart(2, "0");
    return `${m}:${s}`;
  };

  return (
    <div className="recording-screen">
      <div className="recording-header">
        <button className="back-btn" onClick={onBack}>
          ← Back
        </button>
        <h2>Recording</h2>
      </div>

      <div className="recording-body">
        {state === "countdown" && (
          <div className="countdown-overlay">
            <span className="countdown-number">{countdown}</span>
          </div>
        )}

        {state === "idle" && (
          <div className="recording-ready">
            <div className="source-selector">
              <label>Capture Source</label>
              <select className="source-dropdown">
                <option>Entire Screen</option>
                <option>Window...</option>
              </select>
            </div>
            <button className="record-btn" onClick={startCountdown}>
              <span className="record-dot" />
              Start Recording
            </button>
          </div>
        )}

        {(state === "recording" || state === "paused") && (
          <div className="recording-active">
            <div className={`recording-indicator ${state === "paused" ? "paused" : ""}`}>
              <span className="rec-dot" />
              <span className="rec-label">{state === "paused" ? "PAUSED" : "REC"}</span>
            </div>

            <div className="recording-timer">{formatTime(elapsed)}</div>

            <div className="recording-controls">
              <button className="control-btn" onClick={togglePause}>
                {state === "paused" ? "▶ Resume" : "⏸ Pause"}
              </button>
              <button className="control-btn stop" onClick={stopRecording}>
                ■ Stop
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// =============================================================================
// Editor Screen (Timeline + Preview)
// =============================================================================

interface Track {
  id: string;
  name: string;
  type: "transform" | "ripple" | "cursor" | "keystroke";
  keyframes: { id: string; time: number }[];
}

function EditorScreen({ onBack }: { onBack: () => void }) {
  const [playheadTime, setPlayheadTime] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [duration] = useState(30); // placeholder duration
  const [tracks] = useState<Track[]>([
    { id: "t1", name: "Transform", type: "transform", keyframes: [
      { id: "k1", time: 2 }, { id: "k2", time: 8 }, { id: "k3", time: 15 },
    ]},
    { id: "t2", name: "Ripple", type: "ripple", keyframes: [
      { id: "k4", time: 3 }, { id: "k5", time: 12 },
    ]},
    { id: "t3", name: "Cursor", type: "cursor", keyframes: [] },
    { id: "t4", name: "Keystroke", type: "keystroke", keyframes: [
      { id: "k6", time: 5 }, { id: "k7", time: 20 },
    ]},
  ]);

  const playbackRef = useRef<number | null>(null);
  const lastFrameRef = useRef(0);

  useEffect(() => {
    if (isPlaying) {
      lastFrameRef.current = performance.now();
      const animate = () => {
        const now = performance.now();
        const dt = (now - lastFrameRef.current) / 1000;
        lastFrameRef.current = now;
        setPlayheadTime(prev => {
          const next = prev + dt;
          if (next >= duration) {
            setIsPlaying(false);
            return 0;
          }
          return next;
        });
        playbackRef.current = requestAnimationFrame(animate);
      };
      playbackRef.current = requestAnimationFrame(animate);
    }
    return () => {
      if (playbackRef.current) cancelAnimationFrame(playbackRef.current);
    };
  }, [isPlaying, duration]);

  return (
    <div className="editor-screen">
      <div className="editor-header">
        <button className="back-btn" onClick={onBack}>← Back</button>
        <h2>Timeline Editor</h2>
        <div className="editor-transport">
          <button className="transport-btn" onClick={() => setPlayheadTime(0)}>
            ⏮
          </button>
          <button
            className="transport-btn play"
            onClick={() => setIsPlaying(!isPlaying)}
          >
            {isPlaying ? "⏸" : "▶"}
          </button>
          <span className="time-display">
            {formatTimecode(playheadTime)} / {formatTimecode(duration)}
          </span>
        </div>
      </div>

      <div className="editor-body">
        <div className="preview-area">
          <div className="preview-placeholder">
            <span>Video Preview</span>
            <span className="preview-time">{formatTimecode(playheadTime)}</span>
          </div>
        </div>

        <div className="timeline-panel">
          <TimelineRuler
            duration={duration}
            playheadTime={playheadTime}
            onSeek={setPlayheadTime}
          />
          <div className="timeline-tracks">
            {tracks.map(track => (
              <TimelineTrack
                key={track.id}
                track={track}
                duration={duration}
                playheadTime={playheadTime}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function TimelineRuler({
  duration,
  playheadTime,
  onSeek,
}: {
  duration: number;
  playheadTime: number;
  onSeek: (time: number) => void;
}) {
  const rulerRef = useRef<HTMLDivElement>(null);

  const handleClick = (e: React.MouseEvent) => {
    if (!rulerRef.current) return;
    const rect = rulerRef.current.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    onSeek(ratio * duration);
  };

  // Generate time markers
  const markers: { time: number; label: string }[] = [];
  const step = duration <= 30 ? 5 : duration <= 120 ? 15 : 30;
  for (let t = 0; t <= duration; t += step) {
    markers.push({ time: t, label: formatTimecode(t) });
  }

  const playheadPercent = (playheadTime / duration) * 100;

  return (
    <div className="timeline-ruler" ref={rulerRef} onClick={handleClick}>
      {markers.map(m => (
        <span
          key={m.time}
          className="ruler-marker"
          style={{ left: `${(m.time / duration) * 100}%` }}
        >
          {m.label}
        </span>
      ))}
      <div
        className="playhead"
        style={{ left: `${playheadPercent}%` }}
      />
    </div>
  );
}

const TRACK_COLORS: Record<string, string> = {
  transform: "#4a90d9",
  ripple: "#e94560",
  cursor: "#e09145",
  keystroke: "#50c878",
};

function TimelineTrack({
  track,
  duration,
  playheadTime,
}: {
  track: Track;
  duration: number;
  playheadTime: number;
}) {
  const color = TRACK_COLORS[track.type] || "#888";

  return (
    <div className="timeline-track">
      <div className="track-label" style={{ borderLeftColor: color }}>
        {track.name}
      </div>
      <div className="track-lane">
        {track.keyframes.map(kf => (
          <div
            key={kf.id}
            className="keyframe-marker"
            style={{
              left: `${(kf.time / duration) * 100}%`,
              backgroundColor: color,
            }}
            title={`${track.name} @ ${formatTimecode(kf.time)}`}
          />
        ))}
        <div
          className="track-playhead"
          style={{ left: `${(playheadTime / duration) * 100}%` }}
        />
      </div>
    </div>
  );
}

// =============================================================================
// Shared Components
// =============================================================================

function ActionCard({
  icon,
  title,
  description,
  color,
  onClick,
}: {
  icon: string;
  title: string;
  description: string;
  color: string;
  onClick: () => void;
}) {
  return (
    <button className="action-card" onClick={onClick}>
      <span className="action-icon" style={{ color }}>{icon}</span>
      <span className="action-title">{title}</span>
      <span className="action-desc">{description}</span>
    </button>
  );
}

function formatTimecode(seconds: number): string {
  const m = Math.floor(seconds / 60).toString().padStart(2, "0");
  const s = Math.floor(seconds % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

export default App;
