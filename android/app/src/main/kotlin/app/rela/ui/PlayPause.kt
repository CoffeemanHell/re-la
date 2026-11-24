package app.rela.ui

import androidx.compose.animation.graphics.res.animatedVectorResource
import androidx.compose.animation.graphics.res.rememberAnimatedVectorPainter
import androidx.compose.animation.graphics.vector.AnimatedImageVector
import androidx.compose.runtime.Composable
import app.rela.R

/** The pause icon; it turns into the play icon while [paused]. */
@Composable
fun playPausePainter(paused: Boolean) =
    rememberAnimatedVectorPainter(AnimatedImageVector.animatedVectorResource(R.drawable.pause_to_play), paused)
