package org.ocaf.agentsbuilder

import android.os.Bundle
import android.webkit.WebView
import android.webkit.WebResourceResponse
import androidx.webkit.WebViewAssetLoader
import androidx.webkit.WebViewClientCompat
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.AndroidView

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            val editor = remember {
                WebView(this).apply {
                    settings.javaScriptEnabled = true
                    settings.domStorageEnabled = true
                    settings.setAllowFileAccessFromFileURLs(false)
                    settings.setAllowUniversalAccessFromFileURLs(false)
                    settings.allowContentAccess = false
                    settings.allowFileAccess = false
                    val loader = WebViewAssetLoader.Builder()
                        .addPathHandler("/assets/", WebViewAssetLoader.AssetsPathHandler(this@MainActivity))
                        .build()
                    webViewClient = object : WebViewClientCompat() {
                        override fun shouldOverrideUrlLoading(view: WebView, request: android.webkit.WebResourceRequest): Boolean =
                            request.url.scheme != "https" || request.url.host != "appassets.androidplatform.net" ||
                                request.url.path?.startsWith("/assets/") != true
                        override fun shouldInterceptRequest(view: WebView, request: android.webkit.WebResourceRequest): WebResourceResponse? {
                            val resource = loader.shouldInterceptRequest(request.url) ?: return null
                            // Android's MIME map may not recognize .mjs; ES modules require JavaScript MIME.
                            return if (request.url.path?.endsWith(".mjs") == true) {
                                WebResourceResponse("text/javascript", "UTF-8", resource.data)
                            } else resource
                        }
                    }
                    loadUrl("https://appassets.androidplatform.net/assets/index.html")
                }
            }
            AndroidView(factory = { editor }, modifier = Modifier.fillMaxSize())
            DisposableEffect(editor) { onDispose { editor.destroy() } }
        }
    }
}
