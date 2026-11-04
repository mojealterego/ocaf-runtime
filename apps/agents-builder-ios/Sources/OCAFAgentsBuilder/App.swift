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
    func makeUIView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.defaultWebpagePreferences.allowsContentJavaScript = true
        let webView = WKWebView(frame: .zero, configuration: configuration)
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
