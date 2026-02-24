import { useCallback, useRef } from "react";

interface ResizeHandleProps {
    onResize: (delta: number) => void;
}

export function ResizeHandle({ onResize }: ResizeHandleProps) {
    const elRef = useRef<HTMLDivElement>(null);
    const rafId = useRef<number | null>(null);

    const handleMouseDown = useCallback(
        (e: React.MouseEvent) => {
            if (e.button !== 0) return;
            e.preventDefault();
            document.body.style.userSelect = "none";
            elRef.current?.classList.add("active");

            const onMouseMove = (moveEvent: MouseEvent) => {
                if (rafId.current != null) cancelAnimationFrame(rafId.current);
                rafId.current = requestAnimationFrame(() => {
                    onResize(moveEvent.movementX);
                    rafId.current = null;
                });
            };

            const onMouseUp = () => {
                document.body.style.userSelect = "";
                document.removeEventListener("mousemove", onMouseMove);
                document.removeEventListener("mouseup", onMouseUp);
                elRef.current?.classList.remove("active");
                if (rafId.current != null) cancelAnimationFrame(rafId.current);
            };

            document.addEventListener("mousemove", onMouseMove);
            document.addEventListener("mouseup", onMouseUp);
        },
        [onResize]
    );

    return (
        <div
            ref={elRef}
            className="resize-handle"
            role="separator"
            aria-orientation="vertical"
            onMouseDown={handleMouseDown}
            style={{ height: "100%" }}
        />
    );
}
