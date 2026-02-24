import { useState, useRef, useCallback, useEffect } from "react";
import "./App.css";

type Screen = "welcome" | "recording" | "post-recording" | "editor";

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
          onRecordingComplete={() => setScreen("post-recording")}
        />
      )}
      {screen === "post-recording" && (
        <PostRecordingScreen
          onQuickExport={() => setScreen("welcome")}
          onOpenEditor={() => setScreen("editor")}
          onBack={() => setScreen("welcome")}
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

interface RecordingStatusData {
  state: string;
  elapsed: number;
  frameCount: number;
}

interface CaptureSourceInfo {
  id: string;
  name: string;
  sourceType: "display" | "window";
  width: number;
  height: number;
}

function RecordingScreen({
  onBack,
  onRecordingComplete,
}: {
  onBack: () => void;
  onRecordingComplete: () => void;
}) {
  const [state, setState] = useState<RecordingState>("idle");
  const [elapsed, setElapsed] = useState(0);
  const [frameCount, setFrameCount] = useState(0);
  const [countdown, setCountdown] = useState(3);
  const [error, setError] = useState<string | null>(null);
  const [sources, setSources] = useState<CaptureSourceInfo[]>([]);
  const [selectedSourceId, setSelectedSourceId] = useState<string>("");
  const pollRef = useRef<number | null>(null);

  // Load capture sources on mount
  useEffect(() => {
    (async () => {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const srcs = await invoke<CaptureSourceInfo[]>("list_capture_sources");
        setSources(srcs);
        if (srcs.length > 0) setSelectedSourceId(srcs[0].id);
      } catch {
        // Fallback — sources remain empty, backend will use default
      }
    })();
  }, []);

  // Poll backend status while recording/paused
  useEffect(() => {
    if (state === "recording" || state === "paused") {
      const poll = async () => {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          const status = await invoke<RecordingStatusData>("get_recording_status");
          setElapsed(Math.floor(status.elapsed));
          setFrameCount(status.frameCount);
        } catch {
          // Silently ignore poll errors
        }
      };
      pollRef.current = window.setInterval(poll, 250);
      poll(); // immediate first poll
    } else {
      if (pollRef.current) {
        clearInterval(pollRef.current);
        pollRef.current = null;
      }
    }
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, [state]);

  const startCountdown = useCallback(() => {
    setState("countdown");
    setCountdown(3);
    setError(null);
    let count = 3;
    const interval = window.setInterval(async () => {
      count--;
      if (count <= 0) {
        clearInterval(interval);
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          // Set capture target based on selected source
          if (selectedSourceId) {
            const source = sources.find(s => s.id === selectedSourceId);
            if (source) {
              let target;
              if (source.sourceType === "window") {
                target = { type: "window" as const, title: source.name, windowId: 0 };
              } else {
                // Parse display index from source id (e.g., "display-1" → 1)
                const idxStr = source.id.replace("display-", "");
                const displayId = parseInt(idxStr, 10) || 0;
                target = { type: "display" as const, displayId };
              }
              await invoke("set_capture_target", {
                target,
                width: source.width || null,
                height: source.height || null,
              });
            }
          }
          await invoke("start_recording");
          setState("recording");
          setElapsed(0);
          setFrameCount(0);
        } catch (err) {
          setError(String(err));
          setState("idle");
        }
      } else {
        setCountdown(count);
      }
    }, 1000);
  }, [sources, selectedSourceId]);

  const togglePause = async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      if (state === "recording") {
        await invoke("pause_recording");
        setState("paused");
      } else if (state === "paused") {
        await invoke("resume_recording");
        setState("recording");
      }
    } catch (err) {
      setError(String(err));
    }
  };

  const stopRecording = async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("stop_recording");
      setState("idle");
      setElapsed(0);
      setFrameCount(0);
      onRecordingComplete();
    } catch (err) {
      setError(String(err));
      setState("idle");
    }
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
        {error && (
          <div className="recording-error">{error}</div>
        )}

        {state === "countdown" && (
          <div className="countdown-overlay">
            <span className="countdown-number">{countdown}</span>
          </div>
        )}

        {state === "idle" && (
          <div className="recording-ready">
            <div className="source-selector">
              <label>Capture Source</label>
              <select
                className="source-dropdown"
                value={selectedSourceId}
                onChange={(e) => setSelectedSourceId(e.target.value)}
              >
                {sources.length === 0 && <option value="">Entire Screen</option>}
                {sources.map(s => (
                  <option key={s.id} value={s.id}>
                    {s.name} ({s.width}x{s.height})
                  </option>
                ))}
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
            <div className="recording-frame-count">{frameCount} frames</div>

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
// Post-Recording Screen (Quick Export or Open Editor)
// =============================================================================

function PostRecordingScreen({
  onQuickExport,
  onOpenEditor,
  onBack,
}: {
  onQuickExport: () => void;
  onOpenEditor: () => void;
  onBack: () => void;
}) {
  const [status, setStatus] = useState<"idle" | "generating" | "exporting" | "complete" | "error">("idle");
  const [progress, setProgress] = useState(0);
  const [message, setMessage] = useState("");
  const [exportResult, setExportResult] = useState("");

  const handleQuickExport = async () => {
    setStatus("generating");
    setMessage("Generating auto-zoom keyframes...");
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const { listen } = await import("@tauri-apps/api/event");

      // Step 1: Auto-generate keyframes
      const genResult = await invoke<{
        transformCount: number;
        total: number;
      }>("generate_keyframes");
      setMessage(`Generated ${genResult.total} keyframes. Starting export...`);

      // Step 2: Listen for export events
      setStatus("exporting");
      const unlistenProgress = await listen<{
        currentFrame: number;
        totalFrames: number;
        progress: number;
        etaSeconds: number;
        state: string;
      }>("export-progress", (event) => {
        setProgress(event.payload.progress);
        const pct = Math.round(event.payload.progress * 100);
        const eta = event.payload.etaSeconds > 0 ? ` (${Math.ceil(event.payload.etaSeconds)}s left)` : "";
        setMessage(`Exporting: ${pct}%${eta} — ${event.payload.currentFrame}/${event.payload.totalFrames} frames`);
      });
      const unlistenComplete = await listen<string>("export-complete", (event) => {
        setStatus("complete");
        setProgress(1);
        setExportResult(event.payload);
        setMessage("Export complete!");
        unlistenProgress();
        unlistenComplete();
        unlistenError();
      });
      const unlistenError = await listen<string>("export-error", (event) => {
        setStatus("error");
        setMessage(`Export failed: ${event.payload}`);
        unlistenProgress();
        unlistenComplete();
        unlistenError();
      });

      // Step 3: Start export
      await invoke("start_export");
    } catch (err) {
      setStatus("error");
      setMessage(`Error: ${err}`);
    }
  };

  return (
    <div className="post-recording-screen">
      <div className="post-recording-header">
        <button className="back-btn" onClick={onBack}>
          ← Back
        </button>
        <h2>Recording Complete</h2>
      </div>

      {status === "idle" && (
        <div className="post-recording-choices">
          <div className="post-recording-card primary" onClick={handleQuickExport}>
            <div className="card-icon">⚡</div>
            <h3>Export with Auto-Zoom</h3>
            <p>Automatically generate zoom effects from your mouse activity and export immediately.</p>
          </div>
          <div className="post-recording-card" onClick={onOpenEditor}>
            <div className="card-icon">🎬</div>
            <h3>Open in Editor</h3>
            <p>Fine-tune keyframes, adjust zoom levels, and customize effects before exporting.</p>
          </div>
        </div>
      )}

      {(status === "generating" || status === "exporting") && (
        <div className="post-recording-progress">
          <div className="progress-spinner" />
          <p className="progress-message">{message}</p>
          {status === "exporting" && (
            <div className="progress-bar-container">
              <div className="progress-bar-fill" style={{ width: `${progress * 100}%` }} />
            </div>
          )}
        </div>
      )}

      {status === "complete" && (
        <div className="post-recording-complete">
          <div className="complete-icon">✓</div>
          <h3>Export Complete</h3>
          <p className="export-result">{exportResult}</p>
          <button className="action-btn" onClick={onQuickExport}>
            Done
          </button>
        </div>
      )}

      {status === "error" && (
        <div className="post-recording-error">
          <div className="error-icon">✗</div>
          <p className="error-message">{message}</p>
          <div className="error-actions">
            <button className="action-btn" onClick={() => setStatus("idle")}>
              Try Again
            </button>
            <button className="action-btn secondary" onClick={onOpenEditor}>
              Open Editor Instead
            </button>
          </div>
        </div>
      )}
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

interface MousePositionData {
  time: number;
  x: number;
  y: number;
}

function EditorScreen({ onBack }: { onBack: () => void }) {
  const [playheadTime, setPlayheadTime] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [duration, setDuration] = useState(30);
  const [exportProgress, setExportProgress] = useState<ExportProgress | null>(null);
  const [isExporting, setIsExporting] = useState(false);
  const [isGenerating, setIsGenerating] = useState(false);
  const [timelineZoom, setTimelineZoom] = useState(1);
  const [mousePositions, setMousePositions] = useState<MousePositionData[]>([]);
  const [selectedKeyframe, setSelectedKeyframe] = useState<{
    trackType: string;
    keyframe: Keyframe;
  } | null>(null);

  const defaultTracks: Track[] = [
    { id: "t1", name: "Transform", type: "transform", keyframes: [] },
    { id: "t2", name: "Ripple", type: "ripple", keyframes: [] },
    { id: "t3", name: "Cursor", type: "cursor", keyframes: [] },
    { id: "t4", name: "Keystroke", type: "keystroke", keyframes: [] },
  ];

  const [tracks, setTracksRaw] = useState<Track[]>(defaultTracks);

  // Undo/redo stack
  const undoStackRef = useRef<Track[][]>([]);
  const redoStackRef = useRef<Track[][]>([]);
  const MAX_UNDO = 50;

  const setTracks = useCallback((updater: Track[] | ((prev: Track[]) => Track[])) => {
    setTracksRaw(prev => {
      const next = typeof updater === "function" ? updater(prev) : updater;
      // Only push to undo if tracks actually changed
      if (JSON.stringify(prev) !== JSON.stringify(next)) {
        undoStackRef.current = [...undoStackRef.current.slice(-MAX_UNDO + 1), prev];
        redoStackRef.current = []; // clear redo on new edit
      }
      return next;
    });
  }, []);

  const undo = useCallback(() => {
    if (undoStackRef.current.length === 0) return;
    setTracksRaw(prev => {
      const snapshot = undoStackRef.current.pop()!;
      redoStackRef.current.push(prev);
      return snapshot;
    });
    setSelectedKeyframe(null);
  }, []);

  const redo = useCallback(() => {
    if (redoStackRef.current.length === 0) return;
    setTracksRaw(prev => {
      const snapshot = redoStackRef.current.pop()!;
      undoStackRef.current.push(prev);
      return snapshot;
    });
    setSelectedKeyframe(null);
  }, []);

  // Load timeline and mouse data from the backend on mount
  const loadTimelineFromBackend = useCallback(async () => {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const timeline = await invoke<{
        duration: number;
        tracks: { id: string; name: string; type: string; keyframes: Keyframe[] }[];
      }>("get_timeline");
      if (timeline) {
        setDuration(timeline.duration > 0 ? timeline.duration : 30);
        if (timeline.tracks.length > 0) {
          // Use setTracksRaw to avoid polluting the undo stack on load
          setTracksRaw(timeline.tracks.map(t => ({
            id: t.id,
            name: t.name,
            type: t.type as Track["type"],
            keyframes: t.keyframes,
          })));
          // Reset undo/redo when loading fresh data
          undoStackRef.current = [];
          redoStackRef.current = [];
        }
      }
    } catch {
      // No project loaded — keep defaults
    }
  }, []);

  useEffect(() => {
    (async () => {
      await loadTimelineFromBackend();
      // Also load mouse data for cursor preview
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const positions = await invoke<MousePositionData[]>("load_mouse_data");
        if (positions && positions.length > 0) {
          setMousePositions(positions);
        }
      } catch {
        // No mouse data available
      }
    })();
  }, [loadTimelineFromBackend]);

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

  // Listen for export progress/completion/error events from the backend
  useEffect(() => {
    let unlistenProgress: (() => void) | null = null;
    let unlistenComplete: (() => void) | null = null;
    let unlistenError: (() => void) | null = null;

    (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      unlistenProgress = await listen<ExportProgress>("export-progress", (event) => {
        setExportProgress(event.payload);
      });
      unlistenComplete = await listen<string>("export-complete", (event) => {
        console.log("Export complete:", event.payload);
        setExportProgress(prev => prev ? { ...prev, progress: 1, state: "completed" } : null);
        setIsExporting(false);
      });
      unlistenError = await listen<string>("export-error", (event) => {
        console.error("Export failed:", event.payload);
        setExportProgress({ currentFrame: 0, totalFrames: 0, progress: 0, etaSeconds: 0, state: "failed" });
        setIsExporting(false);
      });
    })();

    return () => {
      unlistenProgress?.();
      unlistenComplete?.();
      unlistenError?.();
    };
  }, []);

  const handleExport = async () => {
    if (isExporting) return;
    setIsExporting(true);
    setExportProgress(null);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("start_export");
      // Export runs in background — progress comes via events
    } catch (err) {
      console.error("Export failed to start:", err);
      setExportProgress({ currentFrame: 0, totalFrames: 0, progress: 0, etaSeconds: 0, state: "failed" });
      setIsExporting(false);
    }
  };

  const handleGenerate = async () => {
    if (isGenerating) return;
    setIsGenerating(true);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const result = await invoke<{
        transformCount: number;
        rippleCount: number;
        keystrokeCount: number;
        cursorCount: number;
        total: number;
      }>("generate_keyframes");

      console.log("Generated keyframes:", result);

      // Reload the timeline from backend to reflect generated keyframes
      await loadTimelineFromBackend();
      setSelectedKeyframe(null);
    } catch (err) {
      console.error("Generate failed:", err);
      alert(`Failed to generate keyframes: ${err}`);
    } finally {
      setIsGenerating(false);
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

  // Keyboard shortcuts: Delete, Undo (Ctrl+Z), Redo (Ctrl+Shift+Z / Ctrl+Y)
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      const isInput = (e.target as HTMLElement).tagName === "INPUT" ||
                      (e.target as HTMLElement).tagName === "SELECT";

      // Undo/redo work even in inputs
      if ((e.ctrlKey || e.metaKey) && e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        undo();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === "Z" || e.key === "y")) {
        e.preventDefault();
        redo();
        return;
      }

      if (isInput) return;

      if (e.key === "Delete" || e.key === "Backspace") {
        handleDeleteKeyframe();
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [handleDeleteKeyframe, undo, redo]);

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
            className="generate-btn"
            onClick={handleGenerate}
            disabled={isGenerating}
          >
            {isGenerating ? "Generating..." : "Generate"}
          </button>
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
                : `Exporting ${exportProgress.currentFrame}/${exportProgress.totalFrames} — ${Math.round(exportProgress.progress * 100)}%${exportProgress.etaSeconds > 0 ? ` (${Math.ceil(exportProgress.etaSeconds)}s left)` : ""}`}
            </span>
          </div>
        )}
        <div className="editor-main">
          <VideoPreview playheadTime={playheadTime} duration={duration} mousePositions={mousePositions} />
          <InspectorPanel selection={selectedKeyframe} onUpdateKeyframe={handleUpdateKeyframe} />
        </div>

        <div className="timeline-panel"
          onWheel={(e) => {
            if (e.ctrlKey || e.metaKey) {
              e.preventDefault();
              setTimelineZoom(prev => Math.max(1, Math.min(20, prev * (e.deltaY < 0 ? 1.15 : 0.87))));
            }
          }}
        >
          <div className="timeline-zoom-bar">
            <span className="timeline-zoom-label">Zoom: {Math.round(timelineZoom * 100)}%</span>
            <input
              type="range"
              min={1} max={20} step={0.1}
              value={timelineZoom}
              onChange={(e) => setTimelineZoom(parseFloat(e.target.value))}
              className="timeline-zoom-slider"
            />
            <button className="timeline-zoom-reset" onClick={() => setTimelineZoom(1)}>Reset</button>
          </div>
          <div className="timeline-scrollable" style={{ overflowX: timelineZoom > 1 ? "auto" : "hidden" }}>
            <div style={{ width: `${timelineZoom * 100}%`, minWidth: "100%" }}>
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
                    onMoveKeyframe={(kfId, newTime) => {
                      handleUpdateKeyframe(kfId, "time", newTime);
                    }}
                  />
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// =============================================================================
// Video Preview
// =============================================================================

