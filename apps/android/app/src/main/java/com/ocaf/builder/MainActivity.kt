package com.ocaf.builder

import android.annotation.SuppressLint
import android.net.Uri
import android.os.Bundle
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebSettings
import android.webkit.WebView
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.webkit.WebViewAssetLoader
import androidx.webkit.WebViewClientCompat
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import org.json.JSONObject
import uniffi.ocaf_builder_mobile_ffi.validateLocalJsonDsl
import java.io.ByteArrayInputStream

/** Secure, native Compose shell for the shared Code / NoCode graph editor. */
class MainActivity : ComponentActivity() {
    private val localHost = "appassets.androidplatform.net"
    private val localUrl = "https://appassets.androidplatform.net/assets/www/index.html"
    private var pickerCallback: ValueCallback<Array<Uri>>? = null
    private var exportPayload: ByteArray? = null

    private val importLauncher = registerForActivityResult(ActivityResultContracts.GetContent()) { uri ->
        pickerCallback?.onReceiveValue(if (uri == null) null else arrayOf(uri))
        pickerCallback = null
    }
    private val exportLauncher = registerForActivityResult(
        ActivityResultContracts.CreateDocument("application/json")
    ) { uri ->
        val payload = exportPayload
        exportPayload = null
        if (uri != null && payload != null) {
            try {
                contentResolver.openOutputStream(uri, "w")?.use { it.write(payload) }
            } catch (_: Exception) {
                // SAF failure does not claim a successful export.
            }
        }
    }

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val assetLoader = WebViewAssetLoader.Builder()
            .addPathHandler("/assets/", WebViewAssetLoader.AssetsPathHandler(this))
            .build()
        setContent {
            val web = remember {
                WebView(this).apply {
                    settings.javaScriptEnabled = true
                    settings.domStorageEnabled = true
                    settings.allowFileAccess = false
                    settings.allowContentAccess = false
                    settings.allowFileAccessFromFileURLs = false
                    settings.allowUniversalAccessFromFileURLs = false
                    settings.mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
                    webChromeClient = object : WebChromeClient() {
                        override fun onShowFileChooser(
                            webView: WebView?,
                            filePathCallback: ValueCallback<Array<Uri>>,
                            fileChooserParams: FileChooserParams?
                        ): Boolean {
                            pickerCallback?.onReceiveValue(null)
                            pickerCallback = filePathCallback
                            importLauncher.launch("application/json")
                            return true
                        }
                    }
                    webViewClient = object : WebViewClientCompat() {
                        override fun shouldInterceptRequest(
                            view: WebView, request: WebResourceRequest
                        ): WebResourceResponse? {
                            val uri = request.url
                            if (uri.scheme == "https" && uri.host == localHost &&
                                uri.path?.startsWith("/assets/www/") == true) {
                                return assetLoader.shouldInterceptRequest(uri)
                            }
                            return WebResourceResponse(
                                "text/plain", "utf-8", 403, "Forbidden", emptyMap(),
                                ByteArrayInputStream(ByteArray(0))
                            )
                        }
                        override fun shouldOverrideUrlLoading(
                            view: WebView, request: WebResourceRequest
                        ): Boolean {
                            val uri = request.url
                            return !(uri.scheme == "https" && uri.host == localHost &&
                                uri.path?.startsWith("/assets/www/") == true)
                        }
                    }

                    if (WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) {
                        WebViewCompat.addWebMessageListener(
                            this, "ocafHost", setOf("https://appassets.androidplatform.net")
                        ) { _, message, sourceOrigin, isMainFrame, _ ->
                            if (!isMainFrame || sourceOrigin.host != localHost ||
                                sourceOrigin.scheme != "https") return@addWebMessageListener
                            try {
                                val raw = message.data ?: return@addWebMessageListener
                                if (raw.length > 150000) return@addWebMessageListener
                                val obj = JSONObject(raw)
                                val dataString = obj.getString("content")
                                val data = dataString.toByteArray(Charsets.UTF_8)
                                if (data.size > 128 * 1024) return@addWebMessageListener
                                when (obj.optString("action")) {
                                    "export" -> {
                                        // Native export must pass the same Rust DAG guard as validation.
                                        if (runCatching { validateLocalJsonDsl(dataString) }.isFailure) {
                                            return@addWebMessageListener
                                        }
                                        val name = obj.optString("filename", "project.ocaf-builder.json")
                                            .replace(Regex("[^A-Za-z0-9_.-]"), "_").take(100)
                                        runOnUiThread {
                                            exportPayload = data
                                            exportLauncher.launch(name.ifEmpty { "project.json" })
                                        }
                                    }
                                    "validate" -> {
                                        val result = runCatching { validateLocalJsonDsl(dataString) }
                                        val reply = JSONObject()
                                            .put("ok", result.isSuccess)
                                            .put("plan", result.getOrNull())
                                            .put("error", if (result.isFailure) "Rust validation rejected" else JSONObject.NULL)
                                            .toString()
                                        this.post {
                                            evaluateJavascript(
                                                "window.dispatchEvent(new CustomEvent('ocaf-rust-validation',{detail:" +
                                                    JSONObject.quote(reply) + "}));", null
                                            )
                                        }
                                    }
                                }
                            } catch (_: Exception) {
                                // Invalid messages have no effect.
                            }
                        }
                    }
                    loadUrl(localUrl)
                }
            }
            DisposableEffect(web) {
                onDispose {
                    pickerCallback?.onReceiveValue(null)
                    pickerCallback = null
                    web.stopLoading()
                    web.destroy()
                }
            }
            Column(Modifier.fillMaxSize().background(Color(0xFF07080A))) {
                Text(
                    "OCAF Agents Builder · Code / NoCode · lokalnie",
                    modifier = Modifier.fillMaxWidth().padding(12.dp),
                    color = Color(0xFFD4AF37),
                    style = MaterialTheme.typography.titleMedium
                )
                AndroidView(factory = { web }, modifier = Modifier.fillMaxSize())
            }
        }
    }
}
