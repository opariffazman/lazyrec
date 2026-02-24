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
  keyframes: Keyframe[];
}

interface Keyframe {
  id: string;
  time: number;
  [key: string]: unknown;
}

type InspectorTab = "properties" | "settings";

interface ExportProgress {
  currentFrame: number;
  totalFrames: number;
  progress: number;
  etaSeconds: number;
  state: string;
}

function EditorScreen({ onBack }: { onBack: () => void }) {
  const [playheadTime, setPlayheadTime] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [duration] = useState(30);
  const [exportProgress, setExportProgress] = useState<ExportProgress | null>(null);
  const [isExporting, setIsExporting] = useState(false);
  const [selectedKeyframe, setSelectedKeyframe] = useState<{
    trackType: string;
    keyframe: Keyframe;
  } | null>(null);
  const [tracks, setTracks] = useState<Track[]>([
    { id: "t1", name: "Transform", type: "transform", keyframes: [
      { id: "k1", time: 2, zoom: 2.5, centerX: 0.3, centerY: 0.4, easing: "spring" },
      { id: "k2", time: 8, zoom: 1.8, centerX: 0.6, centerY: 0.5, easing: "easeInOut" },
      { id: "k3", time: 15, zoom: 1.0, centerX: 0.5, centerY: 0.5, easing: "easeOut" },
    ]},
    { id: "t2", name: "Ripple", type: "ripple", keyframes: [
      { id: "k4", time: 3, intensity: 0.8, rippleDuration: 0.4, color: "leftClick" },
      { id: "k5", time: 12, intensity: 0.6, rippleDuration: 0.3, color: "rightClick" },
    ]},
    { id: "t3", name: "Cursor", type: "cursor", keyframes: [] },
    { id: "t4", name: "Keystroke", type: "keystroke", keyframes: [
      { id: "k6", time: 5, text: "Cmd+S", displayDuration: 1.5 },
      { id: "k7", time: 20, text: "Cmd+Z", displayDuration: 1.5 },
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

  const handleExport = async () => {
    if (isExporting) return;
    setIsExporting(true);
    setExportProgress(null);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke("start_export");
      setExportProgress({
        currentFrame: 0,
        totalFrames: 0,
        progress: 1,
        etaSeconds: 0,
        state: "completed",
      });
      console.log("Export result:", result);
    } catch (err) {
      console.error("Export failed:", err);
      setExportProgress({
        currentFrame: 0,
        totalFrames: 0,
        progress: 0,
        etaSeconds: 0,
        state: "failed",
      });
    } finally {
      setIsExporting(false);
    }
  };

  const handleKeyframeSelect = (trackType: string, kf: Keyframe) => {
    setSelectedKeyframe({ trackType, keyframe: kf });
    setPlayheadTime(kf.time);
  };

  let nextKfId = useRef(100);
  const createDefaultKeyframe = (trackType: string, time: number): Keyframe => {
    const id = `k${nextKfId.current++}`;
    switch (trackType) {
      case "transform":
        return { id, time, zoom: 1.0, centerX: 0.5, centerY: 0.5, easing: "easeInOut" };
      case "ripple":
        return { id, time, intensity: 0.8, rippleDuration: 0.4, color: "leftClick" };
      case "keystroke":
        return { id, time, text: "Key", displayDuration: 1.5 };
      default:
        return { id, time };
    }
  };

  const handleAddKeyframe = (trackId: string) => {
    setTracks(prev => prev.map(track => {
      if (track.id !== trackId) return track;
      const kf = createDefaultKeyframe(track.type, playheadTime);
      const keyframes = [...track.keyframes, kf].sort((a, b) => a.time - b.time);
      return { ...track, keyframes };
    }));
  };

  const handleDeleteKeyframe = useCallback(() => {
    if (!selectedKeyframe) return;
    const kfId = selectedKeyframe.keyframe.id;
    setTracks(prev => prev.map(track => ({
      ...track,
      keyframes: track.keyframes.filter(k => k.id !== kfId),
    })));
    setSelectedKeyframe(null);
  }, [selectedKeyframe]);

  const handleUpdateKeyframe = useCallback((keyframeId: string, field: string, value: unknown) => {
    setTracks(prev => prev.map(track => ({
      ...track,
      keyframes: track.keyframes.map(kf =>
        kf.id === keyframeId ? { ...kf, [field]: value } : kf
      ),
    })));
    // Also update the selected keyframe so inspector reflects the change
    setSelectedKeyframe(prev => {
      if (prev && prev.keyframe.id === keyframeId) {
        return { ...prev, keyframe: { ...prev.keyframe, [field]: value } };
      }
      return prev;
    });
  }, []);

  // Delete key handler
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Delete" || e.key === "Backspace") {
        // Don't delete if user is typing in an input
        if ((e.target as HTMLElement).tagName === "INPUT" ||
            (e.target as HTMLElement).tagName === "SELECT") return;
        handleDeleteKeyframe();
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [handleDeleteKeyframe]);

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
          <button
            className="export-btn"
            onClick={handleExport}
            disabled={isExporting}
          >
            {isExporting ? "Exporting..." : "Export"}
          </button>
        </div>
      </div>

      <div className="editor-body">
        {exportProgress && (
          <div className="export-progress-bar">
            <div className="export-progress-fill" style={{ width: `${exportProgress.progress * 100}%` }} />
            <span className="export-progress-text">
              {exportProgress.state === "completed"
                ? "Export complete"
                : exportProgress.state === "failed"
                ? "Export failed"
                : `Exporting... ${Math.round(exportProgress.progress * 100)}%`}
            </span>
          </div>
        )}
        <div className="editor-main">
          <VideoPreview playheadTime={playheadTime} duration={duration} />
          <InspectorPanel selection={selectedKeyframe} onUpdateKeyframe={handleUpdateKeyframe} />
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
                selectedId={selectedKeyframe?.keyframe.id}
                onSelectKeyframe={(kf) => handleKeyframeSelect(track.type, kf)}
                onAddKeyframe={() => handleAddKeyframe(track.id)}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

// =============================================================================
// Video Preview
// =============================================================================

function VideoPreview({
  playheadTime,
  duration,
}: {
  playheadTime: number;
  duration: number;
}) {
  const progress = duration > 0 ? playheadTime / duration : 0;

  return (
    <div className="preview-area">
      <div className="video-preview">
        <div className="preview-canvas">
          {/* Simulated zoom/pan viewport indicator */}
          <div
            className="viewport-indicator"
            style={{
              width: `${60 + 40 * (1 - progress)}%`,
              height: `${60 + 40 * (1 - progress)}%`,
              left: `${20 * progress}%`,
              top: `${15 * progress}%`,
            }}
          />
          <div className="preview-cursor" style={{
            left: `${30 + 40 * progress}%`,
            top: `${40 + 20 * Math.sin(progress * Math.PI * 2)}%`,
          }} />
        </div>
        <div className="preview-overlay">
          <span className="preview-time">{formatTimecode(playheadTime)}</span>
        </div>
      </div>
    </div>
  );
}

// =============================================================================
// Inspector Panel
// =============================================================================

function InspectorPanel({
  selection,
  onUpdateKeyframe,
}: {
  selection: { trackType: string; keyframe: Keyframe } | null;
  onUpdateKeyframe?: (keyframeId: string, field: string, value: unknown) => void;
}) {
  const [tab, setTab] = useState<InspectorTab>("properties");

  if (!selection) {
    return (
      <div className="inspector-panel">
        <div className="inspector-empty">
          <span className="inspector-empty-icon">◇</span>
          <span>Select a keyframe to inspect</span>
        </div>
      </div>
    );
  }

  return (
    <div className="inspector-panel">
      <div className="inspector-tabs">
        <button
          className={`inspector-tab ${tab === "properties" ? "active" : ""}`}
          onClick={() => setTab("properties")}
        >
          Properties
        </button>
        <button
          className={`inspector-tab ${tab === "settings" ? "active" : ""}`}
          onClick={() => setTab("settings")}
        >
          Settings
        </button>
      </div>

      <div className="inspector-body">
        {tab === "properties" ? (
          <KeyframeProperties
            trackType={selection.trackType}
            keyframe={selection.keyframe}
            onUpdate={onUpdateKeyframe}
          />
        ) : (
          <RenderSettingsPanel />
        )}
      </div>
    </div>
  );
}

function KeyframeProperties({
  trackType,
  keyframe,
  onUpdate,
}: {
  trackType: string;
  keyframe: Keyframe;
  onUpdate?: (keyframeId: string, field: string, value: unknown) => void;
}) {
  const color = TRACK_COLORS[trackType] || "#888";
  const update = (field: string, value: unknown) => onUpdate?.(keyframe.id, field, value);

  return (
    <div className="kf-properties">
      <div className="kf-header">
        <span className="kf-badge" style={{ background: color }}>
          {trackType}
        </span>
        <span className="kf-time">@ {formatTimecode(keyframe.time)}</span>
      </div>

      {trackType === "transform" && (
        <>
          <PropertyRow label="Zoom" value={keyframe.zoom as number} type="number" step={0.1} min={0.1} max={10}
            onChange={(v) => update("zoom", v)} />
          <PropertyRow label="Center X" value={keyframe.centerX as number} type="number" step={0.01} min={0} max={1}
            onChange={(v) => update("centerX", v)} />
          <PropertyRow label="Center Y" value={keyframe.centerY as number} type="number" step={0.01} min={0} max={1}
            onChange={(v) => update("centerY", v)} />
        </>
      )}

      {trackType === "ripple" && (
        <>
          <PropertyRow label="Intensity" value={keyframe.intensity as number} type="number" step={0.1} min={0} max={2}
            onChange={(v) => update("intensity", v)} />
          <PropertyRow label="Duration" value={keyframe.rippleDuration as number} type="number" step={0.1} min={0.1} max={5} suffix="s"
            onChange={(v) => update("rippleDuration", v)} />
          <PropertyRow label="Color" value={String(keyframe.color)} type="select"
            options={["leftClick", "rightClick", "middleClick"]}
            onChange={(v) => update("color", v)} />
        </>
      )}

      {trackType === "keystroke" && (
        <>
          <PropertyRow label="Text" value={String(keyframe.text)} type="text"
            onChange={(v) => update("text", v)} />
          <PropertyRow label="Duration" value={keyframe.displayDuration as number} type="number" step={0.1} min={0.1} max={10} suffix="s"
            onChange={(v) => update("displayDuration", v)} />
        </>
      )}

      {trackType === "cursor" && (
        <div className="kf-empty-note">No editable properties</div>
      )}

      <div className="easing-section">
        <label className="section-label">Easing Curve</label>
        <div className="easing-presets">
          {["linear", "easeIn", "easeOut", "easeInOut", "spring"].map(e => (
            <button
              key={e}
              className={`easing-btn ${keyframe.easing === e ? "active" : ""}`}
              onClick={() => update("easing", e)}
            >
              {e}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

function PropertyRow({
  label,
  value,
  type = "text",
  step,
  min,
  max,
  suffix,
  options,
  onChange,
}: {
  label: string;
  value: string | number;
  type?: "text" | "number" | "select";
  step?: number;
  min?: number;
  max?: number;
  suffix?: string;
  options?: string[];
  onChange?: (value: string | number) => void;
}) {
  if (type === "select" && options) {
    return (
      <div className="property-row">
        <span className="property-label">{label}</span>
        <select className="property-select" value={String(value)}
          onChange={(e) => onChange?.(e.target.value)}>
          {options.map(opt => <option key={opt} value={opt}>{opt}</option>)}
        </select>
      </div>
    );
  }

  return (
    <div className="property-row">
      <span className="property-label">{label}{suffix ? ` (${suffix})` : ""}</span>
      <input
        className="property-input"
        type={type}
        value={value}
        step={step}
        min={min}
        max={max}
        onChange={(e) => {
          const v = type === "number" ? parseFloat(e.target.value) || 0 : e.target.value;
          onChange?.(v);
        }}
        readOnly={!onChange}
      />
    </div>
  );
}

function RenderSettingsPanel() {
  return (
    <div className="render-settings">
      <label className="section-label">Output</label>
      <div className="property-row">
        <span className="property-label">Resolution</span>
        <select className="property-select">
          <option>Original</option>
          <option>4K (3840x2160)</option>
          <option>1440p</option>
          <option>1080p</option>
          <option>720p</option>
        </select>
      </div>
      <div className="property-row">
        <span className="property-label">Codec</span>
        <select className="property-select">
          <option>H.265 (HEVC)</option>
          <option>H.264</option>
        </select>
      </div>
      <div className="property-row">
        <span className="property-label">Quality</span>
        <select className="property-select">
          <option>High</option>
          <option>Medium</option>
          <option>Low</option>
          <option>Original</option>
        </select>
      </div>
      <div className="property-row">
        <span className="property-label">Frame Rate</span>
        <select className="property-select">
          <option>Original</option>
          <option>60 fps</option>
          <option>30 fps</option>
        </select>
      </div>

      <label className="section-label">Window Mode</label>
      <div className="property-row">
        <span className="property-label">Background</span>
        <input type="checkbox" />
      </div>
      <div className="property-row">
        <span className="property-label">Corner Radius</span>
        <input className="property-input" value="22" readOnly />
      </div>
      <div className="property-row">
        <span className="property-label">Shadow</span>
        <input className="property-input" value="0.7" readOnly />
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
  selectedId,
  onSelectKeyframe,
  onAddKeyframe,
}: {
  track: Track;
  duration: number;
  playheadTime: number;
  selectedId?: string;
  onSelectKeyframe?: (kf: Keyframe) => void;
  onAddKeyframe?: () => void;
}) {
  const color = TRACK_COLORS[track.type] || "#888";

  return (
    <div className="timeline-track">
      <div className="track-label" style={{ borderLeftColor: color }}>
        {track.name}
      </div>
      <div className="track-lane" onDoubleClick={() => onAddKeyframe?.()}>
        {track.keyframes.map(kf => (
          <div
            key={kf.id}
            className={`keyframe-marker ${kf.id === selectedId ? "selected" : ""}`}
            style={{
              left: `${(kf.time / duration) * 100}%`,
              backgroundColor: color,
            }}
            title={`${track.name} @ ${formatTimecode(kf.time)}`}
            onClick={() => onSelectKeyframe?.(kf)}
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
