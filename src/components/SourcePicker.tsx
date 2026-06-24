import type { AnimeSourceId } from "../types";
import { sourceLabel } from "../utils/source";
import { Icon } from "./Icon";

const SOURCES: AnimeSourceId[] = ["sushianimes", "goyabu"];

interface SourcePickerProps {
  value: AnimeSourceId;
  onChange: (source: AnimeSourceId) => void;
}

export function SourcePicker({ value, onChange }: SourcePickerProps) {
  return (
    <div className="source-picker">
      <span className="source-picker-label">
        <Icon name="fa-database" /> Fonte
      </span>
      <div className="source-picker-chips">
        {SOURCES.map((id) => (
          <button
            key={id}
            type="button"
            className={`source-chip ${value === id ? "active" : ""}`}
            onClick={() => onChange(id)}
          >
            {sourceLabel(id)}
          </button>
        ))}
      </div>
    </div>
  );
}
