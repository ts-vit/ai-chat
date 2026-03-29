import { useCallback, useEffect, useRef, useState } from "react";

export type SplitDirection = "horizontal" | "vertical";

export interface ResizablePanelProps {
  /** First panel content */
  first: React.ReactNode;
  /** Second panel content */
  second: React.ReactNode;
  /** Split direction: "horizontal" = left|right, "vertical" = top|bottom. Default: "horizontal" */
  direction?: SplitDirection;
  /** Initial split position in percent (0-100). Default: 50 */
  initialPosition?: number;
  /** Minimum size of each panel in percent. Default: 20 */
  minPosition?: number;
  /** Maximum size of first panel in percent. Default: 80 */
  maxPosition?: number;
  /** Divider width/height in px. Default: 5 */
  dividerSize?: number;
  /** Whether resizing is enabled. Default: true */
  resizable?: boolean;
  /** Called when split position changes (percent) */
  onPositionChange?: (position: number) => void;
  /** Container style override */
  style?: React.CSSProperties;
}

export function ResizablePanel({
  first,
  second,
  direction = "horizontal",
  initialPosition = 50,
  minPosition = 20,
  maxPosition = 80,
  dividerSize = 5,
  resizable = true,
  onPositionChange,
  style,
}: ResizablePanelProps) {
  const [position, setPosition] = useState(initialPosition);
  const [isDragging, setIsDragging] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const isHorizontal = direction === "horizontal";

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (!resizable || e.button !== 0) return;
      e.preventDefault();
      setIsDragging(true);
    },
    [resizable],
  );

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      let percent: number;
      if (isHorizontal) {
        percent = ((e.clientX - rect.left) / rect.width) * 100;
      } else {
        percent = ((e.clientY - rect.top) / rect.height) * 100;
      }
      const clamped = Math.min(maxPosition, Math.max(minPosition, percent));
      setPosition(clamped);
      onPositionChange?.(clamped);
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    document.body.style.cursor = isHorizontal ? "col-resize" : "row-resize";
    document.body.style.userSelect = "none";

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
    return () => {
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isDragging, isHorizontal, minPosition, maxPosition, onPositionChange]);

  const containerStyle: React.CSSProperties = {
    display: "flex",
    flexDirection: isHorizontal ? "row" : "column",
    overflow: "hidden",
    ...style,
  };

  const firstStyle: React.CSSProperties = isHorizontal
    ? {
        width: `${position}%`,
        minWidth: 0,
        overflow: "hidden",
        display: "flex",
        flexDirection: "column",
      }
    : {
        height: `${position}%`,
        minHeight: 0,
        overflow: "hidden",
        display: "flex",
        flexDirection: "column",
      };

  const secondStyle: React.CSSProperties = {
    flex: 1,
    minWidth: 0,
    minHeight: 0,
    overflow: "hidden",
    display: "flex",
    flexDirection: "column",
  };

  const dividerStyle: React.CSSProperties = {
    flexShrink: 0,
    backgroundColor: isDragging
      ? "var(--mantine-color-brand-5)"
      : isHovered
        ? "var(--mantine-color-dark-3)"
        : "var(--mantine-color-default-border)",
    transition: isDragging ? "none" : "background-color 0.15s",
    ...(isHorizontal
      ? {
          width: dividerSize,
          cursor: resizable ? "col-resize" : "default",
        }
      : {
          height: dividerSize,
          cursor: resizable ? "row-resize" : "default",
        }),
  };

  return (
    <div ref={containerRef} style={containerStyle}>
      <div style={firstStyle}>{first}</div>
      <div
        role="separator"
        aria-orientation={isHorizontal ? "vertical" : "horizontal"}
        style={dividerStyle}
        onMouseDown={handleMouseDown}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
      />
      <div style={secondStyle}>{second}</div>
    </div>
  );
}
