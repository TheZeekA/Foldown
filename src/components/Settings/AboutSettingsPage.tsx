import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { BrandMark } from "../BrandMark";
import { ABOUT_DEVELOPER, ABOUT_EMAIL, formatVersion } from "./aboutMetadata";

export function AboutSettingsPage() {
  const [version, setVersion] = useState("Loading version…");
  const [contactError, setContactError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;

    void getVersion()
      .then((runningVersion) => {
        if (isMounted) setVersion(formatVersion(runningVersion));
      })
      .catch(() => {
        if (isMounted) setVersion("Version unavailable");
      });

    return () => {
      isMounted = false;
    };
  }, []);

  return (
    <section className="settings-modal__page settings-modal__about-page" aria-labelledby="settings-about-heading">
      <div className="settings-modal__section">
        <h2 id="settings-about-heading" className="settings-modal__page-heading">About</h2>
        <div className="settings-modal__about-branding">
          <BrandMark size={40} withWordmark />
          <p className="settings-modal__about-version">{version}</p>
        </div>

        <dl className="settings-modal__about-metadata">
          <div>
            <dt>Developer</dt>
            <dd>{ABOUT_DEVELOPER}</dd>
          </div>
          <div>
            <dt>Contact</dt>
            <dd>
              <button
                className="settings-modal__about-contact"
                type="button"
                onClick={() => {
                  setContactError(null);
                  void openUrl(`mailto:${ABOUT_EMAIL}`).catch((error) => {
                    setContactError(`Could not open your email application: ${String(error)}`);
                  });
                }}
              >
                {ABOUT_EMAIL}
              </button>
            </dd>
          </div>
        </dl>
        {contactError && <p className="settings-modal__status settings-modal__status--error" role="alert">{contactError}</p>}

      </div>
    </section>
  );
}
