import { convertFileSrc } from "@tauri-apps/api/core";
import type { Element, Node, Root } from "hast";
import rehypeSanitize from "rehype-sanitize";
import rehypeStringify from "rehype-stringify";
import remarkGfm from "remark-gfm";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { unified } from "unified";
import { isProtocolRelative, resolveLocalImagePath } from "./imageUrls";

export interface MarkdownRenderContext {
  openPath: string | null;
  workspaceRoot: string | null;
}

function isElement(node: Node): node is Element {
  return node.type === "element";
}

/**
 * Rewrites local `<img src>` values to their Tauri asset URL before
 * `rehype-sanitize` runs. This must happen pre-sanitize: an absolute local
 * path (a Windows drive letter) is otherwise indistinguishable from a URI
 * scheme and gets its `src` stripped by the sanitizer before any
 * post-processing step could see it. A protocol-relative value
 * (`//host/path`) is explicitly neutralized rather than left for the
 * sanitizer, whose protocol allowlist has no concept of "no scheme" versus
 * "relative to the current origin" and lets it through unfiltered — which
 * would otherwise let a malicious document reach Foldown's local-file-serving
 * asset protocol under the guise of an "external" image.
 */
function resolveLocalImages(context: MarkdownRenderContext, toAssetUrl: (path: string) => string) {
  return (tree: Root) => {
    const walk = (node: Node) => {
      if (isElement(node) && node.tagName === "img" && typeof node.properties.src === "string") {
        const src = node.properties.src.trim();
        const resolved = context.openPath && context.workspaceRoot
          ? resolveLocalImagePath(src, context.openPath, context.workspaceRoot)
          : null;
        if (resolved) {
          node.properties.src = toAssetUrl(resolved);
        } else if (isProtocolRelative(src)) {
          delete node.properties.src;
        }
      }
      if ("children" in node) (node as { children: Node[] }).children.forEach(walk);
    };
    walk(tree);
  };
}

export async function markdownToHtml(
  body: string,
  context: MarkdownRenderContext,
  toAssetUrl: (path: string) => string = convertFileSrc,
): Promise<string> {
  const processor = unified()
    .use(remarkParse)
    .use(remarkGfm)
    .use(remarkRehype)
    .use(() => resolveLocalImages(context, toAssetUrl))
    .use(rehypeSanitize)
    .use(rehypeStringify);
  return String(await processor.process(body));
}
