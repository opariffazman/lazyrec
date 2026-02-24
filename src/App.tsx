import { useState } from "react";
import "./App.css";

type Screen = "welcome" | "recording" | "editor";

function App() {
  const [screen, setScreen] = useState<Screen>("welcome");

  return (
    <main className="app">
      {screen === "welcome" && (
        <WelcomeScreen
          onStartRecording={() => setScreen("recording")}
        />
      )}
      {screen === "recording" && (
        <div className="placeholder-screen">
          <h2>Recording</h2>
          <p>Recording UI coming soon...</p>
          <button onClick={() => setScreen("welcome")}>Back</button>
        </div>
      )}
      {screen === "editor" && (
        <div className="placeholder-screen">
          <h2>Editor</h2>
          <p>Timeline editor coming soon...</p>
          <button onClick={() => setScreen("welcome")}>Back</button>
        </div>
      )}
    </main>
  );
}

function WelcomeScreen({ onStartRecording }: { onStartRecording: () => void }) {
  const [isDragging, setIsDragging] = useState(false);

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    const files = Array.from(e.dataTransfer.files);
    const file = files[0];
    if (file) {
      console.log("Dropped file:", file.name);
      // TODO: handle video/project file opening
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
          onClick={() => console.log("TODO: open video")}
        />
        <ActionCard
          icon="📁"
          title="Open Project"
          description="Continue editing"
          color="#e09145"
          onClick={() => console.log("TODO: open project")}
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

export default App;
