package app.rela.playback

/**
 * Losing the sound (a call, another app, headphones out) stops it on this phone only. What it takes to bring it back
 * after a loss that passes is kept here; RelaService is the platform's side.
 */
class SoundLogic(private val pause: () -> Unit, private val mute: (Boolean) -> Unit) {
    private var muted = false
    private var resume: (() -> Unit)? = null

    /** The audio focus is to come back: the request must stay. */
    val waiting get() = muted || resume != null

    /**
     * While something plays. [passing]: the focus comes back (a call). Listening together is muted then, the root plays
     * on; for good, a follower leaves. [resume] starts again what played.
     */
    fun lost(passing: Boolean, following: Boolean, resume: () -> Unit) {
        this.resume = null
        if (passing && following) {
            set(true)
        } else {
            if (passing) this.resume = resume
            pause()
        }
    }

    /** The focus came back; false: to silence, it can be given up. */
    fun gained(playing: Boolean, following: Boolean): Boolean {
        set(false)
        val back = resume.takeIf { !playing && !following } // a paused root is not ours to start
        resume = null
        back?.invoke()
        return playing || back != null
    }

    /** Playing again, by hand or by us: no resume is owed. [focus]: and the focus is ours (not while a call lasts). */
    fun playing(focus: Boolean) {
        resume = null
        if (focus) set(false)
    }

    private fun set(m: Boolean) {
        if (m != muted) {
            muted = m
            mute(m)
        }
    }
}
