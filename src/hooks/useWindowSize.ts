import { useState, useEffect } from "react";

export function useWindowSize() {
    const [width, setWidth] = useState(window.innerWidth);

    useEffect(() => {
        const handler = () => setWidth(window.innerWidth);
        window.addEventListener("resize", handler);
        return () => window.removeEventListener("resize", handler);
    }, []);

    return {
        width,
        isCompact: width < 1100,
        isNarrow: width < 900,
        isVeryNarrow: width < 800,
    };
}
