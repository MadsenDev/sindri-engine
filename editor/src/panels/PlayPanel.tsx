import { useState } from "react";
import PlayView from "../components/PlayView";
import type { CameraInfo, PlayState } from "../app/types";

interface PlayPanelProps {
  playState: PlayState;
  renderTick: number;
  camera: CameraInfo | null;
}

const resolutions = [
  { label: "Free", value: null as null | [number, number] },
  { label: "16:9", value: [16, 9] as [number, number] },
  { label: "4:3", value: [4, 3] as [number, number] },
  { label: "1:1", value: [1, 1] as [number, number] },
];

export default function PlayPanel({ playState, renderTick, camera }: PlayPanelProps) {
  const [aspect, setAspect] = useState<null | [number, number]>([16, 9]);

  return (
    <div className="panel dock-panel">
      <div className="panel-header tight">
        <div className="panel-actions">
          <span className="panel-footnote muted">
            {playState === "playing"
              ? "Running"
              : playState === "paused"
              ? "Paused"
              : "Stopped"}
          </span>
          <select
            className="unity-button muted"
            value={aspect ? aspect.join(":") : "free"}
            onChange={(event) => {
              const next =
                resolutions.find((entry) =>
                  entry.value ? entry.value.join(":") === event.target.value : event.target.value === "free"
                ) ?? resolutions[0];
              setAspect(next.value);
            }}
          >
            {resolutions.map((entry) => (
              <option
                key={entry.label}
                value={entry.value ? entry.value.join(":") : "free"}
              >
                {entry.label}
              </option>
            ))}
          </select>
        </div>
      </div>
      <div className="panel-body muted-bg">
        <PlayView
          playState={playState}
          aspectRatio={aspect}
          renderTick={renderTick}
          camera={camera}
        />
      </div>
    </div>
  );
}
