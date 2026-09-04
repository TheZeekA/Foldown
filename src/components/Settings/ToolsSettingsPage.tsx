import { useState } from "react";
import { baseName } from "../../lib/paths";
import {
  bulkConvertDocuments,
  convertDocument,
  pickConvertSource,
  pickConvertSources,
  pickDestinationFolder,
  pickMarkdownSavePath,
} from "../../lib/tauriApi";

type ConvertStatus = { kind: "success" | "error"; message: string };

export function ToolsSettingsPage() {
  const [converting, setConverting] = useState(false);
  const [convertStatus, setConvertStatus] = useState<ConvertStatus | null>(null);

  const handleConvertOne = async () => {
    setConvertStatus(null);
    try {
      const source = await pickConvertSource();
      if (!source) return;
      const stem = baseName(source).replace(/\.[^.]+$/, "");
      const dest = await pickMarkdownSavePath(`${stem}.md`);
      if (!dest) return;
      setConverting(true);
      await convertDocument(source, dest);
      setConvertStatus({ kind: "success", message: `Converted to "${baseName(dest)}".` });
    } catch (error) {
      setConvertStatus({ kind: "error", message: String(error) });
    } finally {
      setConverting(false);
    }
  };

  const handleConvertBulk = async () => {
    setConvertStatus(null);
    try {
      const sources = await pickConvertSources();
      if (sources.length === 0) return;
      const destDir = await pickDestinationFolder();
      if (!destDir) return;
      setConverting(true);
      const results = await bulkConvertDocuments(sources, destDir);
      const failed = results.filter((result) => result.error);
      const succeeded = results.length - failed.length;
      if (failed.length === 0) {
        setConvertStatus({
          kind: "success",
          message: `Converted ${succeeded} file${succeeded === 1 ? "" : "s"} to "${baseName(destDir)}".`,
        });
      } else {
        const names = failed.map((result) => baseName(result.source_path)).join(", ");
        setConvertStatus({
          kind: "error",
          message: `Converted ${succeeded} of ${results.length}. Failed: ${names}.`,
        });
      }
    } catch (error) {
      setConvertStatus({ kind: "error", message: String(error) });
    } finally {
      setConverting(false);
    }
  };

  return (
    <section className="settings-modal__page" aria-labelledby="settings-tools-heading">
      <div className="settings-modal__section">
        <h2 id="settings-tools-heading" className="settings-modal__page-heading">Tools</h2>
        <h3 className="settings-modal__section-title">Document conversion</h3>
        <button className="settings-modal__item settings-modal__item--accent" onClick={handleConvertOne} disabled={converting}>
          Convert Document to Markdown
        </button>
        <button className="settings-modal__item settings-modal__item--accent" onClick={handleConvertBulk} disabled={converting}>
          Bulk Convert to Markdown
        </button>
        <p className="settings-modal__hint">Supports .txt, .html, .csv, and .docx files.</p>
        {convertStatus && (
          <p role={convertStatus.kind === "error" ? "alert" : "status"} className={`settings-modal__status${convertStatus.kind === "error" ? " settings-modal__status--error" : ""}`}>
            {convertStatus.message}
          </p>
        )}
      </div>
    </section>
  );
}
