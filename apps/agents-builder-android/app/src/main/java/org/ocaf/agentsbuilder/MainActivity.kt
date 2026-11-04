package org.ocaf.agentsbuilder

import android.os.Bundle
import android.webkit.WebView
import android.webkit.WebViewClient
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
                    webViewClient = WebViewClient()
                    loadUrl("file:///android_asset/index.html")
                }
            }
            AndroidView(factory = { editor }, modifier = Modifier.fillMaxSize())
            DisposableEffect(editor) { onDispose { editor.destroy() } }
        }
    }
}
