use std::path::Path;

fn validate_pdf_output_path(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("PDF output path must be absolute".into());
    }
    if path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("pdf"))
        != Some(true)
    {
        return Err("PDF output path must end in .pdf".into());
    }
    Ok(())
}

#[cfg(windows)]
#[tauri::command]
pub async fn export_webview_to_pdf(
    webview: tauri::WebviewWindow,
    path: String,
) -> Result<(), String> {
    use std::{
        os::windows::ffi::OsStrExt,
        sync::{Arc, Mutex},
    };
    use webview2_com::{
        Microsoft::Web::WebView2::Win32::{ICoreWebView2PrintSettings, ICoreWebView2_7},
        PrintToPdfCompletedHandler,
    };
    use windows::core::{Interface, PCWSTR};

    let output = Path::new(&path);
    validate_pdf_output_path(output)?;
    let wide_path: Vec<u16> = output.as_os_str().encode_wide().chain(Some(0)).collect();
    let (sender, receiver) = tokio::sync::oneshot::channel::<Result<(), String>>();
    let sender = Arc::new(Mutex::new(Some(sender)));
    let setup_sender = sender.clone();

    webview
        .with_webview(move |platform_webview| {
            let callback_sender = sender.clone();
            let result: windows::core::Result<()> = (|| unsafe {
                let controller = platform_webview.controller();
                let core = controller.CoreWebView2()?;
                let printable: ICoreWebView2_7 = core.cast()?;
                let handler =
                    PrintToPdfCompletedHandler::create(Box::new(move |error_code, success| {
                        let result = match error_code {
                            Err(error) => Err(format!("WebView2 PDF export failed: {error}")),
                            Ok(()) if success => Ok(()),
                            Ok(()) => Err("WebView2 could not write the PDF".into()),
                        };
                        if let Some(sender) = callback_sender.lock().unwrap().take() {
                            let _ = sender.send(result);
                        }
                        Ok(())
                    }));
                printable.PrintToPdf(
                    PCWSTR(wide_path.as_ptr()),
                    None::<&ICoreWebView2PrintSettings>,
                    &handler,
                )
            })();
            if let Err(error) = result {
                if let Some(sender) = setup_sender.lock().unwrap().take() {
                    let _ =
                        sender.send(Err(format!("Could not start WebView2 PDF export: {error}")));
                }
            }
        })
        .map_err(|error| format!("Could not access the PDF export webview: {error}"))?;

    receiver
        .await
        .map_err(|_| "WebView2 PDF export ended before completion".to_string())?
}

#[cfg(not(windows))]
#[tauri::command]
pub async fn export_webview_to_pdf(
    _webview: tauri::WebviewWindow,
    _path: String,
) -> Result<(), String> {
    Err("Direct PDF export is only available on Windows".into())
}

#[cfg(test)]
mod tests {
    use super::validate_pdf_output_path;
    use std::path::Path;

    #[test]
    fn accepts_an_absolute_pdf_path() {
        let path = if cfg!(windows) {
            Path::new(r"C:\notes\report.pdf")
        } else {
            Path::new("/tmp/report.pdf")
        };
        assert!(validate_pdf_output_path(path).is_ok());
    }

    #[test]
    fn rejects_relative_and_non_pdf_paths() {
        assert!(validate_pdf_output_path(Path::new("report.pdf")).is_err());
        let path = if cfg!(windows) {
            Path::new(r"C:\notes\report.txt")
        } else {
            Path::new("/tmp/report.txt")
        };
        assert!(validate_pdf_output_path(path).is_err());
    }
}
