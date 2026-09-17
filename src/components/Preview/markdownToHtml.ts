import { convertFileSrc } from "@tauri-apps/api/core";
import rehypeSanitize from "rehype-sanitize";
import rehypeStringify from "rehype-stringify";
import remarkGfm from "remark-gfm";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { unified } from "unified";
import { replaceLocalImageSources } from "./imageUrls";

export interface MarkdownRenderContext {
  openPath: string | null;
  workspaceRoot: string | null;
}

const processor = unified()
  .use(remarkParse)
  .use(remarkGfm)
  .use(remarkRehype)
  .use(rehypeSanitize)
  .use(rehypeStringify);

export async function markdownToHtml(
  body: string,
  { openPath, workspaceRoot }: MarkdownRenderContext,
  toAssetUrl: (path: string) => string = convertFileSrc,
): Promise<string> {
  const rendered = String(await processor.process(body));
  return openPath && workspaceRoot
    ? replaceLocalImageSources(rendered, openPath, workspaceRoot, toAssetUrl)
    : rendered;
}
