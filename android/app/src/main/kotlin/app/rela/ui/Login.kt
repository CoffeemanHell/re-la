package app.rela.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.platform.LocalAutofillManager
import androidx.compose.ui.semantics.contentType
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import app.rela.core.Rela
import uniffi.rela.PlayerState

/**
 * The desktop's login panel: with a saved login only the connection status and "Vazgeç"; else server, user and
 * password. The password goes to the core and is never kept (the core keeps md5(password + salt)); the system's
 * password manager can fill and save it.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun Login(s: PlayerState) {
    val core = Rela.core
    var url by rememberSaveable { mutableStateOf(core?.setting("url") ?: "") }
    var user by rememberSaveable { mutableStateOf(core?.setting("user") ?: "") }
    var pass by rememberSaveable { mutableStateOf("") }
    val autofill = LocalAutofillManager.current
    val go = {
        core?.login(url.trim(), user.trim(), pass)
        autofill?.commit() // the password manager offers to save it, before the field empties
        pass = ""
    }
    Column(
        Modifier.fillMaxSize().safeDrawingPadding().imePadding().verticalScroll(rememberScrollState()).padding(32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp, Alignment.CenterVertically),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text("re-la", style = MaterialTheme.typography.displaySmall, fontWeight = FontWeight.Bold)
        Text(
            "Navidrome üzerinden birlikte, kayıpsız dinle.",
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
        val field = Modifier.fillMaxWidth().widthIn(max = 480.dp)
        if (!s.signedIn) {
            OutlinedTextField(
                url, { url = it }, field, label = { Text("Sunucu") }, placeholder = { Text("https://muzik.ornek.com") },
                singleLine = true, keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = ImeAction.Next),
            )
            OutlinedTextField(
                user, { user = it }, field.semantics { contentType = ContentType.Username }, label = { Text("Kullanıcı adı") }, singleLine = true,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Next),
            )
            OutlinedTextField(
                pass, { pass = it }, field.semantics { contentType = ContentType.Password }, label = { Text("Şifre") }, singleLine = true,
                visualTransformation = PasswordVisualTransformation(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password, imeAction = ImeAction.Go),
                keyboardActions = KeyboardActions(onGo = { go() }),
            )
        }
        if (s.connecting) LoadingIndicator()
        if (s.signedIn) {
            OutlinedButton({ core?.logout() }, field) { Text("Vazgeç") } // a hanging connect can be abandoned
        } else {
            Button(go, field, enabled = url.isNotBlank() && user.isNotBlank()) { Text("Bağlan") }
        }
        if (s.status.isNotEmpty()) {
            Text(
                s.status,
                color = if (s.loginFailed) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
        }
    }
}
