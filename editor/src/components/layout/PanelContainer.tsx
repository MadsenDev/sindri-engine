import { ReactNode } from "react";
import "./PanelContainer.css";

export interface PanelContainerProps {
  layout: "horizontal" | "vertical" | "grid";
  children: ReactNode;
  gap?: number;
}

export default function PanelContainer({ layout, children, gap = 8 }: PanelContainerProps) {
  // This is a wrapper - the actual panel management will be in App.tsx
  return (
    <div className={`panel-container panel-container-${layout}`} style={{ gap }}>
      {children}
    </div>
  );
}

