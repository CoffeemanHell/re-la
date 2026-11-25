//! The desktop deck: one libmpv per deck. Commands go to mpv from a thread of
//! their own, so nothing waits on mpv; an event thread per deck turns mpv's events into DeckEvents. libmpv2's safe API
//! only: the C side (strings mpv owns, event payloads) stays inside that crate.
#![forbid(unsafe_code)]
use libmpv2::events::{Event, PropertyData};
use libmpv2::{Error, Mpv, mpv_end_file_reason as why};
use rela_core::player::{Deck, DeckEvent, DeckSink, EndReason};
use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};
use std::thread::JoinHandle;

const DOUBLES: [&str; 3] = ["time-pos", "duration", "audio-bitrate"];
const STRINGS: [&str; 7] = [
    "audio-params/format",
    "audio-params/samplerate",
    "audio-out-params/format",
    "audio-out-params/samplerate",
    "current-ao",
    "audio-codec-name",
    "audio-params/channel-count",
];

/// What the command thread does, in the order the deck asked.
enum Cmd {
    Run(Vec<String>),
    Double(&'static str, f64),
    Flag(&'static str, bool),
}

struct MpvDeck {
    mpv: Arc<Mpv>,
    cmds: Option<Sender<Cmd>>,
    threads: Vec<JoinHandle<()>>,
}

impl MpvDeck {
    fn send(&self, c: Cmd) {
        if let Some(tx) = &self.cmds {
            let _ = tx.send(c);
        }
    }

    fn command(&self, args: &[&str]) {
        self.send(Cmd::Run(args.iter().map(|a| a.replace('\0', "")).collect()));
    }

    fn get_string(&self, name: &str) -> String {
        self.mpv.get_property::<String>(name).unwrap_or_else(|_| "-".into())
    }
}

/// mpv's answer to a command doesn't matter here (it didn't with the async calls either): a failed seek or load shows
/// up as an event.
fn run(mpv: &Mpv, c: Cmd) {
    let _ = match c {
        Cmd::Run(args) => match args.split_first() {
            Some((name, rest)) => mpv.command(name, &rest.iter().map(String::as_str).collect::<Vec<_>>()),
            None => Ok(()),
        },
        Cmd::Double(name, v) => mpv.set_property(name, v),
        Cmd::Flag(name, on) => mpv.set_property(name, on),
    };
}

fn first_output(all: Vec<(String, String)>) -> Vec<(String, String)> {
    let Some(ao) = all.iter().find_map(|(name, _)| Some(name.split_once('/')?.0.to_string())) else { return vec![] };
    all.into_iter().filter(|(name, _)| name.split_once('/').is_some_and(|(a, _)| a == ao)).collect()
}

fn num(v: f64) -> String {
    format!("{v:.3}")
}

impl Deck for MpvDeck {
    fn load(&mut self, url: &str, paused: bool) {
        self.send(Cmd::Flag("pause", paused));
        self.command(&["loadfile", url, "replace"]);
    }
    fn append(&mut self, url: &str) {
        self.command(&["loadfile", url, "append"]);
    }
    fn clear_appended(&mut self) {
        self.command(&["playlist-clear"]);
    }
    fn drop_first(&mut self) {
        self.command(&["playlist-remove", "0"]);
    }
    fn set_paused(&mut self, paused: bool) {
        self.send(Cmd::Flag("pause", paused));
    }
    fn seek(&mut self, s: f64) {
        self.command(&["seek", &num(s), "absolute+exact"]);
    }
    fn set_volume(&mut self, v: f64) {
        self.send(Cmd::Double("volume", v));
    }
    fn set_speed(&mut self, s: f64) {
        self.send(Cmd::Double("speed", s));
    }
    fn stop(&mut self) {
        self.command(&["stop"]);
    }
    fn set_exclusive(&mut self, on: bool) {
        self.send(Cmd::Flag("audio-exclusive", on));
        self.command(&["ao-reload"]);
    }
    fn set_audio_device(&mut self, name: &str) {
        self.command(&["set", "audio-device", name]);
    }
    /// mpv lists every audio output's devices, the best output first (PipeWire's, then the same ones again as PulseAudio
    /// and ALSA): the first one's are enough.
    fn audio_devices(&self) -> Vec<(String, String)> {
        let n = self.mpv.get_property::<i64>("audio-device-list/count").unwrap_or(0);
        let all: Vec<(String, String)> = (0..n)
            .filter_map(|i| {
                let get = |k: &str| self.mpv.get_property::<String>(&format!("audio-device-list/{i}/{k}")).ok();
                Some((get("name")?, get("description")?))
            })
            .collect();
        first_output(all)
    }
    /// Tells a silent mpv from a muted or missing PipeWire stream.
    fn audio_log(&self) -> String {
        format!(
            "ao={} volume={} ao-volume={} ao-mute={} device={}",
            self.get_string("current-ao"),
            self.get_string("volume"),
            self.get_string("ao-volume"),
            self.get_string("ao-mute"),
            self.get_string("audio-device")
        )
    }
}

impl Drop for MpvDeck {
    /// The command thread ends with its channel; `quit` makes mpv send Shutdown, which ends the event thread.
    fn drop(&mut self) {
        self.cmds = None;
        if let Some(t) = self.threads.pop() {
            let _ = t.join();
        }
        let _ = self.mpv.command("quit", &[]);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

fn end_reason(r: libmpv2::EndFileReason) -> EndReason {
    match r {
        why::Eof => EndReason::Eof,
        why::Stop => EndReason::Stop,
        why::Error => EndReason::Error,
        _ => EndReason::Other,
    }
}

/// mpv's events, translated, until mpv shuts down.
fn pump(mpv: Arc<Mpv>, index: usize, sink: DeckSink) {
    loop {
        let ev = match mpv.wait_event(-1.0) {
            None => None,
            Some(Ok(Event::Shutdown)) => return,
            Some(Ok(Event::StartFile)) => Some(DeckEvent::StartFile),
            Some(Ok(Event::FileLoaded)) => Some(DeckEvent::FileLoaded),
            Some(Ok(Event::EndFile(r))) => Some(DeckEvent::EndFile { reason: end_reason(r), error: String::new() }),
            // libmpv2 hands an end with an error over as the error itself; the only other mpv error codes come with
            // command and property replies, which the synchronous calls take, so a Raw one here is a failed file.
            Some(Err(Error::Raw(code))) => {
                Some(DeckEvent::EndFile { reason: EndReason::Error, error: libmpv2_sys::mpv_error_str(code).to_string() })
            }
            Some(Ok(Event::PropertyChange { name, change, .. })) => match change {
                PropertyData::Double(v) => match name {
                    "time-pos" => Some(DeckEvent::Position(Some(v))),
                    "duration" => Some(DeckEvent::Duration(v)),
                    "audio-bitrate" => Some(DeckEvent::Bitrate(Some(v))),
                    _ => None,
                },
                PropertyData::Str(s) => Some(DeckEvent::Prop(name.to_string(), Some(s.to_string()))),
                _ => match name {
                    "time-pos" => Some(DeckEvent::Position(None)),
                    "audio-bitrate" => Some(DeckEvent::Bitrate(None)),
                    "duration" => None,
                    _ => Some(DeckEvent::Prop(name.to_string(), None)),
                },
            },
            _ => None, // other events, a name that isn't UTF-8
        };
        if let Some(ev) = ev {
            sink(index, ev);
        }
    }
}

/// A deck reporting to sink as `index`. ao: the audio outputs to try ("pipewire,pulse,alsa," in the app, "null" in
/// tests). Panics when libmpv can't start: the app can't play anything then.
pub fn mpv_deck(index: usize, sink: DeckSink, ao: &str) -> Box<dyn Deck> {
    let mpv = Mpv::with_initializer(|i| {
        i.set_option("vid", "no")?;
        i.set_option("idle", "yes")?;
        i.set_option("ytdl", "no")?;
        i.set_option("audio-client-name", "re-la")?;
        i.set_option("ao", ao)?;
        i.set_option("audio-exclusive", "no")?;
        // gapless-audio stays at mpv's default "weak": gapless only when formats match, never resamples.
        i.set_option("prefetch-playlist", "yes")?;
        i.set_option("cache", "yes")?;
        i.set_option("demuxer-max-bytes", "256MiB")?; // a typical hi-res FLAC fits in RAM whole; a longer one streams the rest
        // A FLAC seek (resume, timeline click) is ~10 range reads; on one kept-alive connection instead of a new TLS one
        // each, a mid-song start went 4.6 s -> 2.2 s (measured). Live streams ignore the range.
        i.set_option("stream-lavf-o", "multiple_requests=1,request_size=2097152")?;
        i.set_option("volume-max", "100")?;
        // mpv's built-in Lua scripts (on-screen controls, console, menus) do nothing for a deck without a window, and
        // each deck leaked a little of their memory when it went away (heaptrack, leak-check.sh). Not fatal: a libmpv
        // without one of these options still plays.
        for o in ["osc", "load-scripts", "load-stats-overlay", "load-console", "load-osd-console", "load-auto-profiles",
            "load-select", "load-commands", "load-positioning", "load-context-menu"] {
            let _ = i.set_option(o, "no");
        }
        Ok(())
    })
    .expect("libmpv could not start");
    for p in DOUBLES {
        mpv.observe_property(p, libmpv2::Format::Double, 0).expect("observe");
    }
    for p in STRINGS {
        mpv.observe_property(p, libmpv2::Format::String, 0).expect("observe");
    }
    let mpv = Arc::new(mpv);
    let m = mpv.clone();
    let events = std::thread::Builder::new()
        .name(format!("rela-mpv-{index}"))
        .spawn(move || pump(m, index, sink))
        .expect("mpv event thread");
    let (tx, rx) = channel();
    let m = mpv.clone();
    let commands = std::thread::Builder::new()
        .name(format!("rela-mpv-cmd-{index}"))
        .spawn(move || rx.into_iter().for_each(|c| run(&m, c)))
        .expect("mpv command thread");
    // Drop joins the last one first: the command thread, before `quit`
    Box::new(MpvDeck { mpv, cmds: Some(tx), threads: vec![events, commands] })
}

#[cfg(test)]
mod tests {
    #[test]
    fn one_outputs_devices() {
        let d = |n: &str| (n.to_string(), format!("D {n}"));
        let all = vec![d("auto"), d("pipewire"), d("pipewire/a"), d("pipewire/b"), d("pulse/a"), d("alsa/hw")];
        assert_eq!(super::first_output(all), [d("pipewire/a"), d("pipewire/b")]);
        assert_eq!(super::first_output(vec![d("auto"), d("null")]), []);
    }
}
