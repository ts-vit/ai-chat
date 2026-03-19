/**
 * Extract all unique citation indices [N] from text.
 * Returns sorted 1-based indices. Used to determine which sources were actually cited.
 */
export function extractCitationIndices(text: string): number[] {
    const regex = /\[(\d{1,3})\]/g;
    const indices = new Set<number>();
    let match;
    while ((match = regex.exec(text)) !== null) {
        const n = parseInt(match[1], 10);
        if (n >= 1) indices.add(n);
    }
    return [...indices].sort((a, b) => a - b);
}
