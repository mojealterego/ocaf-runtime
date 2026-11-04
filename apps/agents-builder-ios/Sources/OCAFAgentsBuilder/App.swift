import SwiftUI
import WebKit

@main
struct OCAFAgentsBuilderApp: App {
    var body: some Scene {
        WindowGroup {
            EditorView()
                .ignoresSafeArea()
        }
    }
}

struct EditorView: UIViewRepresentable {
    final class Coordinator: NSObject, WKNavigationDelegate {
        func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
                     decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
            guard let url = navigationAction.request.url, url.isFileURL,
                  let root = Bundle.main.url(forResource: "Editor", withExtension: nil)?.standardizedFileURL.path else {
                decisionHandler(.cancel)
                return
            }
            let path = url.standardizedFileURL.path
            decisionHandler(path.hasPrefix(root + "/") ? .allow : .cancel)
        }
    }

    func makeCoordinator() -> Coordinator { Coordinator() }
    func makeUIView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.defaultWebpagePreferences.allowsContentJavaScript = true
        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.navigationDelegate = context.coordinator
        guard let directory = Bundle.main.url(forResource: "Editor", withExtension: nil),
              let index = Bundle.main.url(forResource: "index", withExtension: "html", subdirectory: "Editor") else {
            assertionFailure("Bundled editor resources missing")
            return webView
        }
        webView.loadFileURL(index, allowingReadAccessTo: directory)
        return webView
    }

    func updateUIView(_ uiView: WKWebView, context: Context) {}
}
