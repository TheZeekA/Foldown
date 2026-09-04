import { useEffect, useRef, useState } from "react";
import "./SettingsModal.css";
import { AiSettingsPage } from "./AiSettingsPage";
import { AppSettingsPage } from "./AppSettingsPage";
import { AboutSettingsPage } from "./AboutSettingsPage";
import {
  normalizeInitialSettingsPage,
  SETTINGS_PAGES,
  type SettingsPageId,
} from "./settingsNavigation";
import { ToolsSettingsPage } from "./ToolsSettingsPage";
import { nextFocusIndex } from "./settingsBehavior";

interface SettingsModalProps {
  onClose: () => void;
  initialPage?: SettingsPageId;
}

export function SettingsModal({ onClose, initialPage }: SettingsModalProps) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const [activePage, setActivePage] = useState<SettingsPageId>(() =>
    normalizeInitialSettingsPage(initialPage),
  );

  useEffect(() => {
    const previouslyFocused = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    closeRef.current?.focus();
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
      if (event.key !== "Tab" || !dialogRef.current) return;
      const focusable = Array.from(dialogRef.current.querySelectorAll<HTMLElement>(
        'button:not(:disabled), input:not(:disabled), select:not(:disabled), summary, [href], [tabindex]:not([tabindex="-1"])',
      )).filter((element) => !element.closest("[hidden]") && element.offsetParent !== null);
      if (!focusable.length) return;
      const current = focusable.indexOf(document.activeElement as HTMLElement);
      focusable[nextFocusIndex(focusable.length, current, event.shiftKey)]?.focus();
      event.preventDefault();
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      previouslyFocused?.focus();
    };
  }, [onClose]);

  useEffect(() => { if (contentRef.current) contentRef.current.scrollTop = 0; }, [activePage]);

  return (
    <div className="settings-modal__overlay" onMouseDown={onClose}>
      <div
        ref={dialogRef}
        className="settings-modal"
        role="dialog"
        aria-modal="true"
        aria-label="Settings"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="settings-modal__header">
          <h2 className="settings-modal__title">Settings</h2>
          <button ref={closeRef} className="settings-modal__close" onClick={onClose} aria-label="Close settings">
            ×
          </button>
        </div>

        <div className="settings-modal__body">
          <nav className="settings-modal__navigation" aria-label="Settings pages">
            {SETTINGS_PAGES.map((page) => (
              <button
                key={page.id}
                className={`settings-modal__navigation-button${
                  activePage === page.id ? " settings-modal__navigation-button--active" : ""
                }`}
                aria-current={activePage === page.id ? "page" : undefined}
                onClick={() => setActivePage(page.id)}
              >
                {page.label}
              </button>
            ))}
          </nav>

          <div ref={contentRef} className="settings-modal__content">
            <div hidden={activePage !== "app"}>
              <AppSettingsPage />
            </div>
            <div hidden={activePage !== "ai"}>
              <AiSettingsPage />
            </div>
            <div hidden={activePage !== "tools"}>
              <ToolsSettingsPage />
            </div>
            <div hidden={activePage !== "about"}>
              <AboutSettingsPage />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default SettingsModal;
