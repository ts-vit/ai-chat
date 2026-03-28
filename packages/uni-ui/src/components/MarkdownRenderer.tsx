import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import type { Components } from "react-markdown";

export interface MarkdownRendererProps {
  /** Markdown content to render */
  content: string;
  /** Additional CSS class for the wrapper */
  className?: string;
  /** Custom remarkPlugins (appended to defaults) */
  remarkPlugins?: any[];
  /** Custom rehypePlugins (appended to defaults) */
  rehypePlugins?: any[];
  /** Custom components override for ReactMarkdown */
  components?: Components;
}

/**
 * Markdown renderer with GFM tables/links and syntax highlighting.
 * Wraps react-markdown + remark-gfm + rehype-highlight.
 *
 * Import markdown.css from @uni-fw/ui for highlight.js theming:
 * ```
 * import "@uni-fw/ui/src/styles/markdown.css";
 * ```
 */
export function MarkdownRenderer({
  content,
  className,
  remarkPlugins = [],
  rehypePlugins = [],
  components,
}: MarkdownRendererProps) {
  return (
    <div className={className}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm, ...remarkPlugins]}
        rehypePlugins={[rehypeHighlight, ...rehypePlugins]}
        components={components}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
