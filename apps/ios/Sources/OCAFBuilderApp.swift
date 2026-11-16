import SwiftUI
import WebKit
import UIKit

/// SwiftUI wrapper for the existing offline Code/NoCode editor.
/// No privileged JS bridge other than user-initiated, bounded JSON export.
@main
struct OCAFBuilderApp: App {
    var body: some Scene {
        WindowGroup {
            VStack(spacing: 0) {
                Text("OCAF Agents Builder · Code / NoCode · lokalnie")
                    .font(.headline)
                    .foregroundColor(Color(red: 212.0/255.0, green: 175.0/255.0, blue: 55.0/255.0))
                    .padding(12)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color(red: 7.0/255.0, green: 8.0/255.0, blue: 10.0/255.0))
                BuilderWebView()
            }
        }
    }
}

struct BuilderWebView: UIViewRepresentable {
    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeUIView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.defaultWebpagePreferences.allowsContentJavaScript = true
        config.userContentController.add(context.coordinator, name: "ocafHost")
        let view = WKWebView(frame: .zero, configuration: config)
        view.navigationDelegate = context.coordinator
        context.coordinator.webView = view
        view.backgroundColor = UIColor(red: 7.0/255.0, green: 8.0/255.0, blue: 10.0/255.0, alpha: 1.0)
        view.isOpaque = false
        if let url = Bundle.main.url(forResource: "index", withExtension: "html", subdirectory: "builder-web") {
            let dir = url.deletingLastPathComponent().standardizedFileURL
            context.coordinator.allowedDirectory = dir
            view.loadFileURL(url, allowingReadAccessTo: dir)
        } else {
            view.loadHTMLString("<h1>Brak plików lokalnego edytora.</h1>", baseURL: nil)
        }
        return view
    }

    func updateUIView(_ uiView: WKWebView, context: Context) {}

    final class Coordinator: NSObject, WKNavigationDelegate, WKScriptMessageHandler {
        weak var webView: WKWebView?
        var allowedDirectory: URL?

        func webView(
            _ webView: WKWebView,
            decidePolicyFor navigationAction: WKNavigationAction,
            decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
        ) {
            guard let url = navigationAction.request.url,
                  url.isFileURL,
                  let dir = allowedDirectory else {
                decisionHandler(.cancel)
                return
            }
            decisionHandler(url.standardizedFileURL.path.hasPrefix(dir.path + "/") ? .allow : .cancel)
        }

        func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
            guard message.name == "ocafHost", message.frameInfo.isMainFrame,
                  let source = message.frameInfo.request.url,
                  source.isFileURL,
                  let dir = allowedDirectory,
                  source.standardizedFileURL.path.hasPrefix(dir.path + "/"),
                  let json = message.body as? String,
                  json.utf8.count < 150_000,
                  let raw = json.data(using: .utf8),
                  let payload = try? JSONSerialization.jsonObject(with: raw) as? [String: Any],
                  let content = payload["content"] as? String,
                  let data = content.data(using: .utf8),
                  data.count <= 128 * 1024 else {
                return
            }

            if payload["action"] as? String == "validate" {
                let receipt: [String: Any]
                do {
                    let result = try validateLocalJsonDsl(source: content)
                    receipt = ["ok": true, "plan": result]
                } catch {
                    receipt = ["ok": false, "error": "Rust validation rejected"]
                }
                if let bytes = try? JSONSerialization.data(withJSONObject: receipt) {
                    let base64 = bytes.base64EncodedString()
                    let js = "window.dispatchEvent(new CustomEvent('ocaf-rust-validation',{detail:atob('" + base64 + "')}));"
                    webView?.evaluateJavaScript(js, completionHandler: nil)
                }
                return
            }
            guard payload["action"] as? String == "export" else { return }

            let suppliedName = payload["filename"] as? String ?? "project.ocaf-builder.json"
            let safeName = String(suppliedName.prefix(100).map { character in
                character.isASCII && (character.isLetter || character.isNumber || character == "." || character == "_" || character == "-")
                    ? character : "_"
            })
            let name = safeName.isEmpty ? "project.json" : safeName
            let temporary = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString + "-" + name)
            guard (try? data.write(to: temporary, options: .atomic)) != nil else { return }

            DispatchQueue.main.async { [weak self] in
                guard let root = self?.webView?.window?.rootViewController else { return }
                var presenter = root
                while let next = presenter.presentedViewController { presenter = next }
                let share = UIActivityViewController(activityItems: [temporary], applicationActivities: nil)
                if let popover = share.popoverPresentationController {
                    popover.sourceView = presenter.view
                    popover.sourceRect = CGRect(x: presenter.view.bounds.midX, y: presenter.view.bounds.midY, width: 1, height: 1)
                }
                presenter.present(share, animated: true)
            }
        }
    }
}
