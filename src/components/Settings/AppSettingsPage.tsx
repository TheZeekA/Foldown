import { useSettingsStore } from "../../stores/settings";
import { applyEditorFont, applyTheme, DEFAULT_EDITOR_FONT_FAMILY } from "../../lib/theme";
import type { ThemeMode } from "../../lib/types";
import { useEffect, useState } from "react";
import { persistWithRollback } from "./settingsBehavior";

const THEME_OPTIONS: { label: string; value: ThemeMode }[] = [
  { label: "System", value: "system" },
  { label: "Light", value: "light" },
  { label: "Dark", value: "dark" },
];

const FONT_FAMILY_OPTIONS = [
  { label: "Default", value: DEFAULT_EDITOR_FONT_FAMILY },
  { label: "Consolas", value: "Consolas, monospace" },
  { label: "Cascadia Code", value: '"Cascadia Code", monospace' },
  { label: "Fira Code", value: '"Fira Code", monospace' },
  { label: "JetBrains Mono", value: '"JetBrains Mono", monospace' },
  { label: "Courier New", value: '"Courier New", monospace' },
];

const MIN_FONT_SIZE = 10;
const MAX_FONT_SIZE = 28;

export function AppSettingsPage() {
  const { theme, editorFontFamily, editorFontSize, setTheme, setEditorFont } = useSettingsStore();
  const clampFontSize = (value: number) =>
    Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, value));
  const [status, setStatus] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  // Interim, possibly-invalid text the user is typing into the font-size
  // field. Decoupled from the persisted value so clearing the field or
  // typing a leading digit ("1" before "12") doesn't get silently reverted
  // out from under the user; it snaps back to the persisted value on blur.
  const [fontSizeText, setFontSizeText] = useState(String(editorFontSize));
  useEffect(() => setFontSizeText(String(editorFontSize)), [editorFontSize]);

  const changeTheme = async (next: ThemeMode) => {
    if (saving) return;
    const previous = theme;
    setStatus(null);
    setSaving(true);
    try {
      await persistWithRollback(() => setTheme(next), () => {
        useSettingsStore.setState({ theme: previous });
        applyTheme(previous);
      });
    } catch (error) { setStatus(`Could not save appearance settings: ${String(error)}`); }
    finally { setSaving(false); }
  };

  const changeFont = async (family: string, size: number) => {
    if (saving) return;
    const previous = { family: editorFontFamily, size: editorFontSize };
    setStatus(null);
    setSaving(true);
    try {
      await persistWithRollback(() => setEditorFont(family, size), () => {
        useSettingsStore.setState({ editorFontFamily: previous.family, editorFontSize: previous.size });
        applyEditorFont(previous.family, previous.size);
      });
    } catch (error) { setStatus(`Could not save appearance settings: ${String(error)}`); }
    finally { setSaving(false); }
  };

  return (
    <section className="settings-modal__page" aria-labelledby="settings-app-heading">
      <div className="settings-modal__section">
        <h2 id="settings-app-heading" className="settings-modal__page-heading">App Settings</h2>
        <h3 className="settings-modal__section-title">Appearance</h3>

        <div className="settings-modal__field">
          <span className="settings-modal__field-label">Theme</span>
          <div className="settings-modal__theme-options" role="radiogroup" aria-label="Theme">
            {THEME_OPTIONS.map((option) => (
              <button
                key={option.value}
                className={`settings-modal__theme-button${
                  theme === option.value ? " settings-modal__theme-button--active" : ""
                }`}
                role="radio"
                aria-checked={theme === option.value}
                disabled={saving}
                onClick={() => void changeTheme(option.value)}
              >
                {option.label}
              </button>
            ))}
          </div>
        </div>

        <div className="settings-modal__field">
          <label className="settings-modal__field-label" htmlFor="editor-font-family">
            Editor font
          </label>
          <select
            id="editor-font-family"
            className="settings-modal__select"
            value={editorFontFamily}
            disabled={saving}
            onChange={(e) => void changeFont(e.target.value, editorFontSize)}
          >
            {FONT_FAMILY_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>

        <div className="settings-modal__field">
          <label className="settings-modal__field-label" htmlFor="editor-font-size">
            Font size
          </label>
          <input
            id="editor-font-size"
            className="settings-modal__number-input"
            type="number"
            min={MIN_FONT_SIZE}
            max={MAX_FONT_SIZE}
            value={fontSizeText}
            disabled={saving}
            onChange={(e) => {
              setFontSizeText(e.target.value);
              const parsed = parseInt(e.target.value, 10);
              if (!Number.isNaN(parsed)) {
                void changeFont(editorFontFamily, clampFontSize(parsed));
              }
            }}
            onBlur={() => setFontSizeText(String(editorFontSize))}
          />
        </div>
        {status && <p className="settings-modal__status settings-modal__status--error" role="alert">{status}</p>}
      </div>
    </section>
  );
}