interface FrameData {
  width: number;
  height: number;
  rgbaBase64: string;
}

/// Interpolate cursor position from mouse data at a given time.
/// Falls back to simulated sine wave when no data is available.
function interpolateCursor(
  mousePositions: MousePositionData[],
  time: number,
  progress: number,
): { x: number; y: number } {
  if (mousePositions.length === 0) {
    // Simulated fallback
    return {
      x: 30 + 40 * progress,
      y: 40 + 20 * Math.sin(progress * Math.PI * 2),
    };
  }

  // Binary search for bounding samples
  let lo = 0, hi = mousePositions.length - 1;
  if (time <= mousePositions[lo].time) return { x: mousePositions[lo].x * 100, y: mousePositions[lo].y * 100 };
  if (time >= mousePositions[hi].time) return { x: mousePositions[hi].x * 100, y: mousePositions[hi].y * 100 };

  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (mousePositions[mid].time <= time) lo = mid;
    else hi = mid;
  }

  const a = mousePositions[lo];
  const b = mousePositions[hi];
  const t = b.time > a.time ? (time - a.time) / (b.time - a.time) : 0;
  return {
    x: (a.x + (b.x - a.x) * t) * 100,
    y: (a.y + (b.y - a.y) * t) * 100,
  };
}

function VideoPreview({
  playheadTime,
  duration,
  mousePositions,
}: {
  playheadTime: number;
  duration: number;
  mousePositions: MousePositionData[];
}) {
  const progress = duration > 0 ? playheadTime / duration : 0;
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const lastFetchTime = useRef(-1);
  const fetchTimer = useRef<number | null>(null);

  // Throttled frame extraction — fetch at most every 100ms
  useEffect(() => {
    if (fetchTimer.current) clearTimeout(fetchTimer.current);
    fetchTimer.current = window.setTimeout(async () => {
      // Skip if time hasn't changed enough (avoid redundant fetches)
      if (Math.abs(playheadTime - lastFetchTime.current) < 0.05) return;
      lastFetchTime.current = playheadTime;

      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const frame = await invoke<FrameData>("extract_preview_frame", { time: playheadTime });
        const canvas = canvasRef.current;
        if (!canvas || !frame) return;

        canvas.width = frame.width;
        canvas.height = frame.height;
        const ctx = canvas.getContext("2d");
        if (!ctx) return;

        // Decode base64 RGBA data and draw to canvas
        const binary = atob(frame.rgbaBase64);
        const bytes = new Uint8ClampedArray(binary.length);
        for (let i = 0; i < binary.length; i++) {
          bytes[i] = binary.charCodeAt(i);
        }
        const imageData = new ImageData(bytes, frame.width, frame.height);
        ctx.putImageData(imageData, 0, 0);
      } catch {
        // Silently fall back to simulated preview
      }
    }, 100);
    return () => { if (fetchTimer.current) clearTimeout(fetchTimer.current); };
  }, [playheadTime]);

  const cursor = interpolateCursor(mousePositions, playheadTime, progress);

  return (
    <div className="preview-area">
      <div className="video-preview">
        <div className="preview-canvas">
          <canvas ref={canvasRef} className="preview-canvas-element" />
          {/* Overlay: viewport indicator and cursor */}
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
            left: `${cursor.x}%`,
            top: `${cursor.y}%`,
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

interface RenderSettingsData {
  outputResolution: { type: string; width?: number; height?: number };
  outputFrameRate: { type: string; fps?: number };
  codec: string;
  quality: string;
  backgroundEnabled: boolean;
  cornerRadius: number;
  shadowRadius: number;
  shadowOpacity: number;
  padding: number;
  windowInset: number;
}

const RESOLUTION_OPTIONS = [
  { label: "Original", value: "original" },
  { label: "4K (3840x2160)", value: "uhd4k" },
  { label: "1440p", value: "qhd1440" },
  { label: "1080p", value: "fhd1080" },
  { label: "720p", value: "hd720" },
];

const CODEC_OPTIONS = [
  { label: "H.265 (HEVC)", value: "h265" },
  { label: "H.264", value: "h264" },
];

const QUALITY_OPTIONS = [
  { label: "High", value: "high" },
  { label: "Medium", value: "medium" },
  { label: "Low", value: "low" },
  { label: "Original", value: "original" },
];

const FRAMERATE_OPTIONS = [
  { label: "Original", value: "original" },
  { label: "60 fps", value: "60" },
  { label: "30 fps", value: "30" },
];

function RenderSettingsPanel() {
  const [settings, setSettings] = useState<RenderSettingsData | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const s = await invoke<RenderSettingsData>("get_render_settings");
        setSettings(s);
      } catch {
        // No project loaded
      }
    })();
  }, []);

  const saveSettings = async (updated: RenderSettingsData) => {
    setSettings(updated);
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("update_render_settings", { settings: updated });
    } catch (err) {
      console.error("Failed to save render settings:", err);
    }
  };

  if (!settings) {
    return (
      <div className="render-settings">
        <div className="kf-empty-note">No project loaded</div>
      </div>
    );
  }

  const resolutionValue = settings.outputResolution.type;
  const frameRateValue = settings.outputFrameRate.type === "fixed"
    ? String(settings.outputFrameRate.fps ?? 60)
    : "original";

  return (
    <div className="render-settings">
      <label className="section-label">Output</label>
      <div className="property-row">
        <span className="property-label">Resolution</span>
        <select className="property-select" value={resolutionValue}
          onChange={(e) => saveSettings({
            ...settings,
            outputResolution: { type: e.target.value },
          })}>
          {RESOLUTION_OPTIONS.map(o => <option key={o.value} value={o.value}>{o.label}</option>)}
        </select>
      </div>
      <div className="property-row">
        <span className="property-label">Codec</span>
        <select className="property-select" value={settings.codec}
          onChange={(e) => saveSettings({ ...settings, codec: e.target.value })}>
          {CODEC_OPTIONS.map(o => <option key={o.value} value={o.value}>{o.label}</option>)}
        </select>
      </div>
      <div className="property-row">
        <span className="property-label">Quality</span>
        <select className="property-select" value={settings.quality}
          onChange={(e) => saveSettings({ ...settings, quality: e.target.value })}>
          {QUALITY_OPTIONS.map(o => <option key={o.value} value={o.value}>{o.label}</option>)}
        </select>
      </div>
      <div className="property-row">
        <span className="property-label">Frame Rate</span>
        <select className="property-select" value={frameRateValue}
          onChange={(e) => {
            const v = e.target.value;
            saveSettings({
              ...settings,
              outputFrameRate: v === "original"
                ? { type: "original" }
                : { type: "fixed", fps: parseInt(v) },
            });
          }}>
          {FRAMERATE_OPTIONS.map(o => <option key={o.value} value={o.value}>{o.label}</option>)}
        </select>
      </div>

      <label className="section-label">Window Mode</label>
      <div className="property-row">
        <span className="property-label">Background</span>
        <input type="checkbox" checked={settings.backgroundEnabled}
          onChange={(e) => saveSettings({ ...settings, backgroundEnabled: e.target.checked })} />
      </div>
      <div className="property-row">
        <span className="property-label">Corner Radius</span>
        <input className="property-input" type="number" step={1} min={0} max={100}
          value={settings.cornerRadius}
          onChange={(e) => saveSettings({ ...settings, cornerRadius: parseFloat(e.target.value) || 0 })} />
      </div>
      <div className="property-row">
        <span className="property-label">Shadow Opacity</span>
        <input className="property-input" type="number" step={0.1} min={0} max={1}
          value={settings.shadowOpacity}
          onChange={(e) => saveSettings({ ...settings, shadowOpacity: parseFloat(e.target.value) || 0 })} />
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
  onMoveKeyframe,
}: {
  track: Track;
  duration: number;
  playheadTime: number;
  selectedId?: string;
  onSelectKeyframe?: (kf: Keyframe) => void;
  onAddKeyframe?: () => void;
  onMoveKeyframe?: (keyframeId: string, newTime: number) => void;
}) {
  const color = TRACK_COLORS[track.type] || "#888";
  const laneRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<{ kfId: string; startX: number } | null>(null);

  const handleMouseDown = (e: React.MouseEvent, kf: Keyframe) => {
    e.stopPropagation();
    onSelectKeyframe?.(kf);
    dragRef.current = { kfId: kf.id, startX: e.clientX };

    const handleMouseMove = (me: MouseEvent) => {
      if (!dragRef.current || !laneRef.current) return;
      const rect = laneRef.current.getBoundingClientRect();
      const ratio = Math.max(0, Math.min(1, (me.clientX - rect.left) / rect.width));
      const newTime = Math.round(ratio * duration * 100) / 100; // snap to 10ms
      onMoveKeyframe?.(dragRef.current.kfId, newTime);
    };

    const handleMouseUp = () => {
      dragRef.current = null;
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  };

  return (
    <div className="timeline-track">
      <div className="track-label" style={{ borderLeftColor: color }}>
        {track.name}
      </div>
      <div className="track-lane" ref={laneRef} onDoubleClick={() => onAddKeyframe?.()}>
        {track.keyframes.map(kf => (
          <div
            key={kf.id}
            className={`keyframe-marker ${kf.id === selectedId ? "selected" : ""}`}
            style={{
              left: `${(kf.time / duration) * 100}%`,
              backgroundColor: color,
            }}
            title={`${track.name} @ ${formatTimecode(kf.time)}`}
            onMouseDown={(e) => handleMouseDown(e, kf)}
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
